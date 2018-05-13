#[macro_use]
extern crate singleton;
use singleton::{PreemptiveSingleton, Singleton};
use std::cell::RefCell;

struct A(usize);
impl Default for A {
    fn default() -> Self {
        A(42)
    }
}

struct B(RefCell<usize>);
impl Default for B {
    fn default() -> Self {
        B(RefCell::new(100))
    }
}

static SINGLETON_A: Singleton<A> = make_singleton!();
static SINGLETON_B: PreemptiveSingleton<B> = make_preemptive_singleton!();

fn main() {
    assert!(SINGLETON_A.get_opt().is_none());
    assert!(SINGLETON_B.get_opt().is_none());
    let a1 = SINGLETON_A.get();
    assert!(!SINGLETON_A.get_opt().is_none());
    let a2 = SINGLETON_A.get();
    assert_eq!(a1 as *const _, a2 as *const _);
    let _b = SINGLETON_B.get();
    assert!(!SINGLETON_B.get_opt().is_none());
}
