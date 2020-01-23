
#[allow(unused)]
mod unsafe_entry;
#[allow(unused)]
mod safe_entry;

#[cfg(not(feature="packed-headers"))]
pub use safe_entry::*;

#[cfg(feature="packed-headers")]
pub use unsafe_entry::*;
