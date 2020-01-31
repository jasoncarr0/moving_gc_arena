/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use alloc::rc;
use core::cell::Cell;
use core::marker::PhantomData;

#[repr(C)]
// repr(C) Needed for unsafe header
// Note that Ix<T> can pack any set of
// bits whatsoever
/**
 * A raw index for a region, that should be used for internal edges.
 * 
 * This index is invalidated by many operations. but locations which
 * have always been exposed exactly once by foreach_ix for each collection are
 * guaranteed to have an index which is valid.
 *
 * Furthermore, indices received from a MutEntry or Root/Weak are
 * valid when retrieved.
 *
 * An Ix is valid so long as no invalidating methods have been called.
 * Borrowing rules ensure several situations in which no invalidating method can be called:
 *  - An immutable reference to the region exists
 *  - A mutable or immutable reference to any element of this region exists, such as those
 *    acquired via Ix::get.
 *  - A MutEntry for this region exists.
 *
 * If an Ix is not valid for the given region, behavior is unspecified but safe,
 * A valid instance of T may be returned. Panics may occur with get and get_mut.
 * If the index is valid, then it still points to the expected object.
 */
pub struct Ix<T> {
    ix: usize,
    _t: PhantomData<*mut T>,
    #[cfg(feature = "debug-arena")]
    pub(crate) nonce: u64,
    #[cfg(feature = "debug-arena")]
    pub(crate) generation: u64,
}
use core::fmt;
impl <T> fmt::Debug for Ix<T> {
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
        Ix { ix, _t: PhantomData,
            #[cfg(feature = "debug-arena")]
            nonce,
            #[cfg(feature = "debug-arena")]
            generation, }
    }

    #[inline(always)]
    pub(crate) fn ix(self) -> usize {self.ix}

    /**
     * Get an identifier for this index.
     * It is unique amongst indices in this region,
     * so long as they have not been invalidated.
     *
     * Like the index itself, uniqueness is only
     * guaranteed as long as the index has not been
     * invalidated.
     */
    #[inline(always)]
    pub fn identifier(self) -> usize {self.ix}
}
pub type IxCell<T> = Cell<Ix<T>>;

pub enum SpotVariant<'a, E, T> {
    Present(&'a mut E),
    BrokenHeart(Ix<T>),
}

/**
 * A weak index into a region.
 *
 * This index will never prevent an
 * object from being collected, but
 * can be used to test if an object
 * has been collected, or access
 * it as normal.
 */
pub struct Weak<T> {
    pub(crate) cell: rc::Weak<IxCell<T>>
}
