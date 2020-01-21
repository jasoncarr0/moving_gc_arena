
// Unsafe header. May be smaller and more performant,
//


use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::rc;
use std::cell::Cell;
use std::mem::MaybeUninit;

use crate::types::{Ix, IxCell};

union Header<T> {
    broken_heart: *const Spot<T>,
}

pub(crate) struct Spot<T> {
    header: Header<T>,
    value: MaybeUninit<T>,
}
//
pub(crate) struct Entry<T> {
    spot: Spot<T>
}

