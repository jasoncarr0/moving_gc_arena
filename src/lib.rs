/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![doc(html_root_url = "https://docs.rs/moving_gc_arena/0.1.1")]

use std::rc::Rc;
use std::rc;
use std::cell::Cell;

mod types;
pub use types::{Ix};
use types::{IxCell};

#[derive(Debug, PartialEq, Eq)]
#[allow(unused)]
pub enum Error {
    Indeterminable,
    IncorrectRegion,
    EntryExpired,
    UnexpectedInternalState,
}

use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Error::Indeterminable => "Invalid index",
            Error::IncorrectRegion => "Incorrect region for index",
            Error::EntryExpired => "Index expired",
            Error::UnexpectedInternalState => "Correct region has invalid internal state",
        })
    }

}
impl std::error::Error for Error { }

/**
 * A raw index for a region, that should be used for internal edges.
 * This index is invalidated by many operations, but locations which
 * have always been exposed by foreach_ix for each collection are
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
impl <T> Ix<T> {
    /**
     * If this crate has been compiled with support for validity checking,
     * this method will verify that an index is valid. In such cases,
     * a result of Ok indicates that this index points to a valid location
     * in the given region and has been updated.
     *
     * Otherwise, Ok will always be returned.
     */
    fn check_region(&self, _region: &Region<T>) -> Result<(), Error> {
        Ok(())
    }
    /**
     * Get the value pointd to by this index in its corresponding region.
     *
     * If the region is incorrect, the behavior of this function is
     * unspecified, and it may panic (but may also return a valid T reference).
     * Use try_get to avoid panics.
     */
    pub fn get<'a>(self, region: &'a Region<T>) -> &'a T {
        self.try_get(region).expect("Ix::get")
    }
    pub fn get_mut<'a>(self, region: &'a mut Region<T>) -> &'a mut T {
        self.try_get_mut(region).expect("Ix::get_mut")
    }
    pub fn try_get<'a>(self, region: &'a Region<T>) -> Result<&'a T, Error> {
        self.check_region(region)?;
        match region.data.get(self.ix())
        {
            Some(Spot::Present(e)) => Ok(&e.t),
            Some(Spot::BrokenHeart(_)) => Err(Error::Indeterminable),
            None => Err(Error::Indeterminable)
        }
    }
    pub fn try_get_mut<'a>(self, region: &'a mut Region<T>) -> Result<&'a mut T, Error> {
        self.check_region(region)?;
        match region.data.get_mut(self.ix())
        {
            Some(Spot::Present(e)) => Ok(&mut e.t),
            Some(Spot::BrokenHeart(_)) => Err(Error::Indeterminable),
            None => Err(Error::Indeterminable)
        }
    }
}
/*
 * Ex is a mutable index, which will receive updates
 * to the index as the source arena moves
 */
pub struct Ex<T> {
    cell: Rc<IxCell<T>>
}
impl <T> Clone for Ex<T> {
    fn clone(&self) -> Self {
        Ex {cell: self.cell.clone()}
    }
}
pub type Weak<T> = Ex<T>;
/**
 * A root is always a valid pointer into its corresponding region, regardless of
 * the presence of any garbage collections.
 */
pub type Root<T> = Ex<T>;
impl <T> std::fmt::Debug for Ex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.cell.get().fmt(f)
    }
}

impl <T> Ex<T> {
    /**
     * Gets the value at this location, when
     * passed the correct region. As with Ix,
     * the behavior when the region or location is
     * unspecified (but is still safe).
     */
    pub fn get<'a>(&self, r: &'a Region<T>) -> &'a T {
        self.try_get(r).unwrap()
    }
    pub fn get_mut<'a>(&self, r: &'a mut Region<T>) -> &'a mut T {
        self.try_get_mut(r).unwrap()
    }
    /**
     * Try to get a reference to this data, possibly returning an error.
     *
     * If the region is correct, then an error always indicates that the pointed-to
     * entry is no longer valid
     */
    pub fn try_get<'a>(&self, r: &'a Region<T>) -> Result<&'a T, Error> {
        self.ix().try_get(&r)
    }
    pub fn try_get_mut<'a>(&self, r: &'a mut Region<T>) -> Result<&'a mut T, Error> {
        self.ix().try_get_mut(r)
    }

    #[inline(always)]
    pub fn ix(&self) -> Ix<T> {
        self.cell.get()
    }
}

