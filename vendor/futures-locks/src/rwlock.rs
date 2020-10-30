// vim: tw=80

use futures::{Async, Future, Poll, sync::oneshot};
#[cfg(feature = "tokio")] use futures::future;
#[cfg(feature = "tokio")] use futures::future::IntoFuture;
use std::{
    cell::UnsafeCell,
    clone::Clone,
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync,
};
use super::FutState;
#[cfg(feature = "tokio")] use tokio_executor::{self, Executor, SpawnError};
#[cfg(feature = "tokio")] use tokio_current_thread as current_thread;

/// An RAII guard, much like `std::sync::RwLockReadGuard`.  The wrapped data can
/// be accessed via its `Deref` implementation.
pub struct RwLockReadGuard<T: ?Sized> {
    rwlock: RwLock<T>
}

impl<T: ?Sized> Deref for RwLockReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {&*self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<T> {
    fn drop(&mut self) {
        self.rwlock.unlock_reader();
    }
}

/// An RAII guard, much like `std::sync::RwLockWriteGuard`.  The wrapped data
/// can be accessed via its `Deref`  and `DerefMut` implementations.
pub struct RwLockWriteGuard<T: ?Sized> {
    rwlock: RwLock<T>
}

impl<T: ?Sized> Deref for RwLockWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {&*self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {&mut *self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<T> {
    fn drop(&mut self) {
        self.rwlock.unlock_writer();
    }
}

/// A `Future` representing a pending `RwLock` shared acquisition.
pub struct RwLockReadFut<T: ?Sized> {
    state: FutState,
    rwlock: RwLock<T>,
}

impl<T: ?Sized> RwLockReadFut<T> {
    fn new(state: FutState, rwlock: RwLock<T>) -> Self {
        RwLockReadFut{state, rwlock}
    }
}

impl<T: ?Sized> Drop for RwLockReadFut<T> {
    fn drop(&mut self) {
        match &mut self.state {
            &mut FutState::New => {
                // RwLock hasn't yet been modified; nothing to do
            },
            &mut FutState::Pending(ref mut rx) => {
                rx.close();
                match rx.try_recv() {
                    Ok(Some(())) => {
                        // This future received ownership of the lock, but got
                        // dropped before it was ever polled.  Release the
                        // lock.
                        self.rwlock.unlock_reader()
                    },
                    Ok(None) => {
                        // Dropping the Future before it acquires the lock is
                        // equivalent to cancelling it.
                    },
                    Err(oneshot::Canceled) => {
                        // Never received ownership of the lock
                    }
                }
            },
            &mut FutState::Acquired => {
                // The RwLockReadGuard will take care of releasing the RwLock
            }
        }
    }
}

impl<T: ?Sized> Future for RwLockReadFut<T> {
    type Item = RwLockReadGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (result, new_state) = match &mut self.state {
            &mut FutState::New => {
                let mut lock_data = self.rwlock.inner.mutex.lock()
                    .expect("sync::Mutex::lock");
                if lock_data.exclusive {
                    let (tx, mut rx) = oneshot::channel::<()>();
                    lock_data.read_waiters.push_back(tx);
                    // Even though we know it isn't ready, we need to poll the
                    // receiver in order to register our task for notification.
                    assert!(rx.poll().unwrap().is_not_ready());
                    (Ok(Async::NotReady), FutState::Pending(rx))
                } else {
                    lock_data.num_readers += 1;
                    let guard = RwLockReadGuard{rwlock: self.rwlock.clone()};
                    (Ok(Async::Ready(guard)), FutState::Acquired)
                }
            },
            &mut FutState::Pending(ref mut rx) => {
                match rx.poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    // It's impossible for receiver.poll() to return an error.
                    // The only way that would happen is if the sender got
                    // dropped.  But that can't happen because the RwLock owns
                    // the sender, and the Fut retains a clone of the RwLock.
                    Err(_) => unreachable!(),
                    Ok(Async::Ready(_)) => {
                        let state = FutState::Acquired;
                        let result = Ok(Async::Ready(
                                RwLockReadGuard{rwlock: self.rwlock.clone()}));
                        (result, state)
                    }  // LCOV_EXCL_LINE   kcov false negative
                }
            },
            &mut FutState::Acquired => panic!("Double-poll of ready Future")
        };
        self.state = new_state;
        result
    }
}

/// A `Future` representing a pending `RwLock` exclusive acquisition.
pub struct RwLockWriteFut<T: ?Sized> {
    state: FutState,
    rwlock: RwLock<T>,
}

impl<T: ?Sized> RwLockWriteFut<T> {
    fn new(state: FutState, rwlock: RwLock<T>) -> Self {
        RwLockWriteFut{state, rwlock}
    }
}

impl<T: ?Sized> Drop for RwLockWriteFut<T> {
    fn drop(&mut self) {
        match &mut self.state {
            &mut FutState::New => {
                // RwLock hasn't yet been modified; nothing to do
            },
            &mut FutState::Pending(ref mut rx) => {
                rx.close();
                match rx.try_recv() {
                    Ok(Some(())) => {
                        // This future received ownership of the lock, but got
                        // dropped before it was ever polled.  Release the
                        // lock.
                        self.rwlock.unlock_writer()
                    },
                    Ok(None) => {
                        // Dropping the Future before it acquires the lock is
                        // equivalent to cancelling it.
                    },
                    Err(oneshot::Canceled) => {
                        // Never received ownership of the lock
                    }
                }
            },
            &mut FutState::Acquired => {
                // The RwLockWriteGuard will take care of releasing the RwLock
            }
        }
    }
}

impl<T: ?Sized> Future for RwLockWriteFut<T> {
    type Item = RwLockWriteGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (result, new_state) = match &mut self.state {
            &mut FutState::New => {
                let mut lock_data = self.rwlock.inner.mutex.lock()
                    .expect("sync::Mutex::lock");
                if lock_data.exclusive || lock_data.num_readers > 0 {
                    let (tx, mut rx) = oneshot::channel::<()>();
                    lock_data.write_waiters.push_back(tx);
                    // Even though we know it isn't ready, we need to poll the
                    // receiver in order to register our task for notification.
                    assert!(rx.poll().unwrap().is_not_ready());
                    (Ok(Async::NotReady), FutState::Pending(rx))
                } else {
                    lock_data.exclusive = true;
                    let guard = RwLockWriteGuard{rwlock: self.rwlock.clone()};
                    (Ok(Async::Ready(guard)), FutState::Acquired)
                }
            },
            &mut FutState::Pending(ref mut rx) => {
                match rx.poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    // It's impossible for receiver.poll() to return an error.
                    // The only way that would happen is if the sender got
                    // dropped.  But that can't happen because the RwLock owns
                    // the sender, and the Fut retains a clone of the RwLock.
                    Err(_) => unreachable!(),
                    Ok(Async::Ready(_)) => {
                        let state = FutState::Acquired;
                        let result = Ok(Async::Ready(
                                RwLockWriteGuard{rwlock: self.rwlock.clone()}));
                        (result, state)
                    }  // LCOV_EXCL_LINE   kcov false negative
                }
            },
            &mut FutState::Acquired => panic!("Double-poll of ready Future")
        };
        self.state = new_state;
        result
    }
}

