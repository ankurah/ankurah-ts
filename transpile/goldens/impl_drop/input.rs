//! `impl Drop for T` is the type's own cleanup, and `AkObject.drop()` is the
//! template that calls it: mark, unregister, run this while every field is
//! still alive, then drop the fields. Overriding `drop()` would put the cleanup
//! after the cascade and hand the body dead fields.

pub struct Subscription {
    pub label: String,
    pub live: bool,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.live = false;
    }
}
