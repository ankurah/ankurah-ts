//! `futures` 0.3.31 — the `Stream` trait, the extension traits, and the
//! channel and combinator types the corpus names.
//!
//! `core/src/util/ready_chunks.rs` writes `impl Stream` by hand and calls
//! `poll_next_unpin`, so both the trait's own `poll_next` and `StreamExt`'s
//! `poll_next_unpin` have to be declared, and they are different methods.
//!
//! Every combinator carries its `Stream` or `Future` impl. Without them a call
//! like `.map(f)` produces a nominal `Map<St, F>` at which the next call or the
//! `.await` has nothing to resolve against, which is the failure mode that
//! makes a half-declared extension trait worse than none.

pub trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>>;
    fn size_hint(&self) -> (usize, Option<usize>);
}

/// Every `Stream` of `Result` is a `TryStream`; the blanket is what gives
/// `try_collect` and friends their `Ok`/`Error` projections.
pub trait TryStream: Stream {
    type Ok;
    type Error;
    fn try_poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Result<Self::Ok, Self::Error>>>;
}

impl<S: ?Sized + Stream<Item = Result<T, E>>, T, E> TryStream for S {
    type Ok = T;
    type Error = E;
    fn try_poll_next(self: Pin<&mut S>, cx: &mut std::task::Context<'_>) -> Poll<Option<Result<T, E>>> { todo!() }
}

pub trait StreamExt: Stream {
    /// Deviation, deliberate: real `futures` returns a named `Next<'_, Self>`
    /// future here and the caller awaits it. This is declared `async fn`, so
    /// `s.next().await` gets the right type but `let f = s.next();` gets
    /// `Option<Item>` instead of `Next<'_, Self>`. Every corpus site awaits
    /// immediately. Restore the named future if one ever does not.
    async fn next(&mut self) -> Option<Self::Item> where Self: Unpin;
    fn poll_next_unpin(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> where Self: Unpin;
    fn map<T, F: FnMut(Self::Item) -> T>(self, f: F) -> stream::Map<Self, F> where Self: Sized;
    fn filter<Fut: Future<Output = bool>, F: FnMut(&Self::Item) -> Fut>(self, f: F) -> stream::Filter<Self, Fut, F> where Self: Sized;
    fn filter_map<Fut: Future<Output = Option<T>>, T, F: FnMut(Self::Item) -> Fut>(self, f: F) -> stream::FilterMap<Self, Fut, F> where Self: Sized;
    fn then<Fut: Future, F: FnMut(Self::Item) -> Fut>(self, f: F) -> stream::Then<Self, Fut, F> where Self: Sized;
    fn collect<C: Default + Extend<Self::Item>>(self) -> stream::Collect<Self, C> where Self: Sized;
    fn fold<T, Fut: Future<Output = T>, F: FnMut(T, Self::Item) -> Fut>(self, init: T, f: F) -> stream::Fold<Self, Fut, T, F> where Self: Sized;
    fn for_each<Fut: Future<Output = ()>, F: FnMut(Self::Item) -> Fut>(self, f: F) -> stream::ForEach<Self, Fut, F> where Self: Sized;
    fn take(self, n: usize) -> stream::Take<Self> where Self: Sized;
    fn skip(self, n: usize) -> stream::Skip<Self> where Self: Sized;
    fn chain<St: Stream<Item = Self::Item>>(self, other: St) -> stream::Chain<Self, St> where Self: Sized;
    fn boxed<'a>(self) -> stream::BoxStream<'a, Self::Item> where Self: Sized + Send + 'a;
    fn boxed_local<'a>(self) -> stream::LocalBoxStream<'a, Self::Item> where Self: Sized + 'a;
}

impl<T: ?Sized + Stream> StreamExt for T {}

/// The `Result`-aware half of the stream surface. `try_collect` and `map_ok`
/// are the two the storage crates reach for.
pub trait TryStreamExt: TryStream {
    async fn try_next(&mut self) -> Result<Option<Self::Ok>, Self::Error> where Self: Unpin;
    fn map_ok<T, F: FnMut(Self::Ok) -> T>(self, f: F) -> stream::MapOk<Self, F> where Self: Sized;
    fn map_err<E, F: FnMut(Self::Error) -> E>(self, f: F) -> stream::MapErr<Self, F> where Self: Sized;
    fn try_collect<C: Default + Extend<Self::Ok>>(self) -> stream::TryCollect<Self, C> where Self: Sized;
    fn try_for_each<Fut: Future<Output = Result<(), Self::Error>>, F: FnMut(Self::Ok) -> Fut>(self, f: F) -> stream::TryForEach<Self, Fut, F> where Self: Sized;
    fn try_filter_map<Fut: Future<Output = Result<Option<T>, Self::Error>>, T, F: FnMut(Self::Ok) -> Fut>(self, f: F) -> stream::TryFilterMap<Self, Fut, F> where Self: Sized;
    fn into_stream(self) -> stream::IntoStream<Self> where Self: Sized;
}

impl<S: ?Sized + TryStream> TryStreamExt for S {}

pub trait FutureExt: Future {
    fn poll_unpin(&mut self, cx: &mut std::task::Context<'_>) -> Poll<<Self as Future>::Output> where Self: Unpin;
    fn map<U, F: FnOnce(<Self as Future>::Output) -> U>(self, f: F) -> future::Map<Self, F> where Self: Sized;
    fn then<Fut: Future, F: FnOnce(<Self as Future>::Output) -> Fut>(self, f: F) -> future::Then<Self, Fut, F> where Self: Sized;
    fn boxed<'a>(self) -> future::BoxFuture<'a, <Self as Future>::Output> where Self: Sized + Send + 'a;
    fn boxed_local<'a>(self) -> future::LocalBoxFuture<'a, <Self as Future>::Output> where Self: Sized + 'a;
    fn shared(self) -> future::Shared<Self> where Self: Sized, <Self as Future>::Output: Clone;
    fn fuse(self) -> future::Fuse<Self> where Self: Sized;
}

impl<T: ?Sized + Future> FutureExt for T {}

pub trait TryFutureExt: Future {
    fn map_ok<T, U, E, F: FnOnce(T) -> U>(self, f: F) -> future::MapOk<Self, F> where Self: Sized + Future<Output = Result<T, E>>;
    fn map_err<T, E, U, F: FnOnce(E) -> U>(self, f: F) -> future::MapErr<Self, F> where Self: Sized + Future<Output = Result<T, E>>;
}

impl<T: ?Sized + Future> TryFutureExt for T {}

pub mod stream {
    pub struct Map<St, F>;
    pub struct Filter<St, Fut, F>;
    pub struct FilterMap<St, Fut, F>;
    pub struct Then<St, Fut, F>;
    pub struct Collect<St, C>;
    pub struct Fold<St, Fut, T, F>;
    pub struct ForEach<St, Fut, F>;
    pub struct Take<St>;
    pub struct Skip<St>;
    pub struct Chain<St1, St2>;
    pub struct Iter<I>;
    pub struct Empty<T>;
    pub struct Once<Fut>;
    pub struct MapOk<St, F>;
    pub struct MapErr<St, F>;
    pub struct TryCollect<St, C>;
    pub struct TryForEach<St, Fut, F>;
    pub struct TryFilterMap<St, Fut, F>;
    pub struct IntoStream<St>;

