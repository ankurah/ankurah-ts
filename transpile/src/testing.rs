//! A miniature crate, built the way `batch` builds a real one.
//!
//! Tests hand this a few files of Rust source and then ask the same questions
//! the transpiler asks: what type is this, what does this path resolve to, and
//! what TypeScript comes out the other end.

use crate::diag::{Diag, DiagSink};
use crate::extract;
use crate::registry::{
    build_registry, resolve_type, ExtractedFile, ModuleId, SystemTypeDecl, TypeEnv, TypeRegistry,
};
use crate::ty::{Ty, TypeId};

pub struct Fixture {
    pub reg: TypeRegistry,
    pub sink: DiagSink,
    pub files: Vec<ExtractedFile>,
}

/// The system types the shipped configuration declares. Tests run against the
/// real table, so that a change to it shows up here rather than only in a
/// corpus diff.
pub fn system_decls() -> Vec<SystemTypeDecl> {
    crate::config::Config::load(std::path::Path::new("transpile.toml"))
        .map(|c| c.system_types)
        .unwrap_or_default()
}

impl Fixture {
    pub fn build(files: &[(&str, &str)]) -> Fixture {
        Fixture::build_named("testcrate", files)
    }

    pub fn build_named(crate_name: &str, files: &[(&str, &str)]) -> Fixture {
        let sink = DiagSink::new();
        let mut parsed: Vec<ExtractedFile> = files
            .iter()
            .map(|(path, src)| ExtractedFile {
                path: path.to_string(),
                file: extract::extract_source(path, src, None).expect("parses"),
                declarations_only: false,
            })
            .collect();
        let reg = build_registry(
            &mut parsed,
            &system_decls(),
            &[crate_name.to_string()],
            &sink,
        );
        Fixture {
            reg,
            sink,
            files: parsed,
        }
    }

    pub fn module(&self, file: &str) -> ModuleId {
        self.reg
            .modules()
            .lookup_file(file)
            .expect("module was declared")
    }

    /// Resolve a written type as if it appeared in `file`.
    pub fn ty_in(&self, file: &str, src: &str, params: &[&str]) -> Result<Ty, Diag> {
        self.sink.set_file(file);
        let syn_ty: syn::Type = syn::parse_str(src).expect("parses as a type");
        let params: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let env = TypeEnv::new(&self.reg, self.module(file), &self.sink).with_params(&params);
        resolve_type(&syn_ty, &env)
    }

    pub fn ty(&self, file: &str, src: &str) -> Ty {
        self.ty_in(file, src, &[]).expect("resolves")
    }

    /// The declared type of a struct field, as the registry recorded it.
    pub fn field(&self, file: &str, struct_name: &str, field: &str) -> Ty {
        let entry = self.files.iter().find(|e| e.path == file).expect("file");
        let s = entry
            .file
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .expect("struct");
        let f = s
            .fields
            .iter()
            .find(|f| f.name.as_deref() == Some(field))
            .expect("field");
        f.ty.clone().expect("field type resolved")
    }

    pub fn named(&self, file: &str, name: &str, args: Vec<Ty>) -> Ty {
        let id = self
            .reg
            .module_type(self.module(file), name)
            .expect("declared here");
        Ty::Named { id, args }
    }

    pub fn system(&self, path: &str, args: Vec<Ty>) -> Ty {
        Ty::Named {
            id: self.system_id(path),
            args,
        }
    }

    pub fn system_id(&self, path: &str) -> TypeId {
        self.reg
            .system_type(path)
            .unwrap_or_else(|| panic!("no system type at `{}`", path))
    }

    /// A type context for `file`, as body translation would build one.
    pub fn context(&self, file: &str, self_ty: Option<Ty>) -> crate::infer::TypeContext<'_> {
        self.sink.set_file(file);
        crate::infer::TypeContext::new(
            &self.reg,
            self.module(file),
            self_ty,
            Vec::new(),
            &self.sink,
        )
    }

    pub fn messages(&self) -> Vec<String> {
        self.sink.sorted().into_iter().map(|d| d.message).collect()
    }

    /// Translate every body in `file` and hand back the TypeScript of one
    /// method, so a fix can be checked where it actually shows: in the output.
    pub fn translated_method(&mut self, file: &str, method: &str) -> String {
        let module = self.module(file);
        let entry = self
            .files
            .iter_mut()
            .find(|e| e.path == file)
            .expect("file");
        self.sink.set_file(file);
        crate::translate_module(&mut entry.file, &self.reg, module, &self.sink);
        entry
            .file
            .impls
            .iter()
            .flat_map(|i| i.methods.iter())
            .chain(entry.file.functions.iter())
            .chain(
                entry
                    .file
                    .inline_modules
                    .iter()
                    .flat_map(|(_, f)| f.functions.iter()),
            )
            .find(|m| m.name == method)
            .and_then(|m| m.body_ts.clone())
            .unwrap_or_else(|| panic!("no translated body for `{}`", method))
    }
}