#[derive(Debug, Default)]
struct RwLockData {
    /// True iff the `RwLock` is currently exclusively owned
    exclusive: bool,

    /// The number of tasks that currently have shared ownership of the RwLock
    num_readers: u32,

    // FIFO queue of waiting readers
    read_waiters: VecDeque<oneshot::Sender<()>>,

    // FIFO queue of waiting writers
    write_waiters: VecDeque<oneshot::Sender<()>>,
}

#[derive(Debug, Default)]
struct Inner<T: ?Sized> {
    mutex: sync::Mutex<RwLockData>,
    data: UnsafeCell<T>,
}

/// A Futures-aware RwLock.
///
/// `std::sync::RwLock` cannot be used in an asynchronous environment like
/// Tokio, because an acquisition can block an entire reactor.  This class can
/// be used instead.  It functions much like `std::sync::RwLock`.  Unlike that
/// class, it also has a builtin `Arc`, making it accessible from multiple
/// threads.  It's also safe to `clone`.  Also unlike `std::sync::RwLock`, this
/// class does not detect lock poisoning.
#[derive(Debug, Default)]
pub struct RwLock<T: ?Sized> {
    inner: sync::Arc<Inner<T>>,
}

impl<T: ?Sized> Clone for RwLock<T> {
    fn clone(&self) -> RwLock<T> {
        RwLock { inner: self.inner.clone()}
    }
}

