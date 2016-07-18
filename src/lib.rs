#[macro_use]
extern crate lazy_static;
extern crate libc;

use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use std::mem;

struct CleanUpRecord {
    pub f: *const fn(*mut ()),
    pub ptr: *mut (),
}

unsafe impl Send for CleanUpRecord {}

lazy_static! {
    static ref CLEANUP_QUEUE :
Mutex<Vec<CleanUpRecord>> = Mutex::new(Vec::new());
}

pub struct Singleton<T: Send + Default> {
    ptr: AtomicPtr<Mutex<T>>,
}

unsafe impl<T> Send for Singleton<T> where T: Send + Default {}

impl<T> Default for Singleton<T>
    where T: Send + Default
{
    fn default() -> Self {
        Self::new::<T>()
    }
}

impl<T> Singleton<T>
    where T: Send + Default
{
    pub fn new<X: Send + Default>() -> Singleton<X> {
        Singleton::<X> { ptr: AtomicPtr::default() }
    }
    fn cleanup_fn<X: Send + Default>(ptr: *mut ()) {
        let ptr_mut: *mut Mutex<X> = unsafe { mem::transmute(ptr) };
        mem::drop(unsafe { Box::from_raw(ptr_mut) })
    }

    extern "C" fn cleanup_callback() {
        let guard = CLEANUP_QUEUE.lock().unwrap();
        for it in (*guard).iter() {
            let cleanup_fn: fn(*mut ()) =
                unsafe { mem::transmute(it.f.as_ref().unwrap()) };
            cleanup_fn(it.ptr);
        }
    }

    fn cleanup_register(record: CleanUpRecord) {
        let mut guard = CLEANUP_QUEUE.lock().unwrap();
        if (*guard).len() == 0 {
            unsafe {
                libc::atexit(Self::cleanup_callback);
            }
        }
        (*guard).push(record);
    }

    pub fn singleton(&'static self) -> &'static Mutex<T> {
        let null_ptr = ptr::null_mut() as *mut Mutex<T>;
        let invalid_ptr = 1 as *mut Mutex<T>;
        let mut raw_ptr = self.ptr.load(Ordering::SeqCst);
        while raw_ptr == null_ptr {
            if self.ptr
                .compare_exchange(null_ptr,
                                  invalid_ptr,
                                  Ordering::SeqCst,
                                  Ordering::Relaxed)
                .is_err() {
                raw_ptr = self.ptr.load(Ordering::SeqCst);
                continue;
            }

            // we're chosen!
            let ptr = Box::into_raw(Box::new(Mutex::new(T::default())));

            Self::cleanup_register(CleanUpRecord {
                f: Self::cleanup_fn::<T> as *const _,
                ptr: unsafe { mem::transmute(ptr) },
            });

            self.ptr.store(ptr, Ordering::SeqCst);
            raw_ptr = ptr;
        }

        while raw_ptr == invalid_ptr {
            raw_ptr = self.ptr.load(Ordering::SeqCst);
        }

        unsafe { raw_ptr.as_ref() }.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Singleton;

    struct A;
    impl Default for A {
        fn default() -> Self {
            A {}
        }
    }

    struct B;
    impl Default for B {
        fn default() -> Self {
            B {}
        }
    }

    lazy_static!{
        static ref SINGLETON_A : Singleton<A> = Singleton::<A>::new();
        static ref SINGLETON_B : Singleton<B> = Singleton::<B>::new();
    }

    #[test]
    fn it_works() {
        let _ = SINGLETON_A.singleton();
        let _ = SINGLETON_B.singleton();
    }
}
