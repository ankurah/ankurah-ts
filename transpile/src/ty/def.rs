//! The type value itself.

/// A type's identity in the registry. Two types with the same leaf name in
/// different modules get different ids, which is what keeps a crate's `Ref`
/// away from `std::cell::Ref`.
///
/// Ids at or above `FOREIGN_BASE` name a type the corpus mentions but nothing
/// declares — `ulid::Ulid`, `anyhow::Error`. They have an identity and a name
/// and no known members, which is exactly what the engine knows about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

/// The id spaces are a partition of `u32`; running out of either is a bug in
/// the corpus size, not something to paper over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdSpaceExhausted;

impl std::fmt::Display for IdSpaceExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the type id space is exhausted")
    }
}

impl TypeId {
    pub const FOREIGN_BASE: u32 = 1 << 31;

    pub fn is_foreign(self) -> bool {
        self.0 >= Self::FOREIGN_BASE
    }

    /// Position within whichever half of the id space this id belongs to.
    pub fn index(self) -> usize {
        if self.is_foreign() {
            (self.0 - Self::FOREIGN_BASE) as usize
        } else {
            self.0 as usize
        }
    }
}

/// Rust's built-in scalar types. The integer widths stay apart: the wire format
/// is width-sensitive and `i64` is not `i32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prim {
    Bool,
    Char,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    F32,
    F64,
}

impl Prim {
    /// Is this one of the integer widths? An unsuffixed literal takes the type
    /// of whatever it is written against, and only an integer can be that.
    pub fn is_integer(self) -> bool {
        matches!(
            self,
            Prim::U8
                | Prim::U16
                | Prim::U32
                | Prim::U64
                | Prim::U128
                | Prim::Usize
                | Prim::I8
                | Prim::I16
                | Prim::I32
                | Prim::I64
                | Prim::I128
                | Prim::Isize
        )
    }

    /// Is this one of the SIGNED integer widths? Only those have a `MIN` with
    /// no positive of its own, which is what makes negating one fallible.
    pub fn is_signed_integer(self) -> bool {
        matches!(
            self,
            Prim::I8 | Prim::I16 | Prim::I32 | Prim::I64 | Prim::I128 | Prim::Isize
        )
    }

    /// The Rust name, which is also the name the runtime knows the width by.
    pub fn rust_name(self) -> String {
        format!("{:?}", self).to_lowercase()
    }

    /// The inclusive range of values this integer type holds, as the port
    /// mirrors it.
    ///
    /// For: two places have to agree on the bounds — the arithmetic helpers,
    /// which panic outside them, and the JSON readers, which refuse a document
    /// that carries a value outside them. They disagreed: a reader typed by the
    /// TypeScript SPELLING accepted `1.5`, `-1` and `256` for a `u8`, because
    /// all three are `typeof v === 'number'`.
    ///
    /// R13: `usize` and `isize` are 32-bit here. The port's target is wasm32,
    /// where that is what they are; the 8 bytes they occupy on the bincode wire
    /// is a separate fact, and belongs to the codec.
    ///
    /// The answer is `(i128, u128)` rather than a pair of one type, because no
    /// single Rust integer holds both `i128::MIN` and `u128::MAX`. Every
    /// maximum is non-negative and every minimum fits an `i128`, so the pair is
    /// exact for all twelve widths — `u128` included, which used to be left out
    /// and therefore accepted `-1` in a JSON document and was filtered out of
    /// the arithmetic dispatch.
    pub fn range(self) -> Option<(i128, u128)> {
        Some(match self {
            Prim::U8 => (0, 255),
            Prim::U16 => (0, 65_535),
            Prim::U32 => (0, 4_294_967_295),
            Prim::Usize => (0, 4_294_967_295),
            Prim::U64 => (0, 18_446_744_073_709_551_615),
            Prim::U128 => (0, u128::MAX),
            Prim::I8 => (-128, 127),
            Prim::I16 => (-32_768, 32_767),
            Prim::I32 => (-2_147_483_648, 2_147_483_647),
            Prim::Isize => (-2_147_483_648, 2_147_483_647),
            Prim::I64 => (-9_223_372_036_854_775_808, 9_223_372_036_854_775_807),
            Prim::I128 => (i128::MIN, i128::MAX as u128),
            Prim::Bool | Prim::Char | Prim::F32 | Prim::F64 => return None,
        })
    }

    pub fn from_rust_name(name: &str) -> Option<Prim> {
        Some(match name {
            "bool" => Prim::Bool,
            "char" => Prim::Char,
            "u8" => Prim::U8,
            "u16" => Prim::U16,
            "u32" => Prim::U32,
            "u64" => Prim::U64,
            "u128" => Prim::U128,
            "usize" => Prim::Usize,
            "i8" => Prim::I8,
            "i16" => Prim::I16,
            "i32" => Prim::I32,
            "i64" => Prim::I64,
            "i128" => Prim::I128,
            "isize" => Prim::Isize,
            "f32" => Prim::F32,
            "f64" => Prim::F64,
            _ => return None,
        })
    }
}

/// How long an array is. A const generic is allowed here and nowhere else.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArrayLen {
    Lit(u64),
    /// A const generic parameter or a named constant, e.g. `[T; N]`.
    Named(String),
}

/// A trait named in a bound or a trait object, with its arguments and any
/// associated-type bindings written alongside them.
///
/// `Fn(A, B) -> R` is stored the way Rust desugars it: one argument holding the
/// tuple of inputs (`Ty::Unit` when there are none) and an `Output` binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitRef {
    pub id: TypeId,
    pub args: Vec<Ty>,
    pub bindings: Vec<(String, Ty)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Prim(Prim),

    /// A struct, enum, trait-as-type or system type, with its type arguments.
    Named {
        id: TypeId,
        args: Vec<Ty>,
    },

    /// A generic type parameter, by the name its declaration gives it.
    Param(String),

    /// `impl Trait` in argument position: Rust's anonymous generic parameter.
    /// It has no name to look bounds up by, so it carries them.
    ImplTrait {
        bounds: Vec<TraitRef>,
    },

    /// `&T` / `&mut T`. Lifetimes are dropped; emission erases the reference.
    Ref {
        mutable: bool,
        inner: Box<Ty>,
    },

    Tuple(Vec<Ty>),

    Array {
        elem: Box<Ty>,
        len: ArrayLen,
    },

    Slice(Box<Ty>),

    /// The unsized `str`. `String` is an ordinary named system type.
    Str,

    /// `()`, the empty tuple.
    Unit,

    /// `!`.
    Never,

    /// `dyn Trait + Send + Sync`, in the order written.
    Dyn {
        traits: Vec<TraitRef>,
    },

    /// A projection such as `<I as Iterator>::Item`. `trait_` is the trait it
    /// projects through when the source names one, with the arguments that
    /// trait was written with — `<T as From<u8>>::Out` keeps the `u8`.
    Assoc {
        base: Box<Ty>,
        trait_: Option<Box<TraitRef>>,
        name: String,
    },

    /// A `_` written in an annotation or a turbofish.
    Infer,
}

impl Ty {
    /// Strip `&` / `&mut` off the outside. Method and field lookup work on the
    /// referent; references are an emission-invisible wrapper.
    pub fn peel_refs(&self) -> &Ty {
        let mut ty = self;
        while let Ty::Ref { inner, .. } = ty {
            ty = inner;
        }
        ty
    }

    pub fn id(&self) -> Option<TypeId> {
        match self {
            Ty::Named { id, .. } => Some(*id),
            _ => None,
        }
    }
}
