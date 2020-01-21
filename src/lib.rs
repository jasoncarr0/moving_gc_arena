/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![doc(html_root_url = "https://docs.rs/moving_gc_arena/0.2.0")]

use std::rc::Rc;
use std::rc;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};

mod types;
#[cfg(feature = "debug-arena")]
mod nonce;
mod inject;

pub use types::{Ix};
use types::{IxCell};
use inject::InjectInto;

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
        match self {
            Error::Indeterminable => write!(f, "Invalid index"),
            Error::IncorrectRegion => write!(f, "Incorrect region for index"),
            Error::EntryExpired => write!(f, "Index expired"),
            Error::UnexpectedInternalState => write!(f, "Correct region has invalid internal state"),
        }
    }

}
impl std::error::Error for Error { }

/**
 * A raw index for a region, that should be used for internal edges.
 * This index is invalidated by many operations, but locations which
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
    pub fn check_region<R>(&self, region: &Region<R>) -> Result<(), Error> where
        T: InjectInto<R>
    {
        #[cfg(feature = "debug-arena")]
        {
            if self.nonce != region.nonce {
                Err(Error::IncorrectRegion)?;
            } else if self.generation < region.generation {
                Err(Error::EntryExpired)?;
            } else if self.generation > region.generation {
                Err(Error::Indeterminable)?;
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
    pub fn get<'a, R>(self, region: &'a Region<R>) -> &'a T where
        T: InjectInto<R>
    {
        self.try_get(region).expect("Ix::get")
    }
    pub fn get_mut<'a, R>(self, region: &'a mut Region<R>) -> &'a mut T where
        T: InjectInto<R>
    {
        self.try_get_mut(region).expect("Ix::get_mut")
    }
    pub fn try_get<'a, R>(self, region: &'a Region<R>) -> Result<&'a T, Error> where
        T: InjectInto<R>
    {
        self.check_region(region)?;
        match region.data.get(self.ix())
        {
            Some(Spot::Present(e)) => InjectInto::project_ref(&e.t).ok_or(Error::Indeterminable),
            Some(Spot::BrokenHeart(_)) => Err(Error::Indeterminable),
            None => Err(Error::Indeterminable)
        }
    }
    pub fn try_get_mut<'a, R>(self, region: &'a mut Region<R>) -> Result<&'a mut T, Error> where
        T: InjectInto<R>
    {
        self.check_region(region)?;
        match region.data.get_mut(self.ix())
        {
            Some(Spot::Present(e)) => InjectInto::project_mut(&mut e.t).ok_or(Error::Indeterminable),
            Some(Spot::BrokenHeart(_)) => Err(Error::Indeterminable),
            None => Err(Error::Indeterminable)
        }
    }
}
/**
 * Ex is a mutable index, which will receive updates
 * to the index as the source arena moves
 */