impl<T> RwLock<T> {
    /// Create a new `RwLock` in the unlocked state.
    pub fn new(t: T) -> RwLock<T> {
        let lock_data = RwLockData {
            exclusive: false,
            num_readers: 0,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
        };  // LCOV_EXCL_LINE   kcov false negative
        let inner = Inner {
            mutex: sync::Mutex::new(lock_data),
            data: UnsafeCell::new(t)
        };  // LCOV_EXCL_LINE   kcov false negative
        RwLock { inner: sync::Arc::new(inner)}
    }

    /// Consumes the `RwLock` and returns the wrapped data.  If the `RwLock`
    /// still has multiple references (not necessarily locked), returns a copy
    /// of `self` instead.
    pub fn try_unwrap(self) -> Result<T, RwLock<T>> {
        match sync::Arc::try_unwrap(self.inner) {
            Ok(inner) => Ok({
                // `unsafe` is no longer needed as of somewhere around 1.25.0.
                // https://github.com/rust-lang/rust/issues/35067
                #[allow(unused_unsafe)]
                unsafe { inner.data.into_inner() }
            }),
            Err(arc) => Err(RwLock {inner: arc})
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Returns a reference to the underlying data, if there are no other
    /// clones of the `RwLock`.
    ///
    /// Since this call borrows the `RwLock` mutably, no actual locking takes
    /// place -- the mutable borrow statically guarantees no locks exist.
    /// However, if the `RwLock` has already been cloned, then `None` will be
    /// returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use futures_locks::*;
    /// # fn main() {
    /// let mut lock = RwLock::<u32>::new(0);
    /// *lock.get_mut().unwrap() += 5;
    /// assert_eq!(lock.try_unwrap().unwrap(), 5);
    /// # }
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if let Some(inner) = sync::Arc::get_mut(&mut self.inner) {
            let lock_data = inner.mutex.get_mut().unwrap();
            let data = unsafe { inner.data.get().as_mut() }.unwrap();
            debug_assert!(!lock_data.exclusive);
            debug_assert_eq!(lock_data.num_readers, 0);
            Some(data)
        } else {
            None
        }
    }

    /// Acquire the `RwLock` nonexclusively, read-only, blocking the task in the
    /// meantime.
    ///
    /// When the returned `Future` is ready, then this task will have read-only
    /// access to the protected data.
    ///
    /// # Examples
    /// ```
    /// # use futures_locks::*;
    /// # use futures::executor::{Spawn, spawn};
    /// # use futures::Future;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(42);
    /// let fut = rwlock.read().map(|mut guard| { *guard });
    /// assert_eq!(spawn(fut).wait_future(), Ok(42));
    /// # }
    ///
    /// ```
    pub fn read(&self) -> RwLockReadFut<T> {
        return RwLockReadFut::new(FutState::New, self.clone())
    }

    /// Acquire the `RwLock` exclusively, read-write, blocking the task in the
    /// meantime.
    ///
    /// When the returned `Future` is ready, then this task will have read-write
    /// access to the protected data.
    ///
    /// # Examples
    /// ```
    /// # use futures_locks::*;
    /// # use futures::executor::{Spawn, spawn};
    /// # use futures::Future;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(42);
    /// let fut = rwlock.write().map(|mut guard| { *guard = 5;});
    /// spawn(fut).wait_future().expect("spawn");
    /// assert_eq!(rwlock.try_unwrap().unwrap(), 5);
    /// # }
    ///
    /// ```
    pub fn write(&self) -> RwLockWriteFut<T> {
        return RwLockWriteFut::new(FutState::New, self.clone())
    }

    /// Attempts to acquire the `RwLock` nonexclusively.
    ///
    /// If the operation would block, returns `Err` instead.  Otherwise, returns
    /// a guard (not a `Future`).
    ///
    /// # Examples
    /// ```
    /// # use futures_locks::*;
    /// # fn main() {
    /// let mut lock = RwLock::<u32>::new(5);
    /// let r = match lock.try_read() {
    ///     Ok(guard) => *guard,
    ///     Err(()) => panic!("Better luck next time!")
    /// };
    /// assert_eq!(5, r);
    /// # }
    /// ```
    pub fn try_read(&self) -> Result<RwLockReadGuard<T>, ()> {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        if lock_data.exclusive {
            Err(())
        } else {
            lock_data.num_readers += 1;
            Ok(RwLockReadGuard{rwlock: self.clone()})
        }
    }

    /// Attempts to acquire the `RwLock` exclusively.
    ///
    /// If the operation would block, returns `Err` instead.  Otherwise, returns
    /// a guard (not a `Future`).
    ///
    /// # Examples
    /// ```
    /// # use futures_locks::*;
    /// # fn main() {
    /// let mut lock = RwLock::<u32>::new(5);
    /// match lock.try_write() {
    ///     Ok(mut guard) => *guard += 5,
    ///     Err(()) => panic!("Better luck next time!")
    /// }
    /// assert_eq!(10, lock.try_unwrap().unwrap());
    /// # }
    /// ```
    pub fn try_write(&self) -> Result<RwLockWriteGuard<T>, ()> {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        if lock_data.exclusive || lock_data.num_readers > 0 {
            Err(())
        } else {
            lock_data.exclusive = true;
            Ok(RwLockWriteGuard{rwlock: self.clone()})
        }
    }

    /// Release a shared lock of an `RwLock`.
    fn unlock_reader(&self) {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        assert!(lock_data.num_readers > 0);
        assert!(!lock_data.exclusive);
        assert_eq!(lock_data.read_waiters.len(), 0);
        lock_data.num_readers -= 1;
        if lock_data.num_readers == 0 {
            while let Some(tx) = lock_data.write_waiters.pop_front() {
                if tx.send(()).is_ok() {
                    lock_data.exclusive = true;
                    return
                }
            }
        }
    }

    /// Release an exclusive lock of an `RwLock`.
    fn unlock_writer(&self) {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        assert!(lock_data.num_readers == 0);
        assert!(lock_data.exclusive);

        // First try to wake up any writers
        while let Some(tx) = lock_data.write_waiters.pop_front() {
            if tx.send(()).is_ok() {
                return;
            }
        }

        // If there are no writers, try to wake up readers
        lock_data.exclusive = false;
        lock_data.num_readers += lock_data.read_waiters.len() as u32;
        for tx in lock_data.read_waiters.drain(..) {
            // Ignore errors, which are due to a reader's future getting
            // dropped before it was ready
            let _ = tx.send(());
        }
    }
}

impl<T: 'static + ?Sized> RwLock<T> {
    /// Acquires a `RwLock` nonexclusively and performs a computation on its
    /// guarded value in a separate task.  Returns a `Future` containing the
    /// result of the computation.
    ///
    /// When using Tokio, this method will often hold the `RwLock` for less time
    /// than chaining a computation to [`read`](#method.read).  The reason is
    /// that Tokio polls all tasks promptly upon notification.  However, Tokio
    /// does not guarantee that it will poll all futures promptly when their
    /// owning task gets notified.  So it's best to hold `RwLock`s within their
    /// own tasks, lest their continuations get blocked by slow stacked
    /// combinators.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate tokio_ as tokio;
    /// # use futures_locks::*;
    /// # use futures::{Future, IntoFuture, lazy};
    /// # use tokio::runtime::current_thread::Runtime;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(5);
    /// let mut rt = Runtime::new().unwrap();
    /// let r = rt.block_on(lazy(|| {
    ///     rwlock.with_read(|mut guard| {
    ///         Ok(*guard) as Result<u32, ()>
    ///     }).unwrap()
    /// }));
    /// assert_eq!(r, Ok(5));
    /// # }
    /// ```
    #[cfg(any(feature = "tokio", all(feature = "nightly-docs", rustdoc)))]
    #[cfg_attr(feature = "nightly-docs", doc(cfg(feature = "tokio")))]
    pub fn with_read<F, B, R, E>(&self, f: F)
        -> Result<impl Future<Item = R, Error = E>, SpawnError>
        where F: FnOnce(RwLockReadGuard<T>) -> B + Send + 'static,
              B: IntoFuture<Item = R, Error = E> + 'static,
              <B as IntoFuture>::Future: Send,
              R: Send + 'static,
              E: Send + 'static,
              T: Send
    {
        let (tx, rx) = oneshot::channel::<Result<R, E>>();
        tokio_executor::DefaultExecutor::current().spawn(Box::new(self.read()
            .and_then(move |data| {
                f(data).into_future()
                       .then(move |result| {
                           // Swallow errors; there's nothing to do if the
                           // receiver got cancelled
                           let _ = tx.send(result);
                           future::ok::<(), ()>(())
                       })
            })
            // We control the sender so we're sure it won't be dropped before
            // sending so we can unwrap safely
        )).map(|_| rx.then(Result::unwrap))
    }

