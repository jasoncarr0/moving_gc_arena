/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![doc(html_root_url = "https://docs.rs/moving_gc_arena/0.2.1")]

use std::rc::Rc;
use std::rc;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};

mod types;
#[cfg(feature = "debug-arena")]
mod nonce;
mod entry;
mod has_ix;

pub use types::{Ix, Weak};
use types::{IxCell, SpotVariant};
use entry::{Entry, Spot};
pub use has_ix::HasIx;

#[derive(Debug, PartialEq, Eq)]
#[allow(unused)]
/**
 * Type of region access errors.
 */
pub enum Error {
    /**
     * Incorrect usage resulted in an error,
     * but the system does not have enough data
     * to determine exactly what the error was.
     *
     * Enabling the feature "debug-arena" will
     * allow the library to have appropriate data
     * in most cases, with high costs to space usage.
     */
    Indeterminable,
    /**
     * This index has been used with a region for
     * which it was not created.
     */
    IncorrectRegion,
    /**
     * This index has been invalidated by a garbage
     * collection.
     */
    EntryExpired,
    /**
     * This library is in an unexpected internal state.
     * It is not expected that any valid rust code
     * will be able receive this error, so encountering
     * it is likely a bug in the library.
     */
    // It can also occur if e.g. 2^64 collections occur
    // with debug-arena. That is of course still unexpected
    // and still requires an error to occur.
    UnexpectedInternalState,
}

use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Indeterminable => write!(f, "Invalid index"),
            Error::IncorrectRegion => write!(f, "Incorrect region for index"),
            Error::EntryExpired => write!(f, "Index expired"),
            Error::UnexpectedInternalState => write!(f, "Correct region has invalid internal state"),
        }
    }

}
impl std::error::Error for Error { }

impl <T> Ix<T> {
    /**
     * If this crate has been compiled with support for validity checking,
     * this method will verify that an index is valid. In such cases,
     * a result of Ok indicates that this index points to a valid location
     * in the given region and has been updated.
     *
     * Otherwise, Ok will always be returned.
     */
    #[inline]
    #[allow(unused)]
    pub fn check_region(self, region: &Region<T>) -> Result<(), Error> {
        #[cfg(feature = "debug-arena")]
        {
            if self.nonce != region.nonce {
                Err(Error::IncorrectRegion)?;
            } else if self.generation < region.generation {
                Err(Error::EntryExpired)?;
            } else if self.generation > region.generation {
                Err(Error::UnexpectedInternalState)?;
            }
        }
        Ok(())
    }
    /**
     * Get the value pointd to by this index in its corresponding region.
     *
     * If the region is incorrect, the behavior of this function is
     * unspecified, and it may panic (but may also return a valid T reference).
     * Use try_get to avoid panics.
     */
    #[inline]
    pub fn get<'a>(self, region: &'a Region<T>) -> &'a T {
        self.try_get(region).expect("Ix::get")
    }
    #[inline]
    pub fn get_mut<'a>(self, region: &'a mut Region<T>) -> &'a mut T {
        self.try_get_mut(region).expect("Ix::get_mut")
    }
    #[inline]
    pub fn try_get<'a>(self, region: &'a Region<T>) -> Result<&'a T, Error> {
        self.check_region(region)?;
        Ok(region.data.get(self.ix())
            .ok_or(Error::Indeterminable)?
            .get()
            .ok_or(Error::Indeterminable)?
            .get())
    }
    #[inline]
    pub fn try_get_mut<'a>(self, region: &'a mut Region<T>) -> Result<&'a mut T, Error> {
        self.check_region(region)?;
        Ok(region.data.get_mut(self.ix())
            .ok_or(Error::Indeterminable)?
            .get_mut()
            .ok_or(Error::Indeterminable)?
            .get_mut())
    }
}

/**
 * A freshly created entry, allowing root/weak creation, and mutation
 *
 * This entry is created by calls to [`Region::alloc`](struct.Region.html#method.alloc)
 * and will allow the creation of external and internal indices,
 * as well as allowing access to the freshly-created object.
 */
