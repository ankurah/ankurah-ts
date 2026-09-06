//! The two questions a `&mut` to a JavaScript VALUE raises (C1).
//!
//! A number, a string and a boolean are copied at a call, so a callee's writes
//! through a `&mut` to one go nowhere. Such a value lives in a runtime cell
//! instead: these say which locals need one and which parameters are one.

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