    /// Like [`with_read`](#method.with_read) but for Futures that aren't
    /// `Send`.  Spawns a new task on a single-threaded Runtime to complete the
    /// Future.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate tokio_ as tokio;
    /// # use futures_locks::*;
    /// # use futures::{Future, IntoFuture, lazy};
    /// # use std::rc::Rc;
    /// # use tokio::runtime::current_thread;
    /// # fn main() {
    /// // Note: Rc is not `Send`
    /// let rwlock = RwLock::<Rc<u32>>::new(Rc::new(5));
    /// let mut rt = current_thread::Runtime::new().unwrap();
    /// let r = rt.block_on(lazy(|| {
    ///     rwlock.with_read_local(|mut guard| {
    ///         Ok(**guard) as Result<u32, ()>
    ///     }).unwrap()
    /// }));
    /// assert_eq!(r, Ok(5));
    /// # }
    /// ```
    #[cfg(any(feature = "tokio", all(feature = "nightly-docs", rustdoc)))]
    #[cfg_attr(feature = "nightly-docs", doc(cfg(feature = "tokio")))]
    pub fn with_read_local<F, B, R, E>(&self, f: F)
        -> Result<impl Future<Item = R, Error = E>, SpawnError>
        where F: FnOnce(RwLockReadGuard<T>) -> B + 'static,
              B: IntoFuture<Item = R, Error = E> + 'static,
              R: 'static,
              E: 'static
    {
        let (tx, rx) = oneshot::channel::<Result<R, E>>();
        current_thread::TaskExecutor::current().spawn_local(Box::new(
            self.read()
            .and_then(move |data| {
                f(data).into_future()
                       .then(move |result| {
                           // Swallow errors; there's nothing to do if the
                           // receiver got cancelled
                           let _ = tx.send(result);
                           future::ok::<(), ()>(())
                       })
            })
            // We control the sender so we're sure it won't be dropped before
            // sending so we can unwrap safely
        )).map(|_| rx.then(Result::unwrap))
    }

