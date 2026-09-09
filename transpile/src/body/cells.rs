//! The two questions a `&mut` to a JavaScript VALUE raises (C1).
//!
//! A number, a string and a boolean are copied at a call, so a callee's writes
//! through a `&mut` to one go nowhere. Such a value lives in a runtime cell
//! instead: these say which locals need one and which parameters are one.

/// What reading a body's block says before any of it is written.
#[derive(Debug, Default)]
pub(crate) struct Prescan {
    /// Locals this body hands out as `&mut`. Whether each really needs a cell
    /// is settled at its `let`, which is the only place its type is known: a
    /// `&mut` to a class is already a reference in JavaScript.
    pub cells: Vec<String>,
    /// Locals TypeScript can grow an empty array into, by the emitted name.
    pub grown_in_sight: Vec<String>,
}

impl Prescan {
    pub(crate) fn of(block: &syn::Block) -> Prescan {
        Prescan {
            cells: cells_wanted(block),
            grown_in_sight: grown_in_sight(block),
        }
    }
}

/// Every local this block hands out as `&mut`, by the name the emitter writes.
///
/// A `&mut` to a class is already a reference in JavaScript and needs no cell;
/// the decision about the TYPE is made where the local is declared, which is
/// the only place the type is known. This is the syntactic half: which names
/// are borrowed mutably at all.
pub(crate) fn cells_wanted(block: &syn::Block) -> Vec<String> {
    struct Borrows {
        names: Vec<String>,
    }
    impl syn::visit::Visit<'_> for Borrows {
        fn visit_expr_reference(&mut self, node: &syn::ExprReference) {
            if node.mutability.is_some() {
                if let syn::Expr::Path(path) = &*node.expr {
                    if path.path.segments.len() == 1 {
                        let name = crate::name_map::escape_reserved(&crate::name_map::to_camel_case(
                            &path.path.segments[0].ident.to_string(),
                        ));
                        if !self.names.contains(&name) {
                            self.names.push(name);
                        }
                    }
                }
            }
            syn::visit::visit_expr_reference(self, node);
        }
        // A closure's own body borrows in its own scope.
        fn visit_expr_closure(&mut self, _: &syn::ExprClosure) {}
    }
    let mut borrows = Borrows { names: Vec::new() };
    syn::visit::Visit::visit_block(&mut borrows, block);
    borrows.names
}

/// Is this parameter a `&mut` to something the port writes as a JavaScript
/// VALUE, so that a write through it needs a runtime cell?
///
/// A `&mut` to a class is already a reference in JavaScript and needs nothing:
/// `fn fill(v: &mut Vec<u8>)` writes into the array the caller passed. A number,
/// a string, a boolean and a bigint are copied at the call, and so is a
/// nullable of one.
pub(crate) fn is_boxed_mut(param: &crate::types::ParamInfo) -> bool {
    let Some(syn::Type::Reference(reference)) = &param.rust_ty else {
        return false;
    };
    if reference.mutability.is_none() {
        return false;
    }
    crate::is_value_spelling(&param.ty)
}

/// Every local this block declares that TypeScript can grow `[]` into, by the
/// emitted name.
///
/// TypeScript grows `let xs = []` from a `push` it can SEE in the same
/// function, and a closure is its own function: a push inside a callback, and a
/// fill through a callee handed the local, are not ones it sees. Such a local
/// carries the type the engine worked out instead of an evolving `any[]`.
pub(crate) fn grown_in_sight(block: &syn::Block) -> Vec<String> {
    #[derive(Default)]
    struct Frame {
        declared: Vec<String>,
        pushed: Vec<String>,
    }
    struct Grown {
        frames: Vec<Frame>,
        names: Vec<String>,
    }
    impl Grown {
        fn close(&mut self) {
            let frame = self.frames.pop().unwrap_or_default();
            for name in frame.declared {
                if frame.pushed.contains(&name) {
                    self.names.push(name);
                }
            }
        }
        fn named(path: &syn::Path) -> Option<String> {
            (path.segments.len() == 1).then(|| {
                crate::name_map::escape_reserved(&crate::name_map::to_camel_case(
                    &path.segments[0].ident.to_string(),
                ))
            })
        }
    }
    impl syn::visit::Visit<'_> for Grown {
        fn visit_expr_closure(&mut self, node: &syn::ExprClosure) {
            self.frames.push(Frame::default());
            syn::visit::visit_expr_closure(self, node);
            self.close();
        }
        fn visit_local(&mut self, node: &syn::Local) {
            let name = crate::body::BodyTranslator::pat_static(&node.pat);
            if let Some(frame) = self.frames.last_mut() {
                frame.declared.push(name);
            }
            syn::visit::visit_local(self, node);
        }
        fn visit_expr_method_call(&mut self, node: &syn::ExprMethodCall) {
            if node.method == "push" {
                if let syn::Expr::Path(path) = &*node.receiver {
                    if let (Some(name), Some(frame)) =
                        (Self::named(&path.path), self.frames.last_mut())
                    {
                        frame.pushed.push(name);
                    }
                }
            }
            syn::visit::visit_expr_method_call(self, node);
        }
    }
    let mut grown = Grown {
        frames: vec![Frame::default()],
        names: Vec::new(),
    };
    syn::visit::Visit::visit_block(&mut grown, block);
    grown.close();
    grown.names
}
