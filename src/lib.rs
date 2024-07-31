use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::atomic::{AtomicBool, Ordering};

pub use SpinMutex as Mutex;
pub struct SpinMutex<T> {
    pub(crate) lock: AtomicBool,
    pub(crate) data: UnsafeCell<T>,
}

impl<'a, T> SpinMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            lock: AtomicBool::new(false),
        }
    }

    pub fn lock(&'a self) -> SpinMutexGuard<'a, T> {
        while !self
            .lock
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
            .is_ok_and(|v| v == true)
        {
            std::hint::spin_loop();
        }

        SpinMutexGuard::from(self)
    }
}

pub struct SpinMutexGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<'a, T> SpinMutexGuard<'a, T> {
    pub(crate) fn from(m: &'a SpinMutex<T>) -> Self {
        Self {
            lock: &m.lock,
            data: unsafe { &mut *m.data.get() },
        }
    }
}

impl<'a, T> Drop for SpinMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T> DerefMut for SpinMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

unsafe impl<T> Send for SpinMutex<T> {}
unsafe impl<T> Sync for SpinMutex<T> {}

unsafe impl<'a, T> Send for SpinMutexGuard<'a, T> {}
unsafe impl<'a, T> Sync for SpinMutexGuard<'a, T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;
    use std::sync::Arc;
    use std::thread::{sleep, spawn as thread_spawn, JoinHandle};
    use std::time::Duration;

    const SLEEP_TIME: Duration = Duration::from_millis(100);

    #[test]
    fn it_works() {
        let m = Arc::new(Mutex::new(0));
        let m2 = m.clone();

        // Could have just used a `[Option<JoinHandle<()>>; 2]`, or a std::vec::Vec, or initialized the array as `= [thread_spawn(), thread_spawn()]`, but I wanted to learn MaybeUninit
        let mut join_handles: [MaybeUninit<JoinHandle<()>>; 2] =
            [const { MaybeUninit::uninit() }; 2];

        join_handles[0] = MaybeUninit::new(thread_spawn(move || {
            sleep(SLEEP_TIME);
            let data = *m.lock();
            assert_eq!(5, data);
        }));

        join_handles[1] = MaybeUninit::new(thread_spawn(move || {
            let mut guard = m2.lock();
            *guard = 5;
        }));
    }
}