    /// Acquires a `RwLock` exclusively and performs a computation on its
    /// guarded value in a separate task.  Returns a `Future` containing the
    /// result of the computation.
    ///
    /// When using Tokio, this method will often hold the `RwLock` for less time
    /// than chaining a computation to [`write`](#method.write).  The reason is
    /// that Tokio polls all tasks promptly upon notification.  However, Tokio
    /// does not guarantee that it will poll all futures promptly when their
    /// owning task gets notified.  So it's best to hold `RwLock`s within their
    /// own tasks, lest their continuations get blocked by slow stacked
    /// combinators.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate tokio_ as tokio;
    /// # use futures_locks::*;
    /// # use futures::{Future, IntoFuture, lazy};
    /// # use tokio::runtime::current_thread::Runtime;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(0);
    /// let mut rt = Runtime::new().unwrap();
    /// let r = rt.block_on(lazy(|| {
    ///     rwlock.with_write(|mut guard| {
    ///         *guard += 5;
    ///         Ok(()) as Result<(), ()>
    ///     }).unwrap()
    /// }));
    /// assert!(r.is_ok());
    /// assert_eq!(rwlock.try_unwrap().unwrap(), 5);
    /// # }
    /// ```
    #[cfg(any(feature = "tokio", all(feature = "nightly-docs", rustdoc)))]
    #[cfg_attr(feature = "nightly-docs", doc(cfg(feature = "tokio")))]
    pub fn with_write<F, B, R, E>(&self, f: F)
        -> Result<impl Future<Item = R, Error = E>, SpawnError>
        where F: FnOnce(RwLockWriteGuard<T>) -> B + Send + 'static,
              B: IntoFuture<Item = R, Error = E> + Send + 'static,
              <B as IntoFuture>::Future: Send,
              R: Send + 'static,
              E: Send + 'static,
              T: Send
    {
        let (tx, rx) = oneshot::channel::<Result<R, E>>();
        tokio_executor::DefaultExecutor::current().spawn(Box::new(self.write()
            .and_then(move |data| {
                f(data).into_future()
                       .then(move |result| {
                           // Swallow errors; there's nothing to do if the
                           // receiver got cancelled
                           let _ = tx.send(result);
                           future::ok::<(), ()>(())
                       })
            })
            // We control the sender so we're sure it won't be dropped before
            // sending so we can unwrap safely
        )).map(|_| rx.then(Result::unwrap))
    }

