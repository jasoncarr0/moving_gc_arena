/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
use std::marker::PhantomData;

pub struct Ix<T> {
    ix: usize,
    _t: PhantomData<*mut T>,
    #[cfg(feature = "debug-arena")]
    pub(crate) nonce: u64,
    #[cfg(feature = "debug-arena")]
    pub(crate) generation: u64,
}
use std::fmt;
impl <T> std::fmt::Debug for Ix<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ix.fmt(f)
    }
}
impl <T> Clone for Ix<T> {
    fn clone(&self) -> Self {
        Ix {
            ix: self.ix,
            _t: PhantomData,
            #[cfg(feature = "debug-arena")]
            nonce: self.nonce,
            #[cfg(feature = "debug-arena")]
            generation: self.generation,
        }
    }
}
impl <T> Copy for Ix<T> {}
unsafe impl <T> Send for Ix<T> {}
unsafe impl <T> Sync for Ix<T> {}


impl <T> Ix<T> {
    pub(crate) fn new(ix: usize,
                      #[cfg(feature = "debug-arena")]
                      nonce: u64,
                      #[cfg(feature = "debug-arena")]
                      generation: u64,
    ) -> Self {
        Ix { ix: ix, _t: PhantomData,
            #[cfg(feature = "debug-arena")]
            nonce,
            #[cfg(feature = "debug-arena")]
            generation, }
    }

    #[inline(always)]
    pub(crate) fn ix(&self) -> usize {self.ix}
}
pub type IxCell<T> = Cell<Ix<T>>;
