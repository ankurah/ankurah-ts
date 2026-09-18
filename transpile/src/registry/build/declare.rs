//! Pass one: what each module declares, in both namespaces.
//!
//! Every type, trait and alias a file writes gets its identity here, before any
//! written type is resolved, because resolving one needs the others to exist.

use super::vis_of;
use crate::diag::DiagSink;
use crate::registry::module::{ModuleId, Vis};
use crate::registry::{AliasDef, TypeDecl, TypeKind, TypeRegistry, ValueDef};
use crate::types::RustFile;

/// Pass one: the module's imports and everything it declares, in both namespaces.
pub(in crate::registry) fn declare_file(reg: &mut TypeRegistry, module: ModuleId, file: &RustFile, sink: &DiagSink) {
    let bindings = crate::registry::uses::module_use_bindings(reg, module, file, sink);
    reg.modules_mut().get_mut(module).uses.extend(bindings);

    for s in &file.structs {
        let vis = vis_of(s.vis, module, reg, sink);
        declare(
            reg,
            module,
            s.name.clone(),
            TypeKind::Struct,
            s.type_params.clone(),
            vis,
            Some(s.constructor),
            sink,
        );
    }
    for e in &file.enums {
        let vis = vis_of(e.vis, module, reg, sink);
        let kind = TypeKind::Enum {
            variants: Vec::new(),
        };
        declare(
            reg,
            module,
            e.name.clone(),
            kind,
            e.type_params.clone(),
            vis,
            None,
            sink,
        );
    }
    for t in &file.traits {
        let vis = vis_of(t.vis, module, reg, sink);
        declare(
            reg,
            module,
            t.name.clone(),
            TypeKind::Trait,
            t.type_params.clone(),
            vis,
            None,
            sink,
        );
    }
    for a in &file.type_aliases {
        let vis = vis_of(a.vis, module, reg, sink);
        let def = AliasDef {
            module,
            name: a.name.clone(),
            type_params: a.type_params.clone(),
            param_defaults: a.param_defaults.clone(),
            rust_ty: a.rust_ty.clone(),
        };
        reg.declare_alias(module, def, vis);
    }
    for c in &file.consts {
        let vis = vis_of(c.vis, module, reg, sink);
        reg.declare_value(
            module,
            ValueDef {
                name: c.name.clone(),
                ty: None,
                sig: None,
                // Settled after the impl table is built, where `Copy` can be
                // asked; a const that is not a fresh value is the common case.
                fresh_at_each_use: false,
            },
            vis,
        );
    }
    for f in &file.functions {
        let vis = vis_of(f.vis, module, reg, sink);
        reg.declare_value(
            module,
            ValueDef {
                name: f.name.clone(),
                ty: None,
                sig: None,
                fresh_at_each_use: false,
            },
            vis,
        );
    }

    for (name, sub_file) in &file.inline_modules {
        let child = reg.modules_mut().child(module, name);
        let vis = vis_of(sub_file.vis, child, reg, sink);
        reg.modules_mut().get_mut(child).vis = vis;
        declare_file(reg, child, sub_file, sink);
    }
}

fn declare(
    reg: &mut TypeRegistry,
    module: ModuleId,
    name: String,
    kind: TypeKind,
    type_params: Vec<String>,
    vis: Vis,
    constructor: Option<crate::types::Constructor>,
    sink: &DiagSink,
) {
    let decl = TypeDecl {
        name: name.clone(),
        kind,
        type_params,
        vis,
        constructor,
    };
    if let Err(err) = reg.declare_type(module, decl) {
        sink.push(crate::diag::Diag {
            file: sink.file(),
            line: 0,
            col: 0,
            message: format!("cannot declare `{}`: {}", name, err),
        });
    }
}