    /// `BoxStream` and its local twin are type aliases in `futures`, not
    /// structs. Declaring them as structs would sever the `Stream` relationship
    /// the alias carries, and `.await`, projections and every extension method
    /// would stop at the boxed value.
    pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;
    pub type LocalBoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + 'a>>;

    pub fn iter<I: IntoIterator>(i: I) -> futures::stream::Iter<<I as IntoIterator>::IntoIter> { todo!() }
    pub fn empty<T>() -> Empty<T> { todo!() }
    pub fn once<Fut: Future>(future: Fut) -> Once<Fut> { todo!() }

    impl<St: Stream, F: FnMut(<St as Stream>::Item) -> T, T> Stream for Map<St, F> {
        type Item = T;
        fn poll_next(self: Pin<&mut Map<St, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<T>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream, Fut: Future<Output = bool>, F: FnMut(&<St as Stream>::Item) -> Fut> Stream for Filter<St, Fut, F> {
        type Item = <St as Stream>::Item;
        fn poll_next(self: Pin<&mut Filter<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<St as Stream>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream, Fut: Future<Output = Option<T>>, T, F: FnMut(<St as Stream>::Item) -> Fut> Stream for FilterMap<St, Fut, F> {
        type Item = T;
        fn poll_next(self: Pin<&mut FilterMap<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<T>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream, Fut: Future, F: FnMut(<St as Stream>::Item) -> Fut> Stream for Then<St, Fut, F> {
        type Item = <Fut as Future>::Output;
        fn poll_next(self: Pin<&mut Then<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<Fut as Future>::Output>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream> Stream for Take<St> {
        type Item = <St as Stream>::Item;
        fn poll_next(self: Pin<&mut Take<St>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<St as Stream>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream> Stream for Skip<St> {
        type Item = <St as Stream>::Item;
        fn poll_next(self: Pin<&mut Skip<St>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<St as Stream>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St1: Stream, St2: Stream<Item = <St1 as Stream>::Item>> Stream for Chain<St1, St2> {
        type Item = <St1 as Stream>::Item;
        fn poll_next(self: Pin<&mut Chain<St1, St2>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<St1 as Stream>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<I: Iterator> Stream for Iter<I> {
        type Item = <I as Iterator>::Item;
        fn poll_next(self: Pin<&mut futures::stream::Iter<I>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<I as Iterator>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<T> Stream for Empty<T> {
        type Item = T;
        fn poll_next(self: Pin<&mut Empty<T>>, cx: &mut std::task::Context<'_>) -> Poll<Option<T>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<Fut: Future> Stream for Once<Fut> {
        type Item = <Fut as Future>::Output;
        fn poll_next(self: Pin<&mut Once<Fut>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<Fut as Future>::Output>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: TryStream, F: FnMut(<St as TryStream>::Ok) -> T, T> Stream for MapOk<St, F> {
        type Item = Result<T, <St as TryStream>::Error>;
        fn poll_next(self: Pin<&mut MapOk<St, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<Result<T, <St as TryStream>::Error>>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: TryStream, F: FnMut(<St as TryStream>::Error) -> E, E> Stream for MapErr<St, F> {
        type Item = Result<<St as TryStream>::Ok, E>;
        fn poll_next(self: Pin<&mut MapErr<St, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<Result<<St as TryStream>::Ok, E>>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<St: Stream> Stream for IntoStream<St> {
        type Item = <St as Stream>::Item;
        fn poll_next(self: Pin<&mut IntoStream<St>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<St as Stream>::Item>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    // The terminal combinators are futures, not streams.
    impl<St: Stream, C: Default + Extend<<St as Stream>::Item>> Future for Collect<St, C> {
        type Output = C;
        fn poll(self: Pin<&mut Collect<St, C>>, cx: &mut std::task::Context<'_>) -> Poll<C> { todo!() }
    }

    impl<St: Stream, Fut: Future<Output = T>, T, F: FnMut(T, <St as Stream>::Item) -> Fut> Future for Fold<St, Fut, T, F> {
        type Output = T;
        fn poll(self: Pin<&mut Fold<St, Fut, T, F>>, cx: &mut std::task::Context<'_>) -> Poll<T> { todo!() }
    }

    impl<St: Stream, Fut: Future<Output = ()>, F: FnMut(<St as Stream>::Item) -> Fut> Future for ForEach<St, Fut, F> {
        type Output = ();
        fn poll(self: Pin<&mut ForEach<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<()> { todo!() }
    }

    impl<St: TryStream, C: Default + Extend<<St as TryStream>::Ok>> Future for TryCollect<St, C> {
        type Output = Result<C, <St as TryStream>::Error>;
        fn poll(self: Pin<&mut TryCollect<St, C>>, cx: &mut std::task::Context<'_>) -> Poll<Result<C, <St as TryStream>::Error>> { todo!() }
    }

    impl<St: TryStream, Fut: Future<Output = Result<(), <St as TryStream>::Error>>, F: FnMut(<St as TryStream>::Ok) -> Fut> Future for TryForEach<St, Fut, F> {
        type Output = Result<(), <St as TryStream>::Error>;
        fn poll(self: Pin<&mut TryForEach<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Result<(), <St as TryStream>::Error>> { todo!() }
    }

    impl<St: TryStream, Fut: Future<Output = Result<Option<T>, <St as TryStream>::Error>>, T, F: FnMut(<St as TryStream>::Ok) -> Fut> Stream for TryFilterMap<St, Fut, F> {
        type Item = Result<T, <St as TryStream>::Error>;
        fn poll_next(self: Pin<&mut TryFilterMap<St, Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Option<Result<T, <St as TryStream>::Error>>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    /// `FuturesUnordered<Pin<Box<F>>>` is what `core/src/util/ready_chunks.rs`
    /// polls. Construction, `FromIterator` and `Default` place no bound on
    /// `Fut`; only the `Stream` impl needs it to be a future.
    pub struct FuturesUnordered<Fut>;

    impl<Fut> FuturesUnordered<Fut> {
        pub fn new() -> FuturesUnordered<Fut> { todo!() }
        pub fn push(&self, future: Fut) { todo!() }
        pub fn len(&self) -> usize { todo!() }
        pub fn is_empty(&self) -> bool { todo!() }
        pub fn iter(&self) -> FuturesUnorderedIter<'_, Fut> { todo!() }
    }

    pub struct FuturesUnorderedIter<'a, Fut>;

    impl<'a, Fut> Iterator for FuturesUnorderedIter<'a, Fut> {
        type Item = &'a Fut;
        fn next(&mut self) -> Option<&'a Fut> { todo!() }
    }

    impl<Fut: Future> Stream for FuturesUnordered<Fut> {
        type Item = <Fut as Future>::Output;
        fn poll_next(self: Pin<&mut FuturesUnordered<Fut>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<Fut as Future>::Output>> { todo!() }
        fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
    }

    impl<Fut> FromIterator<Fut> for FuturesUnordered<Fut> {
        fn from_iter<I: IntoIterator<Item = Fut>>(iter: I) -> FuturesUnordered<Fut> { todo!() }
    }

    impl<Fut> Default for FuturesUnordered<Fut> { fn default() -> FuturesUnordered<Fut> { todo!() } }
}

pub mod future {
    pub struct Map<Fut, F>;
    pub struct Then<Fut1, Fut2, F>;
    pub struct Shared<Fut>;
    pub struct Fuse<Fut>;
    pub struct JoinAll<F>;
    pub struct MapOk<Fut, F>;
    pub struct MapErr<Fut, F>;

    pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
    pub type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

    /// `join_all` collects futures, so the iterator's item has to be one; the
    /// returned `JoinAll` yields a `Vec` of their outputs in order.
    pub fn join_all<I: IntoIterator>(iter: I) -> JoinAll<<I as IntoIterator>::Item>
    where <I as IntoIterator>::Item: Future { todo!() }
    pub fn ready<T>(t: T) -> std::future::Ready<T> { todo!() }
    pub fn pending<T>() -> std::future::Pending<T> { todo!() }

    impl<F: Future> Future for JoinAll<F> {
        type Output = Vec<<F as Future>::Output>;
        fn poll(self: Pin<&mut JoinAll<F>>, cx: &mut std::task::Context<'_>) -> Poll<Vec<<F as Future>::Output>> { todo!() }
    }

    impl<Fut: Future, F: FnOnce(<Fut as Future>::Output) -> U, U> Future for Map<Fut, F> {
        type Output = U;
        fn poll(self: Pin<&mut Map<Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<U> { todo!() }
    }

    impl<Fut1: Future, Fut2: Future, F: FnOnce(<Fut1 as Future>::Output) -> Fut2> Future for Then<Fut1, Fut2, F> {
        type Output = <Fut2 as Future>::Output;
        fn poll(self: Pin<&mut Then<Fut1, Fut2, F>>, cx: &mut std::task::Context<'_>) -> Poll<<Fut2 as Future>::Output> { todo!() }
    }

    impl<Fut: Future> Future for Shared<Fut> where <Fut as Future>::Output: Clone {
        type Output = <Fut as Future>::Output;
        fn poll(self: Pin<&mut Shared<Fut>>, cx: &mut std::task::Context<'_>) -> Poll<<Fut as Future>::Output> { todo!() }
    }

    impl<Fut: Future> Clone for Shared<Fut> { fn clone(&self) -> Shared<Fut> { todo!() } }

    impl<Fut: Future> Future for Fuse<Fut> {
        type Output = <Fut as Future>::Output;
        fn poll(self: Pin<&mut Fuse<Fut>>, cx: &mut std::task::Context<'_>) -> Poll<<Fut as Future>::Output> { todo!() }
    }

    impl<Fut: Future<Output = Result<T, E>>, T, E, F: FnOnce(T) -> U, U> Future for MapOk<Fut, F> {
        type Output = Result<U, E>;
        fn poll(self: Pin<&mut MapOk<Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Result<U, E>> { todo!() }
    }

    impl<Fut: Future<Output = Result<T, E>>, T, E, F: FnOnce(E) -> U, U> Future for MapErr<Fut, F> {
        type Output = Result<T, U>;
        fn poll(self: Pin<&mut MapErr<Fut, F>>, cx: &mut std::task::Context<'_>) -> Poll<Result<T, U>> { todo!() }
    }
}

pub mod channel {
    pub mod oneshot {
        pub struct Sender<T>;
        pub struct Receiver<T>;
        pub struct Canceled;

        pub fn channel<T>() -> (Sender<T>, Receiver<T>) { todo!() }

        impl<T> Sender<T> {
            pub fn send(self, t: T) -> Result<(), T> { todo!() }
            pub fn is_canceled(&self) -> bool { todo!() }
        }

        impl<T> Receiver<T> {
            pub fn close(&mut self) { todo!() }
            pub fn try_recv(&mut self) -> Result<Option<T>, Canceled> { todo!() }
        }

        impl<T> Future for Receiver<T> {
            type Output = Result<T, Canceled>;
            fn poll(self: Pin<&mut Receiver<T>>, cx: &mut std::task::Context<'_>) -> Poll<Result<T, Canceled>> { todo!() }
        }

        impl Debug for Canceled { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::fmt::Display for Canceled { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::error::Error for Canceled {}
    }

    pub mod mpsc {
        pub struct Sender<T>;
        pub struct Receiver<T>;
        pub struct UnboundedSender<T>;
        pub struct UnboundedReceiver<T>;
        pub struct SendError;
        pub struct TrySendError<T>;

        pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) { todo!() }
        pub fn unbounded<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) { todo!() }

        impl<T> UnboundedSender<T> {
            pub fn unbounded_send(&self, msg: T) -> Result<(), TrySendError<T>> { todo!() }
            pub fn close_channel(&self) { todo!() }
            pub fn is_closed(&self) -> bool { todo!() }
        }
        impl<T> Clone for UnboundedSender<T> { fn clone(&self) -> UnboundedSender<T> { todo!() } }

        impl<T> Stream for UnboundedReceiver<T> {
            type Item = T;
            fn poll_next(self: Pin<&mut UnboundedReceiver<T>>, cx: &mut std::task::Context<'_>) -> Poll<Option<T>> { todo!() }
            fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
        }

        impl<T> Stream for Receiver<T> {
            type Item = T;
            fn poll_next(self: Pin<&mut Receiver<T>>, cx: &mut std::task::Context<'_>) -> Poll<Option<T>> { todo!() }
            fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
        }

        impl<T> Sender<T> {
            pub fn try_send(&mut self, msg: T) -> Result<(), TrySendError<T>> { todo!() }
        }
        impl<T> Clone for Sender<T> { fn clone(&self) -> Sender<T> { todo!() } }

        impl<T> Debug for TrySendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl<T> std::fmt::Display for TrySendError<T> { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl Debug for SendError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::fmt::Display for SendError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
        impl std::error::Error for SendError {}
    }
}

impl<S: ?Sized + Stream + Unpin> Stream for &mut S {
    type Item = <S as Stream>::Item;
    fn poll_next(self: Pin<&mut &mut S>, cx: &mut std::task::Context<'_>) -> Poll<Option<<S as Stream>::Item>> { todo!() }
    fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
}

impl<P: DerefMut + Unpin> Stream for Pin<P> where <P as Deref>::Target: Stream {
    type Item = <<P as Deref>::Target as Stream>::Item;
    fn poll_next(self: Pin<&mut Pin<P>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<<P as Deref>::Target as Stream>::Item>> { todo!() }
    fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
}

impl<S: ?Sized + Stream + Unpin> Stream for Box<S> {
    type Item = <S as Stream>::Item;
    fn poll_next(self: Pin<&mut Box<S>>, cx: &mut std::task::Context<'_>) -> Poll<Option<<S as Stream>::Item>> { todo!() }
    fn size_hint(&self) -> (usize, Option<usize>) { todo!() }
}
