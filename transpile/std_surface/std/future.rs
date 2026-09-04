//! `std::future`
//!
//! `.await` is not a method call and does not resolve through this trait; the
//! engine reads `Future::Output` off the awaited expression's type. That
//! projection is why the trait is declared: 201 awaits in `core` alone, and
//! `core/src/util/ready_chunks.rs` writes `impl Stream` by hand over
//! `FuturesUnordered<Pin<Box<F>>>`.

pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output>;
}

pub trait IntoFuture {
    type Output;
    type IntoFuture: Future<Output = Self::Output>;
    fn into_future(self) -> Self::IntoFuture;
}

impl<F: Future> IntoFuture for F {
    type Output = <F as Future>::Output;
    type IntoFuture = F;
    fn into_future(self) -> F { todo!() }
}

impl<F: Future + Unpin + ?Sized> Future for &mut F {
    type Output = <F as Future>::Output;
    fn poll(self: Pin<&mut &mut F>, cx: &mut std::task::Context<'_>) -> Poll<<F as Future>::Output> { todo!() }
}

impl<P: DerefMut> Future for Pin<P> where <P as Deref>::Target: Future {
    type Output = <<P as Deref>::Target as Future>::Output;
    fn poll(self: Pin<&mut Pin<P>>, cx: &mut std::task::Context<'_>) -> Poll<<<P as Deref>::Target as Future>::Output> { todo!() }
}

pub struct Ready<T>;
pub struct Pending<T>;

pub fn ready<T>(t: T) -> Ready<T> { todo!() }
pub fn pending<T>() -> Pending<T> { todo!() }

impl<T> Future for Ready<T> {
    type Output = T;
    fn poll(self: Pin<&mut Ready<T>>, cx: &mut std::task::Context<'_>) -> Poll<T> { todo!() }
}

impl<T> Future for Pending<T> {
    type Output = T;
    fn poll(self: Pin<&mut Pending<T>>, cx: &mut std::task::Context<'_>) -> Poll<T> { todo!() }
}
