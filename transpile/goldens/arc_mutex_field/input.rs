use std::sync::{Arc, Mutex};

/// The shared-inner shape ankurah's signals use: a handle over `Arc<Inner>`
/// whose state sits behind a lock.
pub struct Counter(Arc<Inner>);

struct Inner {
    label: Mutex<String>,
}

impl Counter {
    pub fn new(label: String) -> Self { Counter(Arc::new(Inner { label: Mutex::new(label) })) }

    /// The read under test: through the Arc, then through the Mutex guard.
    pub fn label_len(&self) -> usize { self.0.label.lock().unwrap().len() }

    pub fn set_label(&self, label: String) { *self.0.label.lock().unwrap() = label; }
}
