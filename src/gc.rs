
use std::rc::Rc;
use std::rc::Weak;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;


pub type Nonce = u64;
#[derive(Hash, Debug)]
pub struct Ix<T> {
    ix: usize,
    _t: PhantomData<*mut T>
}
impl <T> Clone for Ix<T> {
    fn clone(&self) -> Self {
        Ix {
            ix: self.ix,
            _t: PhantomData
        }
    }
}
impl <T> Copy for Ix<T> {}
impl <T> Ix<T> {
    fn new(ix: usize) -> Self {
        Ix { ix, _t: PhantomData }
    }
    pub fn get<'a, 'b: 'a>(&'a self, region: &'b Region<T>) -> &'a T {
        match region.data.get(self.ix).expect("Ix::get: invalid region")
        {
            Spot::Present(e) => &e.t,
            Spot::BrokenHeart(i) => i.get(region),
        }
    }
    pub fn get_mut<'b, 'a : 'b>(&'a self, region: &'b mut Region<T>) -> Option<&'b mut T> {
        let mut i = self;
        match region.data.get_mut(self.ix)? {
            Spot::Present(e) => return Some(&mut e.t),
            Spot::BrokenHeart(i) => panic!("Can't follow broken hearts yet for get_mut")};
    }
}
// We don't need UnsafeCell here it turns out
// but we should only update it with the correct
// capability
type IxCell<T> = Cell<Ix<T>>;
/*
 * Ex is a mutable index, which will receive updates
 * to the index as the source arena moves
 */
#[derive(Clone)]
pub struct Ex<T> {
    nonce: Nonce,
    cell: Rc<IxCell<T>>
}

impl <T> Ex<T> {
    /**
     * Gets the value at this location, when
     * passed the corresponding region.
     *
     * This method will panic if the region is not correct
     * (including if the reference has been invalidated by not being updated)
     */
    pub fn get<'a>(&self, r: &'a Region<T>) -> &'a T {
        if self.nonce == r.nonce {
            match r.data.get(self.cell.get().ix).unwrap() {
                Spot::Present(e) => e.get(),
                Spot::BrokenHeart(_) => panic!("Unexpected broken heart")
            }
        } else {
            panic!("Incorrect region")
        }
    }
}

pub struct Entry<T> {
    t: T,
    rc: Weak<IxCell<T>>
}
impl <T> Entry<T> {
    //upgrade to an Ix, creating the cell if necessary
    fn listen(&mut self, ix: Ix<T>) -> Ex<T> {
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
            nonce: 0,
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

    pub fn new(t: T) -> Self {
        Entry {
            t, rc: Weak::new(),
        }
    }
}
pub enum Spot<T> {
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
    // Change this into a broken heart to other,
    // updating the external reference
    fn forward(&mut self, other: Ix<T>) {
        if let Spot::Present(ref mut e) = self {
            e.move_to(other);
        }
        *self = Spot::BrokenHeart(other);
    }
}
pub struct MutEntry<'a, T> {
    ix: Ix<T>,
    entry: &'a mut Entry<T>,
    roots: &'a mut Vec<Ix<T>>,
}
impl <'a, T> MutEntry<'a, T> {
    pub fn listen(&mut self) -> Ex<T> {
        self.entry.listen(self.ix)
    }
    pub fn root(&mut self) -> Ex<T> {
        self.roots.push(self.ix);
        self.listen()
    }
    pub fn ix(&self) -> Ix<T> {
        self.ix
    }
}
impl <'a, T> Deref for MutEntry<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.entry.get()
    }
}
impl <'a, T> DerefMut for MutEntry<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.entry.get_mut()
    }
}

pub mod nonce {
    use std::sync::atomic::{AtomicU64, Ordering};
    static U: AtomicU64 = AtomicU64::new(0);

    pub fn next() -> super::Nonce {
        U.fetch_add(1, Ordering::Relaxed)
    }
}
pub struct Region<T> {
    data: Vec<Spot<T>>,
    nonce: Nonce,
    roots: Vec<Ix<T>>,
}
impl <T> Region<T> {
    pub fn new() -> Self {
        Region {
            data: Vec::new(),
            nonce: nonce::next(),
            roots: Vec::new(),
        }
    }
}

pub trait HasIx<T> {
}
impl <T: HasIx<T>> Region<T> {
    // Ensure that the capacity will 
    fn prim_ensure_capacity(&mut self, new_elems: usize) {
        let len = self.data.len();
        if len <= 0 {return}
        let cap = self.data.capacity();


    }
    pub fn alloc(&mut self, t: T) -> MutEntry<T> {
        let n = self.data.len();
        self.data.push(Spot::Present(Entry::new(t)));
        MutEntry {
            ix: Ix::new(n),
            entry: self.data.get_mut(n).unwrap().get_entry_mut(),
            roots: &mut self.roots
        }
    }

    pub fn gc(&mut self) {
    }
    pub fn gc_into(self, other: &mut Region<T>) {
    }
}