    /// Like [`with_write`](#method.with_write) but for Futures that aren't
    /// `Send`.  Spawns a new task on a single-threaded Runtime to complete the
    /// Future.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate tokio_ as tokio;
    /// # use futures_locks::*;
    /// # use futures::{Future, IntoFuture, lazy};
    /// # use std::rc::Rc;
    /// # use tokio::runtime::current_thread;
    /// # fn main() {
    /// // Note: Rc is not `Send`
    /// let rwlock = RwLock::<Rc<u32>>::new(Rc::new(0));
    /// let mut rt = current_thread::Runtime::new().unwrap();
    /// let r = rt.block_on(lazy(|| {
    ///     rwlock.with_write_local(|mut guard| {
    ///         *Rc::get_mut(&mut *guard).unwrap() += 5;
    ///         Ok(()) as Result<(), ()>
    ///     }).unwrap()
    /// }));
    /// assert!(r.is_ok());
    /// assert_eq!(*rwlock.try_unwrap().unwrap(), 5);
    /// # }
    /// ```
    #[cfg(any(feature = "tokio", all(feature = "nightly-docs", rustdoc)))]
    #[cfg_attr(feature = "nightly-docs", doc(cfg(feature = "tokio")))]
    pub fn with_write_local<F, B, R, E>(&self, f: F)
        -> Result<impl Future<Item = R, Error = E>, SpawnError>
        where F: FnOnce(RwLockWriteGuard<T>) -> B + 'static,
              B: IntoFuture<Item = R, Error = E> + 'static,
              R: 'static,
              E: 'static
    {
        let (tx, rx) = oneshot::channel::<Result<R, E>>();
        current_thread::TaskExecutor::current().spawn_local(Box::new(
            self.write()
            .and_then(move |data| {
                f(data).into_future()
                       .then(move |result| {
                           // Swallow errors; there's nothing to do if the
                           // receiver got cancelled
                           let _ = tx.send(result);
                           future::ok::<(), ()>(())
                       })
            })
            // We control the sender so we're sure it won't be dropped before
            // sending so we can unwrap safely
        )).map(|_| rx.then(Result::unwrap))
    }
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for RwLock<T> {}

// LCOV_EXCL_START
#[cfg(test)]
mod t {
    use super::*;

    /// Pet Kcov
    #[test]
    fn debug() {
        let m = RwLock::<u32>::new(0);
        format!("{:?}", &m);
    }

    #[test]
    fn test_default() {
        let lock = RwLock::default();
        let value: u32 = lock.try_unwrap().unwrap();
        let expected = u32::default();

        assert_eq!(expected, value);
    }
}
// LCOV_EXCL_STOP