pub struct MutEntry<'a, T> {
    ix: Ix<T>,
    entry: &'a mut Entry<T>,
    root: rc::Weak<IxCell<T>>,
    roots: &'a mut Vec<rc::Weak<IxCell<T>>>,
}

/**
 * An external rooted index into a region.
 * 
 * Roots will always keep the objects they
 * point to live in the appropriate region.
 *
 * Roots should generally not be used within a region,
 * instead use [`Ix`](struct.Ix.html).
 * A root that is inside its own region will never
 * be collected and is vulnerable to the same issues
 * as Rc. Similarly, roots between two different regions
 * may cause uncollectable reference cycles.
 */
pub struct Root<T> {
    cell: Rc<IxCell<T>>
}
impl <T> Clone for Root<T> {
    fn clone(&self) -> Self {
        Root {cell: self.cell.clone()}
    }
}
impl <T> Debug for Root<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.cell.get().fmt(f)
    }
}

impl <T> Weak<T> {
    /**
     * Gets the value at this location, when
     * passed the correct region. As with Ix,
     * the behavior when the region or location is
     * unspecified (but is still safe).
     */
    #[inline]
    pub fn get<'a>(&self, r: &'a Region<T>) -> &'a T {
        self.try_get(r).unwrap()
    }
    #[inline]
    pub fn get_mut<'a>(&self, r: &'a mut Region<T>) -> &'a mut T {
        self.try_get_mut(r).unwrap()
    }
    /**
     * Try to get a reference to this data, possibly returning an error.
     *
     * If the region is correct, then an error always indicates that the pointed-to
     * entry is no longer valid
     */
    #[inline]
    pub fn try_get<'a>(&self, r: &'a Region<T>) -> Result<&'a T, Error> {
        match self.ix() {
            Some(i) => i.try_get(r),
            None => Err(Error::EntryExpired)
        }
    }
    #[inline]
    pub fn try_get_mut<'a>(&self, r: &'a mut Region<T>) -> Result<&'a mut T, Error> {
        match self.ix() {
            Some(i) => i.try_get_mut(r),
            None => Err(Error::EntryExpired)
        }
    }
}


/**
 * A root is always a valid pointer into its corresponding region, regardless of
 * the presence of any garbage collections.
 */
impl <T> Root<T> {
    /**
     * Gets the value at this location, when
     * passed the correct region. As with Ix,
     * the behavior when the region or location is
     * unspecified (but is still safe).
     */
    #[inline]
    pub fn get<'a>(&self, r: &'a Region<T>) -> &'a T {
        self.try_get(r).unwrap()
    }
    #[inline]
    pub fn get_mut<'a>(&self, r: &'a mut Region<T>) -> &'a mut T {
        self.try_get_mut(r).unwrap()
    }
    /**
     * Try to get a reference to this data, possibly returning an error.
     *
     * If the region is correct, then an error always indicates that the pointed-to
     * entry is no longer valid
     */
    #[inline]
    pub fn try_get<'a>(&self, r: &'a Region<T>) -> Result<&'a T, Error> {
        self.ix().try_get(&r)
    }
    #[inline]
    pub fn try_get_mut<'a>(&self, r: &'a mut Region<T>) -> Result<&'a mut T, Error> {
        self.ix().try_get_mut(r)
    }

    /**
     * Get the raw index pointed to this by external index.
     * All validity caveats of indices apply, so this should
     * most likely be used only to move into a location
     * that is owned by an element of the Region
     */
    #[inline(always)]
    pub fn ix(&self) -> Ix<T> {
        self.cell.get()
    }
}

impl <'a, T> MutEntry<'a, T> {
    /**
     * Create a root pointer, which will keep this object
     * live across garbage collections.
     */
    pub fn root(&mut self) -> Root<T> {
        let i = self.ix;
        match self.root.upgrade() {
            None => {
                let rc = Rc::new(Cell::new(i));
                self.roots.push(Rc::downgrade(&rc));
                self.root = Rc::downgrade(&rc);
                Root { cell: rc }
            },
            Some(cell) => Root { cell }
        }
    }

