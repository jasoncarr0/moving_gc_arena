
#[allow(unused)]
#[cfg(not(feature="packed-headers"))]
mod safe_entry;
#[allow(unused)]
#[cfg(feature="packed-headers")]
mod unsafe_entry;

#[cfg(not(feature="packed-headers"))]
#[allow(unused)]
pub use safe_entry::*;

#[cfg(feature="packed-headers")]
pub use unsafe_entry::*;
