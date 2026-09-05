//! What serde makes of one declaration, resolved before either half is written.
//!
//! For: a type has TWO serializers — the binary one and the human-readable one
//! — and serde builds both from one description. The port used to build them
//! from two: `bincode_module` dispatched on the resolved `Ty`, `json_module` on
//! the TypeScript spelling, and the only thing they shared was
//! `FieldInfo::serde_with`. So `#[serde(other)]` worked in one half and not the
//! other, `#[serde(transparent)]` in neither, and the JSON half decided whether
//! a value had a `fromJson` by asking whether its spelling started with a
//! capital letter — which put ten calls in the corpus to a static no class
//! declares.
//!
//! This module answers, for one struct or enum: which fields are IN the format,
//! what key each is written under, and what shape each one's type has. The
//! answer is a value both halves can read, and every serde attribute the corpus
//! writes is decided here rather than at the point of emission.

use crate::registry::TypeRegistry;
use crate::types::{EnumInfo, FieldInfo, StructInfo};

/// One field, as serde sees it.
pub(super) struct Member {
    /// The property on the emitted class.
    pub ts_name: String,
    /// The key serde writes, which is the Rust field name.
    pub key: String,
    /// The TypeScript spelling of its type.
    pub ts_ty: String,
    pub shape: super::shape::Shape,
}

/// How a container is written.
pub(super) enum Body {
    /// `struct Unit;` — serde writes `null`.
    Unit,
    /// One field, no wrapper: a newtype struct, and any container serde was
    /// told is `transparent`.
    Transparent(Member),
    /// An object keyed by the Rust field names.
    Named(Vec<Member>),
    /// An array, one element per field, in declaration order.
    Positional(Vec<Member>),
}

/// One enum variant, as serde sees it.
pub(super) struct Variant {
    pub name: String,
    pub key: String,
    pub body: Body,
    /// `#[serde(other)]` — the variant an unknown tag reads as.
    pub is_other: bool,
}

pub(super) struct StructSchema {
    pub body: Body,
}

pub(super) struct EnumSchema {
    pub variants: Vec<Variant>,
}

/// The schema for a struct, or the reason it has none.
pub(super) fn of_struct(reg: &TypeRegistry, info: &StructInfo) -> Result<StructSchema, String> {
    let kept: Vec<&FieldInfo> = info
        .fields
        .iter()
        .filter(|f| !f.serde_skip && !super::shape::is_zero_sized(&f.ts_ty(reg)))
        .collect();
    // serde reads a skipped field back as `Default::default()`. The only
    // skipped field the corpus has is a `PhantomData`, which the emitted class
    // does not carry at all — so there is nothing to build. Any other would
    // need that type's `Default`, which the port does not carry, and is
    // refused rather than filled in with `undefined`.
    for field in info.fields.iter().filter(|f| f.serde_skip) {
        if !super::shape::is_zero_sized(&field.ts_ty(reg)) {
            return Err(format!(
                "`{}` has a `#[serde(skip)]` field the reader would have to build from \
                 `Default::default()`, and the port carries no default for `{}`",
                info.name,
                field.ts_ty(reg)
            ));
        }
    }
    // `#[serde(transparent)]` says: this container IS its one remaining field.
    // A newtype struct is transparent whether or not the attribute is written.
    let transparent = info.serde_transparent || (kept.len() == 1 && kept[0].rust_name.is_none());
    if transparent {
        if kept.len() != 1 {
            return Err(format!(
                "`{}` is `#[serde(transparent)]` and has {} fields the format keeps; serde \
                 allows exactly one",
                info.name,
                kept.len()
            ));
        }
        return Ok(StructSchema {
            body: Body::Transparent(member(reg, kept[0], 0)?),
        });
    }
    if kept.is_empty() {
        return Ok(StructSchema { body: Body::Unit });
    }
    Ok(StructSchema {
        body: body_of(reg, &kept)?,
    })
}

/// The schema for an enum, or the reason it has none.
pub(super) fn of_enum(reg: &TypeRegistry, info: &EnumInfo) -> Result<EnumSchema, String> {
    if info.serde_transparent {
        return Err(format!(
            "`{}` is `#[serde(transparent)]`, which serde allows on a struct and not on an \
             enum",
            info.name
        ));
    }
    let mut variants = Vec::new();
    for variant in &info.variants {
        let kept: Vec<&FieldInfo> = variant
            .fields
            .iter()
            .filter(|f| !f.serde_skip && !super::shape::is_zero_sized(&f.ts_ty(reg)))
            .collect();
        let body = if kept.is_empty() {
            Body::Unit
        } else if kept.len() == 1 && kept[0].rust_name.is_none() {
            Body::Transparent(member(reg, kept[0], 0)?)
        } else {
            body_of(reg, &kept)?
        };
        variants.push(Variant {
            key: variant.name.clone(),
            name: variant.name.clone(),
            body,
            is_other: variant.is_serde_other,
        });
    }
    Ok(EnumSchema { variants })
}

fn body_of(reg: &TypeRegistry, kept: &[&FieldInfo]) -> Result<Body, String> {
    let named = kept.iter().all(|f| f.rust_name.is_some());
    let mut members = Vec::new();
    for (i, field) in kept.iter().enumerate() {
        members.push(member(reg, field, i)?);
    }
    Ok(if named {
        Body::Named(members)
    } else {
        Body::Positional(members)
    })
}

fn member(reg: &TypeRegistry, field: &FieldInfo, index: usize) -> Result<Member, String> {
    let ts_name = field
        .name
        .clone()
        .unwrap_or_else(|| format!("_{}", index));
    let key = field.rust_name.clone().unwrap_or_else(|| ts_name.clone());
    let ts_ty = field.ts_ty(reg);
    let shape = super::shape::of_field(reg, field, &ts_ty)?;
    Ok(Member {
        ts_name,
        key,
        ts_ty,
        shape,
    })
}