#[derive(Debug)]
struct Entry<T> {
    t: T,
    rc: rc::Weak<IxCell<T>>
}
impl <T> Entry<T> {
    //upgrade to an Ix, creating the cell if necessary
    fn weak(&mut self, ix: Ix<T>) -> Ex<T> {
        let cell =
            match self.rc.upgrade() {
                Some(rc) => rc,
                None => {
                    let rc = Rc::new(Cell::new(ix));
                    self.rc = Rc::downgrade(&rc);
                    rc
                }
            };
        Ex {
            cell
        }
    }

    pub fn get(&self) -> &T {
        return &self.t
    }
    pub fn get_mut(&mut self) -> &mut T {
        return &mut self.t
    }

    fn move_to(&mut self, other: Ix<T>) {
        if let Some(rc) = self.rc.upgrade() {
            rc.set(other)
        }
    }

    fn new(t: T) -> Self {
        Entry {
            t, rc: rc::Weak::new(),
        }
    }
}
#[derive(Debug)]
enum Spot<T> {
    Present(Entry<T>),
    BrokenHeart(Ix<T>),
}
impl <T> Spot<T> {
    fn get_entry_mut(&mut self) -> &mut Entry<T> {
        match self {
            Spot::Present(e) => e,
            _ => panic!("moving-gc-region internal error: Unexpected broken heart")
        }
    }

    #[allow(unused)]
    fn into_t(self) -> Option<T> {
        match self {
            Spot::Present(e) => Some(e.t),
            Spot::BrokenHeart(_) => None,
        }
    }
    // Change this into a broken heart to other,
    // updating the external reference
    #[allow(unused)]
    fn move_to(&mut self, other: Ix<T>) {
        if let Spot::Present(ref mut e) = self {
            e.move_to(other);
        }
        *self = Spot::BrokenHeart(other);
    }
}
pub struct MutEntry<'a, T> {
    ix: Ix<T>,
    entry: &'a mut Entry<T>,
    roots: &'a mut Vec<rc::Weak<IxCell<T>>>,
}
impl <'a, T> MutEntry<'a, T> {
    /**
     * Convert this borrowed entry into a permanent root.
     *
     * The root may be cloned after creation, but only one
     * entry can be created via this method.
     */
    pub fn to_root(mut self) -> Root<T> {
        let ex = self.weak();
        self.roots.push(Rc::downgrade(&ex.cell));
        ex
    }

    /**
     * Create a weak pointer, which can be used to access
     * a consistent location in the region, but does not
     * act as a root for garbage collection
     */
    pub fn weak(&mut self) -> Weak<T> {
        self.entry.weak(self.ix)
    }
    pub fn ix(&self) -> Ix<T> {
        self.ix
    }
    pub fn as_ref(&self) -> &T {
        self.entry.get()
    }
    pub fn as_mut_ref(&mut self) -> &mut T {
        self.entry.get_mut()
    }
}

pub struct Region<T> {
    data: Vec<Spot<T>>,
    roots: Vec<rc::Weak<IxCell<T>>>,
}
impl <T> Region<T> {
    pub fn new() -> Self {
        Region {
            data: Vec::new(),
            roots: Vec::new(),
        }
    }
}

pub trait HasIx<T : 'static> {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, f: F) where
        F: FnMut(&'b mut Ix<T>);
}
impl <'a, T: 'static + HasIx<T>> Region<T> {

