//! `gloo-timers` 0.3.0
//!
//! Not on the deliverable's list. One site: the websocket client's reconnect
//! backoff awaits `sleep(Duration::from_millis(delay))`.

pub mod future {
    pub struct TimeoutFuture;

    pub fn sleep(dur: Duration) -> TimeoutFuture { todo!() }

    impl Future for TimeoutFuture {
        type Output = ();
        fn poll(self: Pin<&mut TimeoutFuture>, cx: &mut std::task::Context<'_>) -> Poll<()> { todo!() }
    }
}

pub mod callback {
    pub struct Timeout;
    pub struct Interval;

    // `cancel` hands the closure back so the caller can drop or reuse it, and
    // `forget` hands back the JS handle. Both returning `()` would discard a
    // value the caller is meant to hold, and the ownership memo cares which.
    impl Timeout {
        pub fn new<F: FnOnce() + 'static>(millis: u32, callback: F) -> Timeout { todo!() }
        pub fn cancel(self) -> Closure<dyn FnMut()> { todo!() }
        pub fn forget(self) -> JsValue { todo!() }
    }

    impl Interval {
        pub fn new<F: FnMut() + 'static>(millis: u32, callback: F) -> Interval { todo!() }
        pub fn cancel(self) -> Closure<dyn FnMut()> { todo!() }
        pub fn forget(self) -> JsValue { todo!() }
    }
}
