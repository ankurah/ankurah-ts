// Arc<T> / Weak<T> — reference-counted pointer translations

use super::MethodTranslation;
use crate::resolve::ResolvedType;

/// Translate Arc/Rc static calls (e.g., Arc::clone(&x) → x.clone())
pub fn translate_static(func: &str, args: &[String]) -> Option<String> {
    match func {
        // Arc::clone(&x) / Rc::clone(&x) — idiomatic Rust ref-counting
        "Arc.clone" | "Arc::clone" | "Rc.clone" | "Rc::clone" if args.len() == 1
            => Some(format!("{}.clone()", args[0])),
        // Arc::new(x) / Rc::new(x) — handled separately by constructor logic
        _ => None,
    }
}

/// Translate Arc/Weak method calls
pub fn translate(receiver_ty: &ResolvedType, receiver: &str, method: &str, args: &[String]) -> MethodTranslation {
    let type_name = match receiver_ty {
        ResolvedType::Named { name, .. } => name.as_str(),
        _ => return MethodTranslation::Passthrough,
    };

    match (type_name, method) {
        // Arc::downgrade() → new Weak(arc)
        ("Arc", "downgrade") => MethodTranslation::Expr(format!("Weak.new({})", receiver)),
        // Weak::upgrade() → weak.upgrade()
        ("Weak", "upgrade") => MethodTranslation::Passthrough,
        // Arc/Weak pointer identity
        ("Arc" | "Weak", "as_ptr" | "asPtr") => MethodTranslation::Passthrough,
        // Arc::strong_count, Weak::weak_count — passthrough
        ("Arc", "strong_count" | "strongCount") => MethodTranslation::Passthrough,
        ("Weak", "weak_count" | "weakCount") => MethodTranslation::Passthrough,
        _ => MethodTranslation::Passthrough,
    }
}
