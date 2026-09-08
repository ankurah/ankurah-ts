// A generic constructor is typed by what the closure handed to it ANSWERS.
//
// `Calculated::new(|| a + b)` declares `fn new<F>(compute: F) -> Self where
// F: Fn() -> T`, and nothing but the closure's tail says what `T` is. The two
// types are never equal — a bound is a capability, not a type — so the
// constraint is between what the bound answers and what the closure answers.

pub struct Calculated<T> {
    value: T,
}

impl<T> Calculated<T> {
    pub fn new<F>(compute: F) -> Calculated<T>
    where
        F: Fn() -> T,
    {
        Calculated { value: compute() }
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn is_ready(&self) -> bool {
        true
    }
}

pub fn width(text: String) -> usize {
    let counted = Calculated::new(move || text.len());
    counted.into_value()
}

pub fn built_and_kept(text: String) -> bool {
    let counted = Calculated::new(move || text.len());
    counted.is_ready()
}