pub struct Weak<T> {
    cell: rc::Weak<IxCell<T>>
}
impl <T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        Weak {cell: self.cell.clone()}
    }
}
impl <T> Debug for Weak<T> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.cell.upgrade().fmt(f)
    }
}
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
    pub fn get<'a, R>(&self, r: &'a Region<R>) -> &'a T where
        T: InjectInto<R>
    {
        self.try_get(r).unwrap()
    }
    pub fn get_mut<'a, R>(&self, r: &'a mut Region<R>) -> &'a mut T where
        T: InjectInto<R>
    {
        self.try_get_mut(r).unwrap()
    }
    /**
     * Try to get a reference to this data, possibly returning an error.
     *
     * If the region is correct, then an error always indicates that the pointed-to
     * entry is no longer valid
     */
    pub fn try_get<'a, R>(&self, r: &'a Region<R>) -> Result<&'a T, Error> where
        T: InjectInto<R>
    {
        match self.ix() {
            Some(i) => i.try_get(r),
            None => Err(Error::EntryExpired)
        }
    }
    pub fn try_get_mut<'a, R>(&self, r: &'a mut Region<R>) -> Result<&'a mut T, Error> where
        T: InjectInto<R>
    {
        match self.ix() {
            Some(i) => i.try_get_mut(r),
            None => Err(Error::EntryExpired)
        }
    }

    /**
     * Get the raw index pointed to this by external index.
     * All validity caveats of indices apply, so this should
     * most likely be used only to move into a location
     * that is owned by an element of the Region
     */
    #[inline(always)]
    pub fn ix(&self) -> Option<Ix<T>> {
        Some(self.cell.upgrade()?.get())
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

#[derive(Debug)]
struct Entry<T> {
    // We'll always keep an RC live here so that
    // the weak pointers can use upgrade() to check.
    // At GC time, we clear if weak_count is 0
    rc: Option<Rc<IxCell<T>>>,
    t: T,
}
impl <T> Entry<T> {
    //upgrade to an Ix, creating the cell if necessary
    fn weak(&mut self, ix: Ix<T>) -> Weak<T> {
        let cell = Rc::downgrade(
            &match self.rc {
                Some(ref rc) => rc.clone(),
                None => {
                    let rc = Rc::new(Cell::new(ix));
                    self.rc = Some(rc.clone());
                    rc
                }
            });
        Weak {
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
        if let Some(ref mut rc) = self.rc {
            rc.set(other)
        }
    }

    fn check_clear_rc(&mut self) {
        match self.rc {
            Some(ref mut rc) =>
                if 0 == Rc::weak_count(rc) {
                    self.rc = None;
                },
            None => (),
        }
    }

    fn new(t: T) -> Self {
        Entry {
            t, rc: None,
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
pub struct MutEntry<'a, S, T> {
    ix: Ix<S>,
    entry: &'a mut Entry<T>,
    root: rc::Weak<IxCell<T>>,
    roots: &'a mut Vec<rc::Weak<IxCell<T>>>,
}
impl <'a, S, T> MutEntry<'a, S, T> {
    /**
     * Create a root pointer, which will keep this object
     * live across garbage collections.
     */
    pub fn root(&mut self) -> Root<S> {
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
    pub fn weak(&mut self) -> Weak<S> {
        self.entry.weak(self.ix)
    }
    pub fn ix(&self) -> Ix<S> {
        self.ix
    }
    pub fn as_ref(&self) -> &S {
        InjectInto::project_ref(self.entry.get())
    }
    pub fn as_mut_ref(&mut self) -> &mut S {
        InjectInto::project_mut(self.entry.get_mut())
    }
}

pub struct Region<T> {
    data: Vec<Spot<T>>,
    roots: Vec<rc::Weak<IxCell<T>>>,

    #[cfg(feature = "debug-arena")]
    nonce: u64,
    #[cfg(feature = "debug-arena")]
    generation: u64,
}

impl <T> Region<T> {


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

pub trait HasIx<T : 'static> {
    /**
     * Expose a mutable reference to every Ix owned
     * by this datastructure. Any Ix which is not
     * exposed by this function will be invalidated
     * by a garbage collection. The object which
     * was pointed to may also have been collected.
     *
     * If some Ix is owned by two or more instances of
     * this type (such as via Rc<Cell<...>>),
     * then the situation is tricker. Because
     * this is an uncommon use case, and because
     * enforcing uniqueness in the collector would
     * create additional space and time overheads,
     * ensuring uniqueness is a requirement of the implementer.
     *
     * Avoid panicking in this method. A panic may
     * cause some elements to never be dropped, leaking
     * any owned memory outside the region.
     */
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, f: F) where
        F: FnMut(&'b mut Ix<T>);
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


            if let Spot::Present(e) = s {
                e.check_clear_rc();
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
                .get_entry_mut().get_mut();
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
     * This may trigger a garbage collection. As such,
     * a function is used to generate the new value, which
     * can query the state of the world post-collection.
     */
    pub fn alloc<F>(&mut self, make_t: F) -> MutEntry<T, T> where
        F: FnOnce(&Self) -> T
    {
        self.alloci(make_t)
    }
    pub fn alloci<S, F>(&mut self, make_t: F) -> MutEntry<S, T> where
        F: FnOnce(&Self) -> S,
        S: InjectInto<T>
    {
        //else the index could be incorrect
        self.ensure(1);
        let n = self.data.len();
        self.data.push(Spot::Present(Entry::new(make_t(&self).inject())));
        MutEntry {
            ix: Ix::new(n,
                #[cfg(feature = "debug-arena")]
                self.nonce,
                #[cfg(feature = "debug-arena")]
                self.generation,
                ),
            entry: InjectInto::project_mut(self.data.get_mut(n).unwrap().get_entry_mut()).unwrap(),
            root: rc::Weak::new(),
            roots: &mut self.roots
        }
    }

    /**
     * Immediately trigger a standard garbage collection.
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
        e3.as_mut_ref().ix = Some(e3.ix());
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
