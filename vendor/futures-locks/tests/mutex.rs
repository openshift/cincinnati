//vim: tw=80

use futures::{Future, Stream, future, lazy, stream};
#[cfg(feature = "tokio")]
use std::rc::Rc;
use tokio;
#[cfg(feature = "tokio")]
use tokio::runtime;
use tokio::runtime::current_thread;
use futures_locks::*;

// Create a MutexWeak and then upgrade it to Mutex
#[test]
fn mutex_weak_some() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_weak = Mutex::downgrade(&mutex);

    assert!(mutex_weak.upgrade().is_some())
}

// Create a MutexWeak and drop the mutex so that MutexWeak::upgrade return None
#[test]
fn mutex_weak_none() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_weak = Mutex::downgrade(&mutex);

    drop(mutex);

    assert!(mutex_weak.upgrade().is_none())
}

// Compare Mutexes if it point to the same value
#[test]
fn mutex_eq_ptr_true() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_other = mutex.clone();

    assert!(Mutex::ptr_eq(&mutex, &mutex_other));
}

// Compare Mutexes if it point to the same value
#[test]
fn mutex_eq_ptr_false() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_other = Mutex::<u32>::new(0);

    assert!(!Mutex::ptr_eq(&mutex, &mutex_other));
}

// When a Mutex gets dropped after gaining ownership but before being polled, it
// should drain its channel and relinquish ownership if a message was found.  If
// not, deadlocks may result.
#[test]
fn drop_when_ready() {
    let mutex = Mutex::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(lazy(|| {
        let mut fut1 = mutex.lock();
        let guard1 = fut1.poll();    // fut1 immediately gets ownership
        assert!(guard1.as_ref().unwrap().is_ready());
        let mut fut2 = mutex.lock();
        assert!(!fut2.poll().unwrap().is_ready());
        drop(guard1);                // ownership transfers to fut2
        drop(fut1);
        drop(fut2);                  // relinquish ownership
        let mut fut3 = mutex.lock();
        let guard3 = fut3.poll();    // fut3 immediately gets ownership
        assert!(guard3.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    })).unwrap();
}

// When a pending Mutex gets dropped after being polled() but before gaining
// ownership, ownership should pass on to the next waiter.
#[test]
fn drop_before_ready() {
    let mutex = Mutex::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(lazy(|| {
        let mut fut1 = mutex.lock();
        let guard1 = fut1.poll();    // fut1 immediately gets ownership
        assert!(guard1.as_ref().unwrap().is_ready());
        let mut fut2 = mutex.lock();
        assert!(!fut2.poll().unwrap().is_ready());
        let mut fut3 = mutex.lock();
        assert!(!fut3.poll().unwrap().is_ready());
        drop(fut2);                  // drop before gaining ownership
        drop(guard1);                // ownership transfers to fut3
        drop(fut1);
        let guard3 = fut3.poll();
        assert!(guard3.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    })).unwrap();
}

// Mutably dereference a uniquely owned Mutex
#[test]
fn get_mut() {
    let mut mutex = Mutex::<u32>::new(42);
    *mutex.get_mut().unwrap() += 1;
    assert_eq!(*mutex.get_mut().unwrap(), 43);
}

// Cloned Mutexes cannot be deferenced
#[test]
fn get_mut_cloned() {
    let mut mutex = Mutex::<u32>::new(42);
    let _clone = mutex.clone();
    assert!(mutex.get_mut().is_none());
}

// Acquire an uncontested Mutex.  poll immediately returns Async::Ready
#[test]
fn lock_uncontested() {
    let mutex = Mutex::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(lazy(|| {
        mutex.lock().map(|guard| {
            *guard + 5
        })
    })).unwrap();
    assert_eq!(result, 5);
}

// Pend on a Mutex held by another task in the same tokio Reactor.  poll returns
// Async::NotReady.  Later, it gets woken up without involving the OS.
#[test]
fn lock_contested() {
    let mutex = Mutex::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(lazy(|| {
        let mut fut0 = mutex.lock();
        let guard0 = fut0.poll();    // fut0 immediately gets ownership
        assert!(guard0.as_ref().unwrap().is_ready());

        let mut fut1 = mutex.lock();
        assert!(!fut1.poll().unwrap().is_ready());  // fut1 is blocked

        drop(guard0);               // Ownership transfers to fut1
        let guard1 = fut1.poll();
        assert!(guard1.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    })).unwrap();
}

// A single Mutex is contested by tasks in multiple threads
#[test]
fn lock_multithreaded() {
    let mutex = Mutex::<u32>::new(0);
    let mtx_clone0 = mutex.clone();
    let mtx_clone1 = mutex.clone();
    let mtx_clone2 = mutex.clone();
    let mtx_clone3 = mutex.clone();

    let parent = lazy(move || {
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone0.lock().map(|mut guard| { *guard += 2 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone1.lock().map(|mut guard| { *guard += 3 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone2.lock().map(|mut guard| { *guard += 5 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone3.lock().map(|mut guard| { *guard += 7 })
        }));
        future::ok::<(), ()>(())
    });

    tokio::run(parent);
    assert_eq!(mutex.try_unwrap().expect("try_unwrap"), 17_000);
}

// Mutexes should be acquired in the order that their Futures are waited upon.
#[test]
fn lock_order() {
    let mutex = Mutex::<Vec<u32>>::new(vec![]);
    let fut2 = mutex.lock().map(|mut guard| guard.push(2));
    let fut1 = mutex.lock().map(|mut guard| guard.push(1));
    let mut rt = current_thread::Runtime::new().unwrap();

    let r = rt.block_on(lazy(|| {
        fut1.and_then(|_| fut2)
    }));
    assert!(r.is_ok());
    assert_eq!(mutex.try_unwrap().unwrap(), vec![1, 2]);
}

// Acquire an uncontested Mutex with try_lock
#[test]
fn try_lock_uncontested() {
    let mutex = Mutex::<u32>::new(5);

    let guard = mutex.try_lock().unwrap();
    assert_eq!(5, *guard);
}

// Try and fail to acquire a contested Mutex with try_lock
#[test]
fn try_lock_contested() {
    let mutex = Mutex::<u32>::new(0);

    let _guard = mutex.try_lock().unwrap();
    assert!(mutex.try_lock().is_err());
}

#[test]
fn try_unwrap_multiply_referenced() {
    let mtx = Mutex::<u32>::new(0);
    let _mtx2 = mtx.clone();
    assert!(mtx.try_unwrap().is_err());
}

#[cfg(feature = "tokio")]
#[test]
fn with_err() {
    let mtx = Mutex::<i32>::new(-5);
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(lazy(|| {
        mtx.with(|guard| {
            if *guard > 0 {
                Ok(*guard)
            } else {
                Err("Whoops!")
            }
        }).unwrap()
    }));
    assert_eq!(r, Err("Whoops!"));
}

#[cfg(feature = "tokio")]
#[test]
fn with_ok() {
    let mtx = Mutex::<i32>::new(5);
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(lazy(move || {
        mtx.with(|guard| {
            Ok(*guard) as Result<i32, ()>
        }).unwrap()
    }));
    assert_eq!(r, Ok(5));
}

// Mutex::with should work with multithreaded Runtimes as well as
// single-threaded Runtimes.
// https://github.com/asomers/futures-locks/issues/5
#[cfg(feature = "tokio")]
#[test]
fn with_threadpool() {
    let mtx = Mutex::<i32>::new(5);
    let mut rt = runtime::Runtime::new().unwrap();
    let r = rt.block_on(lazy(move || {
        mtx.with(|guard| {
            Ok(*guard) as Result<i32, ()>
        }).unwrap()
    }));
    assert_eq!(r, Ok(5));
}

#[cfg(feature = "tokio")]
#[test]
fn with_local_ok() {
    // Note: Rc is not Send
    let mtx = Mutex::<Rc<i32>>::new(Rc::new(5));
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(lazy(move || {
        mtx.with_local(|guard| {
            Ok(**guard) as Result<i32, ()>
        }).unwrap()
    }));
    assert_eq!(r, Ok(5));
}
