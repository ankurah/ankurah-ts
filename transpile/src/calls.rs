//! A call written as a path: `Foo::new(..)`, `Ok(..)`, `drop(x)`, a free
//! function, an enum variant.
//!
//! For: the callee is a path, and what the path means decides everything about
//! the emitted call — a constructor, a variant, a runtime function, a method on
//! a class. The engine answers most of it; what is left here is the table that
//! turns those answers into text, and the guesses that remain when the engine
//! could not answer, each of them reported.

use crate::body::BodyTranslator;
use crate::native_types;

impl BodyTranslator<'_> {
    /// `new HashMap()` and `new HashSet()` with the arguments the position
    /// wants of them, where the position said.
    fn with_container_arguments(&self, written: String, span: proc_macro2::Span) -> String {
        let bare = match written.as_str() {
            "new HashMap()" | "new HashSet()" => &written[4..written.len() - 2],
            _ => return written,
        };
        let Some(want) = self.expectation_at(span) else { return written };
        let spelled = match &self.types {
            Some(tc) => crate::name_map::map_ty(tc.borrow().registry, &want),
            None => return written,
        };
        // Only where the expectation is that very container: an expectation of
        // a wrapper around one says nothing about the arguments HERE.
        match spelled.strip_prefix(bare).and_then(|rest| rest.strip_prefix('<')) {
            Some(_) => format!("new {}()", spelled),
            None => written,
        }
    }

    /// Does this path name a TUPLE STRUCT this crate declares?
    ///
    /// A tuple struct's name IS its constructor in Rust, so a capitalised call
    /// on one is a construction the registry can settle rather than the shape
    /// of the name guessing at. `Self(..)` inside the type's own impl is the
    /// same constructor.
    fn names_a_tuple_struct(&self, callee: Option<&syn::Path>) -> bool {
        let Some(path) = callee else { return false };
        let Some(tc) = &self.types else { return false };
        let tc = tc.borrow();
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let id = if segments == ["Self"] {
            match tc.self_ty.as_ref() {
                Some(crate::ty::Ty::Named { id, .. }) => *id,
                _ => return false,
            }
        } else {
            match tc.registry.lookup(tc.module, crate::registry::Ns::Type, &segments) {
                Ok(Some(crate::registry::Def::Type(id))) => id,
                _ => return false,
            }
        };
        let Some(def) = tc.registry.def(id) else { return false };
        matches!(def.kind, crate::registry::TypeKind::Struct)
            && !def.field_order.is_empty()
            && def.field_order.iter().all(|f| f.starts_with('_'))
    }

    pub(crate) fn translate_call(
        &self,
        func: &str,
        args: &[String],
        span: proc_macro2::Span,
        callee: Option<&syn::Path>,
    ) -> String {
        // 0. Resolve inline module qualifiers (e.g., stack.track → track)
        // Resolve inline module qualifiers (e.g., stack.track → track).
        // Import generation is handled by codegen.rs scanning the translated bodies.
        for mod_name in &self.inline_module_names {
            let prefix = format!("{}.", mod_name);
            if let Some(stripped) = func.strip_prefix(&prefix) {
                return self.translate_call(stripped, args, span, callee);
            }
        }

        // 1. Language-level constructs
        match func {
            "Self" => return format!("new {}({})", self.self_type, args.join(", ")),
            "Ok" => return format!("Result.Ok({})", args.join(", ")),
            "Err" => return format!("Result.Err({})", args.join(", ")),
            "Some" if args.len() == 1 => return args[0].clone(),
            "Some" => return args.join(", "),
            "None" => return "null".to_string(),
            // `drop(x)` takes x by value and runs its glue there and then. The
            // move analysis has already taken x off the block's list — it is an
            // argument passed by value like any other — so this releases it
            // once, where the source says.
            "drop" | "mem.drop" | "mem::drop" if args.len() == 1 => {
                return format!("{}.drop()", args[0]);
            }
            // `forget` is the one thing this model cannot express: it hands a
            // value to nobody and cancels its drop. Emitting the release would
            // run glue Rust suppressed, and emitting nothing leaks.
            "mem.forget" | "mem::forget" | "forget" if args.len() == 1 => {
                self.fallback(
                    span,
                    "`mem::forget` suppresses drop glue, which the emitted ownership model \
                     has no way to say; the value is left to the leak registry",
                );
                return format!("/* mem::forget */ void {}", args[0]);
            }
            _ => {}
        }

        // 2. Native type static calls (Vec::new, HashMap::new, Arc::clone, etc.)
        //
        // The table is keyed by the written name, so it is only consulted where
        // that name does not belong to a type this crate declared. A crate's own
        // `Vec` is its own class, and `Vec::new()` on it is `Vec.new()`, not a
        // JavaScript array literal.
        if !self.names_crate_type(callee) {
            if let Some(result) = native_types::translate_static_call(func, args) {
                // A `BTreeMap` keeps its keys in order and the port has no
                // ordered container: the runtime's `HashMap` hashes its keys
                // and iterates in insertion order, so anything that read this
                // one back in key order reads it in another.
                if func.starts_with("BTreeMap") || func.starts_with("BTreeSet") {
                    self.fallback(
                        span,
                        format!(
                            "`{}` keeps its keys in order, and the port's keyed containers \
                             iterate in insertion order; what reads this back in key order \
                             reads it in another",
                            func
                        ),
                    );
                }
                // `new HashMap()` says nothing about what it holds, so
                // TypeScript reads it as `HashMap<unknown, unknown>` and every
                // use of it after that is an error. The position knows: a `let`
                // with a written type, a field, a return.
                return self.with_container_arguments(result, span);
            }
        }

        // 3. Serde/bincode crate calls
        match func {
            // `to_string` answers a `Result<String, Error>`, and it was emitted
            // as a bare string — so `storage-sqlite/engine.ts` called `.mapErr`
            // on one and tsc refused it. And `JSON.stringify` throws on a
            // `bigint` and rounds a wide integer, where serde_json writes the
            // token exactly (R3).
            "serde_json.to_string" | "serde_json::to_string" | "serde_json.toString"
                if args.len() == 1 =>
            {
                return format!("serde_json.stringify(({}).toJSON())", args[0])
            }
            // `from_str::<T>(s)` parses the text and then asks `T` to read
            // itself out of it, which is what `Deserialize` does. Parsing alone
            // hands back a plain object where a `T` was wanted.
            "serde_json.from_str" | "serde_json::from_str" | "serde_json.fromStr"
                if args.len() == 1 =>
            {
                // Only where the type really has the static. `T.fromJson(..)`
                // on a `T` that derives no `Deserialize` turns a parse error
                // into a `TypeError` at the call, with nothing said.
                let target = self.read_into_type(callee, span);
                let declares = target.as_ref().is_some_and(|_| self.reads_json(callee, span));
                // `from_str` answers a `Result`, and so does the port's parse:
                // `JSON.parse` throws where Rust returns `Err`, and reads a
                // `u64` above 2^53 as a rounded double (R3).
                return match target.and_then(class_head).filter(|_| declares) {
                    Some(ty) => format!(
                        "serde_json.parse({}).andThen((v) => {}.fromJson(v))",
                        args[0], ty
                    ),
                    None => {
                        self.fallback(
                            span,
                            "`serde_json::from_str` is written without a type that reads itself \
                             out of the parsed value — a `#[derive(Deserialize)]` is what writes \
                             one — so the parse stands alone and hands back a plain object",
                        );
                        format!("serde_json.parse({})", args[0])
                    }
                };
            }
            "bincode.serialize" | "bincode::serialize" if args.len() == 1 =>
                return format!("(() => {{ const _w = new BincodeWriter(); {}.encode(_w); return _w.finish(); }})()", args[0]),
            "bincode.deserialize" | "bincode::deserialize" if args.len() == 1 => {
                return match self.read_into_type(callee, span) {
                    // What reads the bytes depends on the shape: a `Map` is a
                    // length and its entries, a `Vec` a length and its
                    // elements, and only a type with a class of its own has a
                    // `decode` static. Writing `Map<K, V>.decode(_r)` named a
                    // type where a value belongs and called a static nothing
                    // declares.
                    Some(ty) => format!(
                        "(() => {{ const _r = new BincodeReader({}); return {}; }})()",
                        args[0],
                        crate::bincode_module::decode_expr_with(&ty, "_r", None)
                    ),
                    None => {
                        self.fallback(
                            span,
                            "`bincode::deserialize` is written without saying which type reads \
                             itself out of the bytes, so the reader stands alone",
                        );
                        format!(
                            "(() => {{ const _r = new BincodeReader({}); return _r; }})()",
                            args[0]
                        )
                    }
                };
            }
            _ => {}
        }

        // 4. Box::new is transparent
        if matches!(func, "Box.new" | "Box::new") && args.len() == 1 {
            return args[0].clone();
        }

        // 5. Arc static methods → instance methods
        match func {
            "Arc.asPtr" | "Arc::asPtr" | "Arc.as_ptr" | "Arc::as_ptr"
                if args.len() == 1 => return format!("{}.asPtr()", args[0]),
            "Arc.downgrade" | "Arc::downgrade"
                if args.len() == 1 => return format!("{}.downgrade()", args[0]),
            _ => {}
        }

        // 6. Type::new() constructor pattern
        // System/base types (Arc, Mutex, RwLock, RefCell, etc.) use `new Type(args)` because
        // their TS constructors match the Rust ::new() signature directly.
        // Crate-defined types use `Type.new(args)` because the transpiler emits a
        // `static new()` method with custom initialization logic.
        if func.ends_with(".new") || func.ends_with("::new") {
            let type_name = func.trim_end_matches(".new").trim_end_matches("::new");
            let type_name = if type_name == "Self" { self.self_type } else { type_name };
            // System types with public constructors matching ::new() signature
            let use_constructor = matches!(type_name,
                "Mutex" | "RwLock" | "RefCell" | "HashMap" | "BTreeMap"
                | "HashSet" | "BTreeSet" | "Vec" | "RwLockReadGuard" | "RwLockWriteGuard"
                | "MutexGuard" | "Ref" | "RefMut" | "Box" | "ThreadLocal"
            );
            if use_constructor {
                return format!("new {}({})", type_name, args.join(", "));
            }
            // Everything else (crate-defined types + Arc/Weak): use static new()
            return format!("{}.new({})", type_name, args.join(", "));
        }

        // 7. Self::method() → TypeName.method()
        if func.starts_with("Self.") || func.starts_with("Self::") {
            // The path may already have been written in TypeScript —
            // `Self.setupReceiver` — or still be Rust — `Self::setup_receiver`.
            // Splitting on `::` alone left the whole of the first, so the call
            // came out `LocalProcessConnection.Self.setupReceiver`.
            let method = func.rsplit("::").next().unwrap_or(func);
            let method = method.rsplit('.').next().unwrap_or(method);
            return format!("{}.{}({})", self.self_type, method, args.join(", "));
        }

        // The port's `serde_json` reads and writes plain JavaScript values, so a
        // `Value::X(payload)` is the identity on its payload rather than an
        // enum construction: `@ankurah/base` exports no `Value` at all.
        {
            let segments: Vec<String> = func.split('.').map(str::to_string).collect();
            if let Some(written) =
                crate::native_types::js_value::json_value_construction(&segments, args)
            {
                return written;
            }
            if let Some(why) = crate::native_types::js_value::json_from_value_refusal(&segments) {
                return self.hole(span, why);
            }
        }

        // 8. Enum variant constructor: Type.Variant(args) → new Type('Variant', {...})
        if let Some(dot) = func.rfind('.') {
            let type_name = &func[..dot];
            let variant = &func[dot+1..];

            // The registry answers wherever it has a declaration, which is now
            // everywhere a body is translated — match arms included. A type from
            // another crate has no declaration here, because each crate is
            // transpiled on its own; the engine says "not a variant" and the
            // call is written as the associated function the other crate's
            // TypeScript exposes. Only a translation path with no type context
            // at all is left to guess from the shape of the name.
            let is_enum_variant = match &self.types {
                Some(tc) => tc.borrow().is_variant(type_name, variant),
                None => {
                    let guess = type_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && variant.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && !matches!(type_name, "Math" | "JSON" | "Object" | "Array" | "console" | "Promise");
                    if guess {
                        self.fallback(
                            span,
                            format!("`{}` is guessed to be an enum variant from its capitalisation", func),
                        );
                    }
                    guess
                }
            };

            if is_enum_variant {
                if args.is_empty() {
                    return format!("new {}('{}', {{}})", type_name, variant);
                } else if args.len() == 1 {
                    return format!("new {}('{}', {{ _0: {} }})", type_name, variant, args[0]);
                } else {
                    let fields: Vec<String> = args.iter().enumerate()
                        .map(|(i, a)| format!("_{}: {}", i, a))
                        .collect();
                    return format!("new {}('{}', {{ {} }})", type_name, variant, fields.join(", "));
                }
            }
        }

        // 9. A capitalised name that RESOLVES to a tuple struct is that
        // struct's constructor — the registry says so, and a guess from the
        // shape of the name is only what is left where it does not. `Clock(..)`,
        // `CollectionId(..)` and `AttestationSet(..)` are eleven such calls in
        // `proto`, every one of them a tuple struct this crate declares.
        if func.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && !func.contains('.')
            && !matches!(func, "Ok" | "Some" | "Err" | "None" | "Self")
        {
            if !self.names_a_tuple_struct(callee) {
                self.fallback(
                    span,
                    format!("`{}` is guessed to be a constructor from its capitalisation", func),
                );
            }
            return format!("new {}({})", func, args.join(", "));
        }

        // 10. Default: plain function call
        format!("{}({})", func, args.join(", "))
    }

}

/// The class a static is called on: a type's name without its type arguments.
///
/// `Attested<Event>.fromJson(..)` names an instantiation where a value belongs,
/// and TypeScript reads it as one. `None` where the type has no class at all —
/// a `Map`, an array, a primitive, or a wrapper whose payload the position
/// never named — because there is then no static to call.
fn class_head(ts_type: String) -> Option<String> {
    let head = ts_type.split('<').next().unwrap_or(&ts_type).trim().to_string();
    if head.is_empty()
        || head.ends_with("[]")
        || head.contains(' ')
        || matches!(
            head.as_str(),
            "Map" | "Set" | "Result" | "Option" | "string" | "number" | "bigint" | "boolean"
                | "unknown" | "Uint8Array" | "void" | "never"
        )
    {
        return None;
    }
    Some(head)
}
