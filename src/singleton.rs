use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr::null_mut;
use std::mem::drop;

#[repr(usize)]
enum SingletonState {
    Initial = 0,
    Loading = 1,
    Ready = 2,
    Finalized = 3,
}

/// A pointer type for holding shared global state in multi-thread environment.
pub struct Singleton<T: Send + Sync> {
    #[doc(hidden)]
    pub state: AtomicUsize,
    #[doc(hidden)]
    pub ptr: AtomicPtr<T>,
}

/// Create an uninitialized singleton.
///
/// This is intended as a workaround before const fn stablizes.
/// When const fn is stablized, you can just call Singleton::new().
#[macro_export]
macro_rules! make_singleton {
    () => {
        Singleton {
            state: ::std::sync::atomic::AtomicUsize::new(0),
            ptr: ::std::sync::atomic::AtomicPtr::new(::std::ptr::null_mut())
        }
    };
}

impl<T: Send + Sync> Default for Singleton<T> {
    fn default() -> Self {
        make_singleton!()
    }
}

impl<T: Send + Sync> Singleton<T> {
    /// Create an uninitialized singleton.
    #[cfg(feature = "const_fn")]
    pub const fn new() -> Self {
        make_singleton!()
    }

    /// Create an uninitialized singleton.
    #[cfg(not(feature = "const_fn"))]
    pub fn new() -> Self {
        make_singleton!()
    }

    /// Access the singleton; initialize it with `Default::default()` if it is uninitialized.
    ///
    pub fn get(&self) -> &T
    where
        T: Default,
    {
        self.get_or_insert_with(<T as Default>::default)
    }

    /// Access the singleton; or return `None` if it is not yet uninitialized.
    pub fn get_opt(&self) -> Option<&T> {
        unsafe { self.ptr.load(Ordering::SeqCst).as_ref() }
    }

    fn error_stateshift() {
        // never type is not landing yet.
        panic!("singleton: state shifted during singleton initialization. Maybe caused by unsafe finalized() calling. ");
    }

    fn error_finalized() {
        // never type is not landing yet.
        panic!("singleton: trying to access a finalized singleton. Maybe caused by unsafe finalized() calling. ");
    }

    /// Access the singleton; initialize it with custom function if it is uninitialized.
    pub fn get_or_insert_with<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(v) = unsafe { self.ptr.load(Ordering::SeqCst).as_ref() } {
            return v;
        }

        let mut cur_state = self.state.compare_and_swap(
            SingletonState::Initial as _,
            SingletonState::Loading as _,
            Ordering::SeqCst,
        );
        'spin: loop {
            if cur_state == SingletonState::Loading as _ {
                // some other threading is trying to initialize this singleton.
                // wait and retry.
                cur_state = self.state.load(Ordering::SeqCst);
                continue 'spin;
            } else if cur_state == SingletonState::Initial as _
                || cur_state == SingletonState::Ready as _
            {
                if cur_state == SingletonState::Initial as _ {
                    let v = Box::into_raw(Box::new(f()));
                    self.ptr.store(v, Ordering::SeqCst);
                    cur_state = self.state.compare_and_swap(
                        SingletonState::Loading as _,
                        SingletonState::Ready as _,
                        Ordering::SeqCst,
                    );

                    if cur_state != SingletonState::Loading as _ {
                        Self::error_stateshift();
                        unreachable!();
                    }
                }

                if let Some(v) = unsafe { self.ptr.load(Ordering::SeqCst).as_ref() } {
                    return v;
                } else {
                    Self::error_stateshift();
                    unreachable!();
                }
            }

            Self::error_finalized();
            unreachable!();
        }
        // unreachable!()
    }

    /// Put the singleton into a finalized state, destruct the singleton value if it is initialized.
    ///
    /// This is unsafe and only useful when the value holds other resources.
    pub unsafe fn finalize(&self) {
        self.state
            .store(SingletonState::Finalized as _, Ordering::SeqCst);
        let old_ptr = self.ptr.swap(null_mut(), Ordering::SeqCst);
        if old_ptr.is_null() {
            return;
        }
        drop(Box::from_raw(old_ptr));
    }
}

impl<T: Send + Sync> Drop for Singleton<T> {
    fn drop(&mut self) {
        unsafe {
            self.finalize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Singleton;

    struct A(usize);
    impl Default for A {
        fn default() -> Self {
            A(42)
        }
    }

    struct B(usize);
    impl Default for B {
        fn default() -> Self {
            B(100)
        }
    }

    static SINGLETON_A: Singleton<A> = make_singleton!();
    static SINGLETON_B: Singleton<B> = make_singleton!();

    #[test]
    fn it_works() {
        assert!(SINGLETON_A.get_opt().is_none());
        assert!(SINGLETON_B.get_opt().is_none());
        let a1 = SINGLETON_A.get();
        assert!(!SINGLETON_A.get_opt().is_none());
        let a2 = SINGLETON_A.get();
        assert_eq!(a1 as *const _, a2 as *const _);
        let _b = SINGLETON_B.get();
        assert!(!SINGLETON_B.get_opt().is_none());
    }
}
