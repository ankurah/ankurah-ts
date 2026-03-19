//! Phase 2: Body translation — syn::Expr/Stmt → TS expression/statement text
//!
//! Translates Rust function bodies to TypeScript. Each syn expression variant
//! maps to a TS expression string. The output is deterministic and structural,
//! prioritizing 1:1 correspondence with the Rust source over elegance.

use syn;
use crate::name_map;
use crate::macros;
use crate::match_expr;
use crate::control_flow;
use crate::ownership;

// ── Public entry points ─────────────────────────────────────────────────

/// Translate a block of statements to TS (default Self type)
pub fn translate_block(block: &syn::Block) -> String {
    BodyTranslator::new("Self").translate_block(block)
}

/// Translate a block with a known self type name for Self resolution
pub fn translate_block_with_self(block: &syn::Block, self_type: &str) -> String {
    BodyTranslator::new(self_type).translate_block(block)
}

/// Translate a single expression (used by match_expr, control_flow, macros modules)
pub fn translate_expr(expr: &syn::Expr) -> String {
    BodyTranslator::new("Self").expr(expr)
}

/// Translate a pattern (used by match_expr, control_flow modules)
pub fn translate_pat(pat: &syn::Pat) -> String {
    BodyTranslator::pat_static(pat)
}

/// Indent each line by 2 spaces
pub fn indent(s: &str) -> String {
    s.lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {}", line) })
        .collect::<Vec<_>>()
        .join("\n")
        + if s.ends_with('\n') { "\n" } else { "" }
}

// ── Translator struct ───────────────────────────────────────────────────

pub struct BodyTranslator<'a> {
    pub self_type: &'a str,
}

impl<'a> BodyTranslator<'a> {
    pub fn new(self_type: &'a str) -> Self {
        Self { self_type }
    }

    // ── Block translation with ownership tracking ───────────────────

