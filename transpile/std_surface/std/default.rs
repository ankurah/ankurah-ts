//! `std::default`

pub trait Default: Sized {
    fn default() -> Self;
}

impl Default for bool { fn default() -> bool { todo!() } }
impl Default for char { fn default() -> char { todo!() } }
impl Default for u8 { fn default() -> u8 { todo!() } }
impl Default for u16 { fn default() -> u16 { todo!() } }
impl Default for u32 { fn default() -> u32 { todo!() } }
impl Default for u64 { fn default() -> u64 { todo!() } }
impl Default for u128 { fn default() -> u128 { todo!() } }
impl Default for usize { fn default() -> usize { todo!() } }
impl Default for i8 { fn default() -> i8 { todo!() } }
impl Default for i16 { fn default() -> i16 { todo!() } }
impl Default for i32 { fn default() -> i32 { todo!() } }
impl Default for i64 { fn default() -> i64 { todo!() } }
impl Default for i128 { fn default() -> i128 { todo!() } }
impl Default for isize { fn default() -> isize { todo!() } }
impl Default for f32 { fn default() -> f32 { todo!() } }
impl Default for f64 { fn default() -> f64 { todo!() } }
impl Default for () { fn default() -> () { todo!() } }
