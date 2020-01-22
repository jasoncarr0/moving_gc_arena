
//#[cfg(not (feature = "header_safety"))]
//mod safe_entry;

//#[cfg(not (feature = "header_safety"))]
//pub use safe_entry::*;

//#[cfg(feature = "header_safety")]
//#[cfg(feature = "header_safety")]
#[allow(unused)]
mod unsafe_entry;
#[allow(unused)]
mod safe_entry;
pub use safe_entry::*;