    /**
     * Create a weak pointer, which can be used to access
     * a consistent location in the region, but does not
     * act as a root for garbage collection
     */
    #[inline]
    pub fn weak(&mut self) -> Weak<T> {
        self.entry.weak(self.ix)
    }
    #[inline]
    pub fn ix(&self) -> Ix<T> {
        self.ix
    }
    #[inline]
    #[deprecated(since="0.2.0", note="Please use MutEntry::get")]
    pub fn as_ref(&self) -> &T {
        self.entry.get()
    }
    #[inline]
    #[deprecated(since="0.2.0", note="Please use MutEntry::get_mut")]
    pub fn as_mut_ref(&mut self) -> &mut T {
        self.entry.get_mut()
    }
    #[inline]
    pub fn get(&self) -> &T {
        self.entry.get()
    }
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.entry.get_mut()
    }
}

/**
 * The type of a collectable region.
 *
 * This object can be used to allocate, collect,
 * traverse and update the objects within it.
 *
 * Access to the region is exposed through methods
 * on the corresponding reference types, and requires
 * references to this region in order to safely
 * reference the data within. This ensures that
 * garbage collections do not interrupt accesses and
 * vice versa, and allows for a conservative compile-time check for uniqueness, rather than
 * requiring use of an internal Cell type.
 *
 * Since garbage collection is a property of the region, it is not statically checked for indices.
 * Weak and Root will always be in sync with their
 * source region, but raw indices Ix may be invalidated.
 * Some methods (which necessarily take &mut self) may invalidate raw indices by moving the
 * objects, such as for a garbage collection.
 * These will be documented.
 *
 */
pub struct Region<T> {
    data: Vec<Spot<T>>,
    roots: Vec<rc::Weak<IxCell<T>>>,

    #[cfg(feature = "debug-arena")]
    nonce: u64,
    #[cfg(feature = "debug-arena")]
    generation: u64,
}

impl <T> Region<T> {

    #[inline]
    pub fn new() -> Self {
        Region {
            data: Vec::new(),
            roots: Vec::new(),
            #[cfg(feature = "debug-arena")]
            nonce: nonce::next(),
            #[cfg(feature = "debug-arena")]
            generation: 0,
        }
    }
}
impl <T> Default for Region<T> {
    fn default() -> Self {
        Self::new()
    }
}


impl <'a, T: 'static + HasIx<T>> Region<T> {



    // Perform a gc into a new destination vector. For efficiency,
    // the vector must have enough capacity for the new elements
    fn prim_gc_to<'b : 'a>(src: &mut [Spot<T>], dst: &'b mut Vec<Spot<T>>,
                           roots: &mut Vec<rc::Weak<IxCell<T>>>,
                           #[cfg(feature = "debug-arena")] old_gen: (u64, u64),
                           #[cfg(feature = "debug-arena")] new_gen: (u64, u64),
                           )
    where
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
            let new_index = Ix::new(len,
                #[cfg(feature = "debug-arena")]
                new_gen.0,
                #[cfg(feature = "debug-arena")]
                new_gen.1,
            );

            let obj = s.move_to(new_index);