    pub fn translate_block(&self, block: &syn::Block) -> String {
        let mut out = String::new();
        let stmts = &block.stmts;

        // Collect local bindings for drop insertion
        let mut locals: Vec<(String, String)> = Vec::new();
        for stmt in stmts {
            if let syn::Stmt::Local(local) = stmt {
                ownership::collect_local_bindings(&local.pat, &mut locals);
            }
        }

        // Determine which locals are consumed (returned, passed as args, stored)
        let mut consumed_vars = std::collections::HashSet::new();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    ownership::collect_direct_vars(expr, &mut consumed_vars);
                }
            }
            ownership::collect_consumed_vars_in_stmt(stmt, &mut consumed_vars);
        }

        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last {
                if let syn::Stmt::Expr(expr, None) = stmt {
                    let drops = ownership::generate_drops(&locals, &consumed_vars);
                    if !drops.is_empty() {
                        out.push_str(&drops);
                    }
                    out.push_str(&control_flow::translate_expr_in_return_position_with(expr, self));
                    out.push('\n');
                } else {
                    out.push_str(&self.stmt(stmt));
                    let drops = ownership::generate_drops(&locals, &consumed_vars);
                    if !drops.is_empty() {
                        out.push_str(&drops);
                    }
                }
            } else {
                out.push_str(&self.stmt(stmt));
            }
        }
        out
    }

    // ── Statement translation ───────────────────────────────────────

    fn stmt(&self, stmt: &syn::Stmt) -> String {
        match stmt {
            syn::Stmt::Local(local) => self.local(local),
            syn::Stmt::Expr(expr, semi) => {
                let ts = self.expr(expr);
                if semi.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
            syn::Stmt::Item(_) => String::new(),
            syn::Stmt::Macro(macro_stmt) => {
                let ts = macros::translate_macro(&macro_stmt.mac);
                if macro_stmt.semi_token.is_some() {
                    format!("{};\n", ts)
                } else {
                    format!("{}\n", ts)
                }
            }
        }
    }

    fn local(&self, local: &syn::Local) -> String {
        let pat = Self::pat_static(&local.pat);

        if let Some(init) = &local.init {
            let expr = self.expr(&init.expr);

            if let Some((_tok, _diverge)) = &init.diverge {
                return format!("/* let-else */ const {} = {};\n", pat, expr);
            }

            let keyword = if is_mut_binding(&local.pat) { "let" } else { "const" };
            format!("{} {} = {};\n", keyword, pat, expr)
        } else {
            format!("let {};\n", pat)
        }
    }

    // ── Pattern translation (static — no self_type needed) ──────────

    pub fn pat_static(pat: &syn::Pat) -> String {
        match pat {
            syn::Pat::Ident(ident) => name_map::to_camel_case(&ident.ident.to_string()),
            syn::Pat::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::TupleStruct(ts) => {
                let parts: Vec<String> = ts.elems.iter().map(Self::pat_static).collect();
                parts.join(", ")
            }
            syn::Pat::Struct(s) => {
                let fields: Vec<String> = s.fields.iter().map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let pat = Self::pat_static(&f.pat);
                    if member == pat { member } else { format!("{}: {}", member, pat) }
                }).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            syn::Pat::Wild(_) => "_".to_string(),
            syn::Pat::Lit(_) => "/* pat literal */".to_string(),
            syn::Pat::Path(path) => Self::path_static(&path.path),
            syn::Pat::Reference(r) => Self::pat_static(&r.pat),
            syn::Pat::Type(t) => Self::pat_static(&t.pat),
            syn::Pat::Or(or_pat) => {
                let parts: Vec<String> = or_pat.cases.iter().map(Self::pat_static).collect();
                parts.join(" | ")
            }
            syn::Pat::Slice(slice) => {
                let parts: Vec<String> = slice.elems.iter().map(Self::pat_static).collect();
                format!("[{}]", parts.join(", "))
            }
            syn::Pat::Rest(_) => "...".to_string(),
            _ => "/* unknown pat */".to_string(),
        }
    }

    // ── Expression translation ──────────────────────────────────────

    pub fn expr(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Lit(lit) => translate_lit(&lit.lit),
            syn::Expr::Path(path) => Self::path_static(&path.path),

            syn::Expr::Field(field) => {
                let base = self.expr(&field.base);
                let member = match &field.member {
                    syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                    syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                };
                if base == "self" { format!("this.{}", member) } else { format!("{}.{}", base, member) }
            }

            syn::Expr::MethodCall(call) => {
                let receiver = self.expr(&call.receiver);
                let method = name_map::map_fn_name(&call.method.to_string());
                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();
                self.translate_method_call(&receiver, &method, &args)
            }

            syn::Expr::Call(call) => {
                let func = self.expr(&call.func);
                let args: Vec<String> = call.args.iter().map(|a| self.expr(a)).collect();
                self.translate_call(&func, &args)
            }

            syn::Expr::Binary(bin) => {
                format!("{} {} {}", self.expr(&bin.left), translate_binop(&bin.op), self.expr(&bin.right))
            }

            syn::Expr::Unary(unary) => {
                let e = self.expr(&unary.expr);
                match &unary.op {
                    syn::UnOp::Not(_) => format!("!{}", e),
                    syn::UnOp::Neg(_) => format!("-{}", e),
                    syn::UnOp::Deref(_) => e,
                    _ => format!("/* unknown unary op */ {}", e),
                }
            }

            syn::Expr::If(if_expr) => control_flow::translate_if(if_expr),

            syn::Expr::Block(block) => {
                if block.block.stmts.len() == 1 {
                    if let syn::Stmt::Expr(expr, None) = &block.block.stmts[0] {
                        return self.expr(expr);
                    }
                }
                let body = self.translate_block(&block.block);
                format!("{{\n{}}}", indent(&body))
            }

            syn::Expr::Return(ret) => {
                if let Some(expr) = &ret.expr {
                    format!("return {}", self.expr(expr))
                } else {
                    "return".to_string()
                }
            }

            syn::Expr::Match(me) => match_expr::translate_match(me),

            syn::Expr::Closure(closure) => {
                let params: Vec<String> = closure.inputs.iter().map(Self::pat_static).collect();
                let body = self.expr(&closure.body);
                format!("({}) => {}", params.join(", "), body)
            }

            syn::Expr::ForLoop(for_loop) => {
                let pat = Self::pat_static(&for_loop.pat);
                let iter = self.expr(&for_loop.expr);
                let body = self.translate_block(&for_loop.body);
                format!("for (const {} of {}) {{\n{}}}", pat, iter, indent(&body))
            }

            syn::Expr::While(while_loop) => {
                let cond = self.expr(&while_loop.cond);
                let body = self.translate_block(&while_loop.body);
                format!("while ({}) {{\n{}}}", cond, indent(&body))
            }

            syn::Expr::Loop(loop_expr) => {
                let body = self.translate_block(&loop_expr.body);
                format!("while (true) {{\n{}}}", indent(&body))
            }

            syn::Expr::Break(brk) => {
                if let Some(expr) = &brk.expr {
                    format!("break /* {} */", self.expr(expr))
                } else { "break".to_string() }
            }

            syn::Expr::Continue(_) => "continue".to_string(),

            syn::Expr::Assign(assign) => {
                format!("{} = {}", self.expr(&assign.left), self.expr(&assign.right))
            }

            syn::Expr::Index(idx) => {
                format!("{}[{}]", self.expr(&idx.expr), self.expr(&idx.index))
            }

            syn::Expr::Reference(reference) => self.expr(&reference.expr),

            syn::Expr::Paren(paren) => format!("({})", self.expr(&paren.expr)),

            syn::Expr::Tuple(tuple) => {
                let parts: Vec<String> = tuple.elems.iter().map(|e| self.expr(e)).collect();
                format!("[{}]", parts.join(", "))
            }

            syn::Expr::Array(arr) => {
                let items: Vec<String> = arr.elems.iter().map(|e| self.expr(e)).collect();
                format!("[{}]", items.join(", "))
            }

            syn::Expr::Struct(s) => {
                let name = Self::path_static(&s.path);
                let fields: Vec<String> = s.fields.iter().map(|f| {
                    let member = match &f.member {
                        syn::Member::Named(ident) => name_map::to_camel_case(&ident.to_string()),
                        syn::Member::Unnamed(idx) => format!("_{}", idx.index),
                    };
                    let value = self.expr(&f.expr);
                    if member == value { member } else { format!("{}: {}", member, value) }
                }).collect();
                format!("new {}({{ {} }})", name, fields.join(", "))
            }

            syn::Expr::Try(try_expr) => self.expr(&try_expr.expr),
            syn::Expr::Await(await_expr) => format!("await {}", self.expr(&await_expr.base)),

            syn::Expr::Range(range) => {
                let from = range.start.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                let to = range.end.as_ref().map(|e| self.expr(e)).unwrap_or_default();
                format!("/* range {}..{} */", from, to)
            }

            syn::Expr::Cast(cast) => {
                format!("{} as {}", self.expr(&cast.expr), name_map::map_type(&cast.ty))
            }

            syn::Expr::Macro(mac) => macros::translate_macro(&mac.mac),

            syn::Expr::Unsafe(unsafe_block) => {
                let body = self.translate_block(&unsafe_block.block).trim().to_string();
                format!("/* unsafe — consider provided impl */ {}", body)
            }

            syn::Expr::Async(async_block) => {
                let body = self.translate_block(&async_block.block);
                format!("(async () => {{\n{}}})()", indent(&body))
            }

            syn::Expr::Let(let_expr) => {
                let pat = Self::pat_static(&let_expr.pat);
                let expr = self.expr(&let_expr.expr);
                format!("/* let {} = {} */", pat, expr)
            }

            syn::Expr::Repeat(repeat) => {
                format!("Array({}).fill({})", self.expr(&repeat.len), self.expr(&repeat.expr))
            }

            _ => "/* TODO: unhandled expr */".to_string(),
        }
    }

    // ── Method call translation ─────────────────────────────────────

    fn translate_method_call(&self, receiver: &str, method: &str, args: &[String]) -> String {
        match method {
            // Result/Option
            "unwrap" | "expect" => receiver.to_string(),
            "unwrapOr" if args.len() == 1 => format!("{} ?? {}", receiver, args[0]),
            "unwrapOrElse" if args.len() == 1 => format!("{} ?? ({})()", receiver, args[0]),
            "unwrapOrDefault" => format!("{} ?? default()", receiver),
            "isOk" | "isSome" => format!("{} != null", receiver),
            "isErr" | "isNone" => format!("{} == null", receiver),
            "ok" | "mapErr" => receiver.to_string(),

            // Collections
            "len" if args.is_empty() => format!("{}.length", receiver),
            "isEmpty" if args.is_empty() => format!("{}.length === 0", receiver),
            "push" => format!("{}.push({})", receiver, args.join(", ")),
            "pop" => format!("{}.pop()", receiver),
            "last" => format!("{}.at(-1)", receiver),
            "first" => format!("{}[0]", receiver),
            "get" if args.len() == 1 => format!("{}.get({})", receiver, args[0]),
            "contains" if args.len() == 1 => format!("{}.includes({})", receiver, args[0]),
            "sort" if args.is_empty() => format!("{}.sort()", receiver),
            "sortBy" if args.len() == 1 => format!("{}.sort({})", receiver, args[0]),
            "reverse" => format!("{}.reverse()", receiver),
            "join" if args.len() == 1 => format!("{}.join({})", receiver, args[0]),

            // Iterators
            "map" if args.len() == 1 => format!("{}.map({})", receiver, args[0]),
            "filter" if args.len() == 1 => format!("{}.filter({})", receiver, args[0]),
            "any" if args.len() == 1 => format!("{}.some({})", receiver, args[0]),
            "all" if args.len() == 1 => format!("{}.every({})", receiver, args[0]),
            "find" if args.len() == 1 => format!("{}.find({})", receiver, args[0]),
            "position" if args.len() == 1 => format!("{}.findIndex({})", receiver, args[0]),
            "enumerate" => format!("{}.entries()", receiver),
            "collect" | "iter" | "intoIter" => receiver.to_string(),
            "cloned" => format!("[...{}]", receiver),

            // Conversion
            "clone" => format!("{}.clone()", receiver),
            "toOwned" => format!("{}.clone()", receiver),
            "toString" | "toStr" => format!("{}.toString()", receiver),
            "into" | "from" | "asRef" | "asMut" => receiver.to_string(),

            // Vec
            "insert" if args.len() == 2 => format!("{}.splice({}, 0, {})", receiver, args[0], args[1]),
            "remove" if args.len() == 1 => format!("{}.splice({}, 1)[0]", receiver, args[0]),
            "extend" if args.len() == 1 => format!("{}.push(...{})", receiver, args[0]),
            "clear" => format!("{}.length = 0", receiver),
            "truncate" if args.len() == 1 => format!("{}.length = {}", receiver, args[0]),
            "drain" => format!("{}.splice(0)", receiver),

            // Map
            "insertMap" if args.len() == 2 => format!("{}.set({}, {})", receiver, args[0], args[1]),
            "entry" => format!("/* {}.entry({}) */", receiver, args.join(", ")),

            // String
            "startsWith" if args.len() == 1 => format!("{}.startsWith({})", receiver, args[0]),
            "endsWith" if args.len() == 1 => format!("{}.endsWith({})", receiver, args[0]),
            "trim" => format!("{}.trim()", receiver),
            "splitStr" if args.len() == 1 => format!("{}.split({})", receiver, args[0]),
            "replacen" | "replace" => format!("{}.replace({})", receiver, args.join(", ")),

            // Comparison
            "cmp" | "partialCmp" if args.len() == 1 => format!("{}.compareTo({})", receiver, args[0]),
            "eq" if args.len() == 1 => format!("{}.equals({})", receiver, args[0]),
            "binarySearch" if args.len() == 1 => format!("{}.binarySearch({})", receiver, args[0]),

            _ => format!("{}.{}({})", receiver, method, args.join(", ")),
        }
    }

    // ── Function call translation ───────────────────────────────────

    fn translate_call(&self, func: &str, args: &[String]) -> String {
        match func {
            "Self" => format!("new {}({})", self.self_type, args.join(", ")),
            "Ok" | "Some" => {
                if args.len() == 1 { args[0].clone() } else { args.join(", ") }
            }
            "Err" => format!("throw new Error({})", args.join(", ")),
            "None" => "null".to_string(),
            "Vec.new" | "Vec::new" => "[]".to_string(),
            "HashMap.new" | "HashMap::new" | "BTreeMap.new" | "BTreeMap::new" => "new Map()".to_string(),
            "HashSet.new" | "HashSet::new" | "BTreeSet.new" | "BTreeSet::new" => "new Set()".to_string(),
            "String.new" | "String::new" => "''".to_string(),
            _ if func.ends_with(".new") || func.ends_with("::new") => {
                let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
                format!("new {}({})", type_name, args.join(", "))
            }
            _ if func.starts_with("Self.") || func.starts_with("Self::") => {
                let method = func.split('.').last().unwrap_or(func);
                let method = func.split("::").last().unwrap_or(method);
                format!("{}.{}({})", self.self_type, method, args.join(", "))
            }
            _ => {
                if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !func.contains('.')
                    && !matches!(func, "Ok" | "Some" | "Err" | "None" | "Self")
                {
                    format!("new {}({})", func, args.join(", "))
                } else {
                    format!("{}({})", func, args.join(", "))
                }
            }
        }
    }

    // ── Path translation (static) ───────────────────────────────────

    fn path_static(path: &syn::Path) -> String {
        let segments: Vec<String> = path.segments.iter().map(|seg| {
            let name = seg.ident.to_string();
            match name.as_str() {
                "self" => "this".to_string(),
                "Self" => "Self".to_string(),
                "None" => "null".to_string(),
                "true" | "false" => name,
                "Ok" | "Some" | "Err" => name,
                "std" | "core" | "alloc" | "crate" | "super" => name,
                _ => {
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        name
                    } else {
                        name_map::to_camel_case(&name)
                    }
                }
            }
        }).collect();

        let joined = segments.join(".");
        match joined.as_str() {
            s if s.contains("std.") || s.contains("core.") => {
                segments.last().cloned().unwrap_or(joined)
            }
            s if s.starts_with("crate.") => {
                segments.last().cloned().unwrap_or(joined)
            }
            _ => joined,
        }
    }
}

// ── Standalone helpers ──────────────────────────────────────────────────

fn is_mut_binding(pat: &syn::Pat) -> bool {
    if let syn::Pat::Ident(ident) = pat {
        ident.mutability.is_some()
    } else {
        false
    }
}

fn translate_lit(lit: &syn::Lit) -> String {
    match lit {
        syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "\\'")),
        syn::Lit::Int(i) => i.base10_digits().to_string(),
        syn::Lit::Float(f) => f.base10_digits().to_string(),
        syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
        syn::Lit::Char(c) => format!("'{}'", c.value()),
        syn::Lit::Byte(b) => format!("{}", b.value()),
        _ => "/* unknown literal */".to_string(),
    }
}

fn translate_binop(op: &syn::BinOp) -> &'static str {
    match op {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::BitXor(_) => "^",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::BitOr(_) => "|",
        syn::BinOp::Shl(_) => "<<",
        syn::BinOp::Shr(_) => ">>",
        syn::BinOp::Eq(_) => "===",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Ne(_) => "!==",
        syn::BinOp::Ge(_) => ">=",
        syn::BinOp::Gt(_) => ">",
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => "/* unknown op */",
    }
}
