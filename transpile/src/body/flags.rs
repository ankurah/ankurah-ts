//! Where a statement's MOVE FLAGS stand, and what has to stand above them.
//!
//! A flag is written before the statement it belongs to, because after a
//! `return` it would be dead code. That is right for the move itself and wrong
//! for everything the statement evaluates on the way there: an argument that
//! throws leaves the flag set and the moved value released by nobody.

use super::BodyTranslator;

impl BodyTranslator<'_> {
    /// Give a value a name that stands ABOVE the statement's move flags.
    fn name_above_the_flag(&self, written: String) -> String {
        let name = self.fresh_hoist("_b");
        self.own
            .before_flags
            .borrow_mut()
            .push(format!("const {} = {};\n", name, written));
        name
    }

    /// The arguments of a call whose CALLEE carries a move flag, each given a
    /// name that stands above the flag.
    ///
    /// J3: a move flag is written before the statement it belongs to, because
    /// after a `return` it would be dead code. That is right for the move
    /// itself and wrong for everything the statement evaluates on the way
    /// there: an argument that throws leaves the flag set and the callee
    /// released by nobody. Naming the arguments first puts every expression
    /// that can throw above the flag, and a place — a name, a field, a literal
    /// — is left alone, because reading one cannot throw and naming it would
    /// only add noise.
    pub(crate) fn lifted_above_the_flag(
        &self,
        callee: &syn::Expr,
        exprs: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
        written: Vec<String>,
    ) -> Vec<String> {
        let syn::Expr::Path(path) = callee else { return written };
        let Some(name) = crate::ownership::moves::local_name(path) else { return written };
        if !self.own.flags.borrow().contains_key(&name) {
            return written;
        }
        written
            .into_iter()
            .enumerate()
            .map(|(index, text)| match exprs.iter().nth(index) {
                Some(expr) if !crate::body::is_place(expr) => self.name_above_the_flag(text),
                _ => text,
            })
            .collect()
    }
}