    // Perform a gc into a new destination vector. For efficiency,
    // the vector must have enough capacity for the new elements
    fn prim_gc_to<'b : 'a>(src: &mut [Spot<T>], dst: &'b mut Vec<Spot<T>>,
                           roots: &mut Vec<rc::Weak<IxCell<T>>>) where
        T : HasIx<T>
    {

        // safety NOTE: Necessary for safety of this method,
        // since we need to avoid a particular invalidation later
        // This means that dst should never move for safety
        dst.reserve(src.len());
        let dst_spot_ptr = dst.as_ptr() as *mut Spot<T>;

        //NOTE: as a closure we're unable to mark
        //this as unsafe, but it is unsafe and should
        //always be called from an unsafe block
        let push_spot = |len: usize, s: &mut Spot<T>| {
            let new_index = Ix::new(len);

            if let Spot::Present(e) = s {
                e.move_to(new_index);
            };
            let obj = std::mem::replace(s, Spot::BrokenHeart(new_index));

            unsafe {
                let end = dst_spot_ptr.add(len);
                std::ptr::write(end, obj);
            }
            new_index
        };

        //Start searching at the vector length before any roots
        let mut obj_index = dst.len();

        //Push each root onto the destination, updating roots
        *roots = roots.drain(..).filter_map(|root| {
            let rc = root.upgrade()?;
            let s = src.get_mut(rc.get().ix())?;
            unsafe {
                rc.set(push_spot(dst.len(), s));
                dst.set_len(dst.len() + 1);
            }
            Some(root)
        }).collect();

        //Cheney copy starting at each of the roots
        while obj_index < dst.len() {
            let len = dst.len();
            let obj = dst.get_mut(obj_index).unwrap()
                .get_entry_mut().get_mut();
            let mut len_offset = 0;

            obj.foreach_ix( |pointed| {
                match src.get_mut(pointed.ix()) {
                    Some(s) => {
                        match s {
                            Spot::Present(_) => {
                                //safety requirement for push_spot
                                #[allow(unused)]
                                unsafe {
                                    *pointed = push_spot(len + len_offset, s);
                                }
                                len_offset += 1;
                            },
                            Spot::BrokenHeart(new_index) => {
                                *pointed = *new_index
                            }
                        }
                    },
                    None => {
                        panic!("Invalid index {} found from HasIx<T> at {} during GC.", pointed.ix(), obj_index);
                    }
                }
            });
            unsafe {
                dst.set_len(len + len_offset);
            }
            obj_index += 1;
        }
    }
    // Ensure that the capacity supports new_elems more
    // elements, collecting garbage if necessary
    pub fn ensure(&mut self, additional: usize) {
        let len = self.data.len();
        let cap = self.data.capacity();
        if cap >= len + additional { return }
        let mut dst = Vec::with_capacity(len + std::cmp::max(len, additional));
        Self::prim_gc_to(&mut self.data, &mut dst, &mut self.roots);
        self.roots = self.roots.drain(..).filter(|root| {root.upgrade().is_some()}).collect();
        self.data = dst;
    }

    /**
     * Allocate a new object, returning a handle.
     *
     * This may trigger a garbage collection. As such,
     * a function is used to generate the new value, which
     * can query the state of the world post-collection.
     */
    pub fn alloc<F>(&mut self, make_t: F) -> MutEntry<T> where
        F: FnOnce(&Self) -> T
    {
        //else the index could be incorrect
        self.ensure(1);
        let n = self.data.len();
        self.data.push(Spot::Present(Entry::new(make_t(&self))));
        MutEntry {
            ix: Ix::new(n),
            entry: self.data.get_mut(n).unwrap().get_entry_mut(),
            roots: &mut self.roots
        }
    }

    /**
     * Immediately trigger a standard garbage collection.
     *
     */
    pub fn gc(&mut self) {
        let mut dst = Vec::with_capacity(self.data.len());
        Self::prim_gc_to(&mut self.data, &mut dst, &mut self.roots);
        self.roots = self.take_valid_roots().collect();
        self.data = dst;
    }
    /**
     * Move the elements of this region onto the end of another Region.
     * This can trigger a collection in the other region if it
     * must be re-allocated.
     */
    pub fn gc_into(mut self, other: &mut Region<T>) {
        other.ensure(self.data.len());
        Self::prim_gc_to(&mut self.data, &mut other.data, &mut self.roots);
        other.roots.extend(self.take_valid_roots());
    }
    /**
     * Return the current capacity of this region. A collection won't
     * be triggered by allocation unless the desired amount exceeds the capacity.
     */
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    /**
     * Return the current number of entries in the region.
     */
    pub fn len(&self) -> usize {
        self.data.len()
    }
    fn take_valid_roots(&mut self) -> impl Iterator<Item=rc::Weak<IxCell<T>>> + '_ {
        self.roots.drain(..).filter(|root| {root.upgrade().is_some()})
    }
}