            unsafe {
                let end = dst_spot_ptr.add(len);
                std::ptr::write(end, obj);
            }
            new_index
        };

        //Start searching at the vector length before any roots
        let mut obj_index = dst.len();

        #[cfg(feature = "debug-arena")]
        let check_gen = |ix: Ix<T>, internal: bool| {
            {
                let prefix = if internal {"GC internal error (root)"} else {"GC"};
                if ix.nonce != old_gen.0 {
                    if ix.nonce == new_gen.0 {
                        panic!("{}: Index processed twice", prefix);
                    } else {
                        panic!("{}: Invalid source index for root", prefix);
                    }
                } else if ix.generation < old_gen.1 {
                    panic!("{}: Index is for a generation that is too old, it may have missed processing", prefix);
                } else if ix.generation > old_gen.1 {
                    panic!("{}: Index is for a generation that is too new, it may have been processed twice", prefix);
                }
            }
        };


        //Push each root onto the destination, updating roots
        *roots = roots.drain(..).filter_map(|root| {
            let rc = root.upgrade()?;
            let ix = rc.get();
            #[cfg(feature = "debug-arena")]
            check_gen(ix, true);

            let s = src.get_mut(ix.ix())?;
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
                .get_mut().unwrap().get_mut();
            let mut len_offset = 0;

            // NOTE for safety:
            // foreach_ix can panic,
            // therefore, length should never
            // be set until a valid object is in the location
            obj.foreach_ix( |pointed| {
                #[cfg(feature = "debug-arena")]
                check_gen(*pointed, false);

                match src.get_mut(pointed.ix()) {
                    Some(s) => {
                        match s.variant() {
                            SpotVariant::Present(_) => {
                                //safety requirement for push_spot
                                #[allow(unused)]
                                unsafe {
                                    *pointed = push_spot(len + len_offset, s);
                                }
                                len_offset += 1;
                            },
                            SpotVariant::BrokenHeart(new_index) => {
                                *pointed = new_index
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

    /**
     * Ensure that the capacity supports new_elems more
     * elements, collecting garbage if necessary.
     */
    pub fn ensure(&mut self, additional: usize) {
        let len = self.data.len();
        let cap = self.data.capacity();
        if cap >= len + additional { return }
        let mut dst = Vec::with_capacity(len + std::cmp::max(len, additional));

        #[cfg(feature = "debug-arena")]
        let new_gen = (self.nonce, self.generation+1);

        Self::prim_gc_to(&mut self.data, &mut dst, &mut self.roots,
            #[cfg(feature = "debug-arena")]
            (self.nonce, self.generation),
            #[cfg(feature = "debug-arena")]
            new_gen);
        self.roots = self.roots.drain(..).filter(|root| {root.upgrade().is_some()}).collect();
        self.data = dst;

        #[cfg(feature = "debug-arena")]
        {
            self.generation = new_gen.1;
        }
    }

    /**
     * Allocate a new object, returning a temporary handle,
     * which can be used to mutate the object and to get
     * roots, weak pointers, and internal pointers to
     * the object.
     *
     * This may trigger a garbage collection and invalidate
     * raw indices.  As such, a function is used to
     * generate the new value, which
     * can query the state of the world post-collection.
     */
    pub fn alloc<F>(&mut self, make_t: F) -> MutEntry<T> where
        F: FnOnce(&Self) -> T
    {
        //else the index could be incorrect
        self.ensure(1);
        let n = self.data.len();
        self.data.push(Spot::new(make_t(&self)));
        MutEntry {
            ix: Ix::new(n,
                #[cfg(feature = "debug-arena")]
                self.nonce,
                #[cfg(feature = "debug-arena")]
                self.generation,
                ),
            entry: self.data.get_mut(n).unwrap().get_mut().unwrap(),
            root: rc::Weak::new(),
            roots: &mut self.roots
        }
    }

    /**
     * Immediately trigger a standard garbage collection.
     *
     * This invalidates raw indices.
     *
     * ```rust
     * use moving_gc_arena as gc;
     * let mut r = gc::Region::new();
     *
     * r.alloc(|_|{()});
     * r.gc();
     * assert!(r.is_empty());
     * ```
     */
    pub fn gc(&mut self) {
        let mut dst = Vec::with_capacity(self.data.len());
        Self::prim_gc_to(&mut self.data, &mut dst, &mut self.roots,
            #[cfg(feature = "debug-arena")]
            (self.nonce, self.generation),
            #[cfg(feature = "debug-arena")]
            (self.nonce, self.generation+1));
        self.roots = self.take_valid_roots().collect();
        self.data = dst;
        #[cfg(feature = "debug-arena")]
        {
            self.generation = self.generation+1;
        }
    }
    /**
     * Move the elements of this region onto the end of another Region.
     * This can trigger a collection in the other region if it
     * must be re-allocated.
     */
    pub fn gc_into(mut self, other: &mut Region<T>) {
        other.ensure(self.data.len());
        Self::prim_gc_to(&mut self.data, &mut other.data, &mut self.roots,
            #[cfg(feature = "debug-arena")]
            (self.nonce, self.generation),
            #[cfg(feature = "debug-arena")]
            (other.nonce, other.generation));
        other.roots.extend(self.take_valid_roots());
    }
    /**
     * Return the current capacity of this region. A collection won't
     * be triggered by allocation unless the desired amount exceeds the capacity.
     */
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    /**
     * Return the current number of entries in the region.
     */
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /**
     * Returns true if there are currently no entries in this region.
     */
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    fn take_valid_roots(&mut self) -> impl Iterator<Item=rc::Weak<IxCell<T>>> + '_ {
        self.roots.drain(..).filter(|root| {root.upgrade().is_some()})
    }
}


#[cfg(test)]
mod tests {
    use super::{Ix, Region, HasIx};

    #[derive(Debug)]
    struct Elem {
        ix: Option<Ix<Elem>>,
    }
    impl Elem {
        pub fn new() -> Self {
            Elem { ix: None }
        }
    }
    impl HasIx<Elem> for Elem {
        fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, f: F) where
            F: FnMut(&'b mut Ix<Elem>)
        {
            self.ix.iter_mut().for_each(f)
        }
    }

    #[test]
    pub fn weaks_are_weak() {
        let mut r = Region::new();
        let w1 = r.alloc(|_| {Elem::new()}).weak();

        let mut e2 = r.alloc(|_| {Elem::new()});
        let w2 = e2.weak();
        let r2 = e2.root();

        r.gc();
        let w3 = r.alloc(|_| {Elem::new()}).weak();

        // first is collected by now
        assert!(w1.try_get(&r).is_err());

        // root and new version are both accessible
        assert!(w2.try_get(&r).is_ok());
        assert!(w3.try_get(&r).is_ok());

        // touch r
        drop(r2);
    }

    #[test]
    pub fn roots_are_root() {
        let mut r = Region::new();
        let mut e1 = r.alloc(|_| {Elem::new()});
        let w1 = e1.weak();
        let r1 = e1.root();
        let r2 = r.alloc(|_| {Elem::new()}).root();
        std::mem::drop(r1);
        r.gc();

        //r1 should have stopped being root on drop
        assert!(w1.try_get(&r).is_err());

        //r2 is still a root
        assert!(r2.try_get(&r).is_ok());
    }

    #[test]
    pub fn indirect_correct() {
        let mut r = Region::new();

        let mut e1 = r.alloc(|_| {Elem::new()});
        let w1 = e1.weak();
        let r1 = e1.root();
        let r2 = r.alloc(|_| {Elem {ix: Some(r1.ix())}}).root();
        std::mem::drop(r1);

        let mut e3 = r.alloc(|_| {Elem::new()});
        e3.get_mut().ix = Some(e3.ix());
        let w3 = e3.weak();

        let r4 = r.alloc(|_| {Elem::new()}).root();
        let w5 = r.alloc(|_| {Elem::new()}).weak();

        r4.get_mut(&mut r).ix = Some(w5.ix().unwrap());
        w5.get_mut(&mut r).ix = Some(r4.ix());

        //nothing changed with r4 and w5 during access
        assert!(r4.try_get(&r).is_ok());
        assert!(w5.try_get(&r).is_ok());
        std::mem::drop(r4);

        r.gc();

        //entries 1 and 2 are still good.
        assert!(match w1.try_get(&r) {
            Ok(Elem { ix: None }) => true,
            x => panic!("{:?}", x),
        });
        assert!(match r2.try_get(&r) {
            Ok(Elem { ix: Some(_) }) => true,
            x => panic!("{:?}", x),
        });

        // entries 3, 4 and 5 should be collected
        // despite cycles
        assert!(w3.try_get(&r).is_err());
        assert!(w5.try_get(&r).is_err());

    }


}
