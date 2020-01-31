use core::fmt::{Debug, Formatter};
use alloc::rc::Rc;
use alloc::rc;
use core::cell::Cell;

use crate::types::{Ix, IxCell, SpotVariant, Weak};

#[derive(Debug)]
pub(crate) struct Entry<T> {
    // We'll always keep an RC live here so that
    // the weak pointers can use upgrade() to check.
    // At GC time, we clear if weak_count is 0
    rc: Option<Rc<IxCell<T>>>,
    t: T,
}
impl <T> Entry<T> {
    //upgrade to an Ix, creating the cell if necessary
    pub(crate) fn weak(&mut self, ix: Ix<T>) -> Weak<T> {
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

    #[inline(always)]
    pub fn get(&self) -> &T {
        &self.t
    }
    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.t
    }

    pub(crate) fn move_to(&mut self, other: Ix<T>) {
        self.check_clear_rc();
        if let Some(ref mut rc) = self.rc {
            rc.set(other)
        }
    }

    pub(crate) fn check_clear_rc(&mut self) {
        if let Some(ref mut rc) = self.rc {
            if 0 == Rc::weak_count(rc) {
                self.rc = None;
            }
        }
    }

    pub(crate) fn new(t: T) -> Self {
        Entry {
            t, rc: None,
        }
    }
}


#[derive(Debug)]
pub(crate) enum Spot<T> {
    Present(Entry<T>),
    BrokenHeart(Ix<T>),
}


impl <T> Spot<T> {
    pub(crate) fn new(t: T) -> Self {
        Spot::Present(Entry::new(t))
    }

    pub(crate) fn variant(&mut self) -> SpotVariant<Entry<T>, T> {
        match self {
            Spot::Present(e) => SpotVariant::Present(e),
            Spot::BrokenHeart(i) => SpotVariant::BrokenHeart(*i)
        }
    }

    pub(crate) fn get(&self) -> Option<&Entry<T>> {
        match self {
            Spot::Present(e) => Some(e),
            _ => None,
        }
    }

    pub(crate) fn get_mut(&mut self) -> Option<&mut Entry<T>> {
        match self {
            Spot::Present(e) => Some(e),
            _ => None,
        }
    }

    #[allow(unused)]
    pub(crate) fn into_t(self) -> Option<T> {
        match self {
            Spot::Present(e) => Some(e.t),
            Spot::BrokenHeart(_) => None,
        }
    }
    // Change this into a broken heart to other,
    // updating the external reference
    #[allow(unused)]
    pub(crate) fn move_to(&mut self, other: Ix<T>) -> Spot<T> {
        if let Spot::Present(ref mut e) = self {
            e.move_to(other);
        }
        core::mem::replace(self, Spot::BrokenHeart(other))
    }
}
impl <T> Weak<T> {
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
impl <T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        Weak {cell: self.cell.clone()}
    }
}
impl <T> Debug for Weak<T> {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        self.cell.upgrade().fmt(f)
    }
}
