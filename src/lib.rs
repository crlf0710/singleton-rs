#[macro_use]
mod singleton;

#[macro_use]
mod preemptive;

pub use singleton::Singleton;
pub use preemptive::PreemptiveSingleton;
