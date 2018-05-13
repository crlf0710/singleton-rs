use super::singleton::Singleton;
use std::thread::{self, ThreadId};

#[doc(hidden)]
pub struct PreemptiveInner<T> {
    thread_id: ThreadId,
    data: T,
}

unsafe impl<T> Sync for PreemptiveInner<T> {}

/// A pointer type for holding non-shared global state in multi-thread environment.
/// Only the thread that sucessfully put data in it can access the data.
pub struct PreemptiveSingleton<T: Send> {
    #[doc(hidden)]
    pub singleton: Singleton<PreemptiveInner<T>>,
}

/// Create an uninitialized preemptive singleton.
///
/// This is intended as a workaround before const fn stablizes.
/// When const fn is stablized, you can just call PreemptiveInner::new().
#[macro_export]
macro_rules! make_preemptive_singleton {
    () => {
        $crate::PreemptiveSingleton {
            singleton: make_singleton!()
        }
    };
}

impl<T: Send> PreemptiveSingleton<T> {
    /// Create an uninitialized singleton.
    #[cfg(feature = "const_fn")]
    pub const fn new() -> Self {
        make_preemptive_singleton!()
    }

    /// Create an uninitialized singleton.
    #[cfg(not(feature = "const_fn"))]
    pub fn new() -> Self {
        make_preemptive_singleton!()
    }

    /// Access the singleton; initialize it with `Default::default()` if it is uninitialized.
    /// Panic if it is taken previously by another thread.
    ///
    pub fn get(&self) -> &T
    where
        T: Default,
    {
        self.get_or_insert_with(<T as Default>::default)
    }

    /// Access the singleton; or return `None` if it is not yet uninitialized or taken previously
    /// by another thread.
    pub fn get_opt(&self) -> Option<&T> {
        if let Some(pre_ref) = self.singleton.get_opt() {
            if pre_ref.thread_id == thread::current().id() {
                return Some(&pre_ref.data);
            }
        }
        return None;
    }
    /// Access the singleton; initialize it with custom function if it is uninitialized.
    /// Panic if it is taken previously by another thread.
    pub fn get_or_insert_with<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        let pre_ref = self.singleton.get_or_insert_with(move || PreemptiveInner {
            thread_id: thread::current().id(),
            data: f(),
        });

        if pre_ref.thread_id == thread::current().id() {
            return &pre_ref.data;
        }
        Self::error_occupied();
        unreachable!()
    }

    fn error_occupied() {
        // never type is not landing yet.
        panic!("singleton: trying to access an occupied preemptive singleton from another thread. ");
    }

    /// Put the singleton into a finalized state, destruct the singleton value if it is initialized.
    ///
    /// This is unsafe and only useful when the value holds other resources.
    pub unsafe fn finalize(&self) {
        if self.singleton.get_opt().is_some() {
            return self.singleton.finalize();
        }
    }
}

impl<T: Send> Drop for PreemptiveSingleton<T> {
    fn drop(&mut self) {
        unsafe {
            self.finalize();
        }
    }
}
