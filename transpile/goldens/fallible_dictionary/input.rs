//! A conversion bound whose impl is a genuine `TryFrom` (spec 4.4b).
//!
//! `pick` cannot say which impl converts its `V`, so its caller hands the
//! conversion in, and a `TryFrom` static already answers the `Result` the bound
//! reads. Nothing converts `shifted`'s `String` to a `Sel`, so that conversion
//! is a hole held at its own index: dropped, the next argument answered for it.

pub struct Sel {
    pub text: String,
}

pub struct Bad;

impl TryFrom<&str> for Sel {
    type Error = Bad;
    fn try_from(text: &str) -> Result<Sel, Bad> {
        match text.is_empty() {
            true => Err(Bad),
            false => Ok(Sel { text: text.to_string() }),
        }
    }
}

impl From<i64> for Sel {
    fn from(count: i64) -> Sel {
        Sel { text: count.to_string() }
    }
}

pub fn pick<V>(v: V) -> Option<Sel>
where
    V: TryInto<Sel>,
{
    v.try_into().ok()
}

pub fn parsed() -> Option<Sel> {
    pick("year >= 2020")
}

pub fn refused() -> Option<Sel> {
    pick("")
}

pub fn counted() -> Option<Sel> {
    pick(7i64)
}

pub fn paired<A>(a: A, count: i64) -> i64
where
    A: TryInto<Sel>,
{
    match pick(a) {
        Some(sel) => sel.text.len() as i64 + count,
        None => count,
    }
}

pub fn shifted() -> i64 {
    paired(
        {
            let text = String::new();
            text
        },
        7i64,
    )
}
