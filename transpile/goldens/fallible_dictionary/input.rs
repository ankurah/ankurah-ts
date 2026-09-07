//! A conversion bound whose impl is a genuine `TryFrom` (spec 4.4b).
//!
//! `pick` cannot say which impl converts its `V`, so its caller hands the
//! conversion in. Where the caller's type is concrete and what converts it is a
//! `TryFrom`, the emitted static ALREADY answers the `Result` the bound reads,
//! so it is handed over as it stands. Asked for a `From` instead, this site
//! found nothing and wrote a hole that threw the moment `pick` read it.
//!
//! `shifted` is the other half. Its first argument is a block whose type the
//! engine cannot name. Dropped from the list instead of held at its own index,
//! the `7` after it answered for the parameter before it, and the call was
//! handed `From<i64>` — a conversion for a type that stands nowhere in it.

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
