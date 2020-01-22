
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::rc;
use std::cell::Cell;
use std::mem::{forget, MaybeUninit, ManuallyDrop};
use std::hint::unreachable_unchecked;

use crate::types::{Ix, IxCell, SpotVariant};


#[inline(always)]
unsafe fn invariant_unreachable() {
    if cfg!(debug_assertions) {
        unreachable!()
    } else {
        unreachable_unchecked()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
/**
 * The present data contains a usize.
 * Values of this type must have bottom
 * bit 0.
 *
 * If the collector is not marking, then the bottom
 * two bits must be 00.
 *
 * If the collector is marking, the bottom two bits
 * can also be 10, to indicate a mark
 *
 * The value, once it is aligned by ensuring that
 * the bottom two bits are 00, will be one of two
 * things:
 * 0,
 * or a valid
 * pointer to a *const IxCell<T> which was
 * created by Rc::into_raw on the correct type.
 * That IxCell<T> will be the 
 */
struct PresentData(usize);
impl PresentData {
    // NOTE: Although this type is Copy
    // This effectively should consume, so the Rc will need
    // to be forgotten if this is only a borrow
    unsafe fn into_unchecked<T>(self) -> Option<*const IxCell<T>> {
        match self.0 & (!3usize) {
            0 => None,
            ptr => {
                Some(ptr as *const IxCell<T>)
            }
        }
    }
    unsafe fn from_unchecked<T>(rc: Option<*const IxCell<T>>) -> Self {
        PresentData(match rc {
            Some(rc) => rc as usize,
            None => 0usize,
        })
    }
}

/**
 * A BrokenHeart contains an index that this entry
 * has been relocated to.
 *
 * We likewise assume that the bottom two bits are
 * available for our purposes.
 */
#[repr(C)]
#[derive(Clone, Copy)]
struct BrokenHeart(usize);
impl BrokenHeart {
    unsafe fn into_unchecked<T>(self) -> Ix<T> {
        Ix::new(self.0.wrapping_shr(1))
    }
    fn from_unchecked<T>(ix: Ix<T>) -> Self {
        let val = ix.ix().wrapping_shl(1) | 1usize;
        assert!(val & 1 == 1);
        BrokenHeart(val)
    }
}

/**
 * Unsafe header. May be smaller and more performant,
 * but less-obviously correct, and making more assumptions
 * about architecture.
 *
 * We assume the following of the architecture:
 * Pointers are 4-byte aligned.
 * The all-zero pointer is not a valid pointer.
 *
 * The small size is necessary for some use cases, since
 * it fits in the same size as an Ix<T>
 */
#[repr(C)] // necessary to have correct size
union Header {
    present: PresentData,
    broken_heart: BrokenHeart,
    bits: usize, // raw view, no invariants
}
enum TaggedHeader<T> {
    Present(Option<*const IxCell<T>>),
    BrokenHeart(Ix<T>),
}
impl <T> Default for TaggedHeader<T> {
    fn default() -> TaggedHeader<T> {
        TaggedHeader::Present(None)
    }
}

impl Header {
    //
    // NOTE for safety: Throughout these methods,
    // the correct representation is needed,
    // else the Header::sort method is immediate
    // UB
    //
    // The method itself is not unsafe as it
    // not exposed outside this module
    //

    // Header MUST point to present value
    fn weak<T>(&mut self, ix: Ix<T>) -> Weak<T> {
        let rc: Rc<IxCell<T>> = self.use_tag(|v| {
            match v {
                TaggedHeader::Present(Some(ptr)) => {
                    unsafe {
                        let rc = Rc::from_raw(ptr);
                        let rc2 = rc.clone();
                        let ptr = Rc::into_raw(rc);
                        (TaggedHeader::Present(Some(ptr)), rc2)
                    }
                }
                TaggedHeader::Present(None) => {
                    let rc = Rc::new(Cell::new(ix));
                    let ptr = Rc::into_raw(rc.clone());
                    (TaggedHeader::Present(Some(ptr)), rc)
                }
                _ => panic!("Invalid header state")
            }});
        let cell = Rc::downgrade(&rc);
        assert!(Rc::weak_count(&rc) == 1);
        Weak { cell }
    }

    fn present() -> Self {
        Header {
            present: PresentData(0)
        }
    }

    fn broken_heart<T>(ix: Ix<T>) -> Self {
        Header {
            broken_heart: BrokenHeart::from_unchecked(ix)
        }
    }

    #[inline(always)]
    fn use_tag<F, T, O>(&mut self, f: F) -> O where
        F: FnOnce(TaggedHeader<T>) -> (TaggedHeader<T>, O)
    {
        // must have type invariants above
        let tagged: TaggedHeader<T> = unsafe { self.get_tag() };
        let (tagged, ret) = f(tagged);
        unsafe {match tagged {
            TaggedHeader::Present(rc) => {
                self.present = PresentData::from_unchecked(rc)
            }
            TaggedHeader::BrokenHeart(bh) => {
                self.broken_heart = BrokenHeart::from_unchecked(bh)
            }
        }};
        ret
    }

    #[inline(always)]
    unsafe fn get_tag<T>(&self) -> TaggedHeader<T> {
        unsafe {
            match self.bits & 1usize {
                0 => TaggedHeader::Present(
                    PresentData::into_unchecked(self.present).clone()),
                1 => TaggedHeader::BrokenHeart(
                    BrokenHeart::into_unchecked(self.broken_heart)),
                _ => unreachable!()
            }
        }
    }
}

/**
 * A spot consists of a header, and some optional data
 * that might contain a T.
 *
 * NOTE: As a safety invariant,
 * the value is initialized whenever 
 * Header<T>.present is valid (as PresentData, not as a usize)
 *
 * The value may also be initialized in other situations as
 * required by algorithms, but importantly, those cases will
 * fail to drop, so the value should never be dropped while
 *
 */
pub(crate) struct Spot<T> {
    header: Header,
    value: MaybeUninit<T>,
}
impl <T> Drop for Spot<T> {
    fn drop(&mut self) {
        unsafe {
            match self.header.get_tag::<T>() {
                TaggedHeader::Present(ptr) => {
                    // drop contents
                    let _ = std::ptr::drop_in_place(self.value.as_mut_ptr());
                    // drop rc
                    let _ = if let Some(ptr) = ptr {
                        Rc::from_raw(ptr);
                    };
                },
                _ => ()
            }
        }
    }
}

impl <T> Spot<T> {
    pub(crate) fn new(t: T) -> Self {
        Spot {
            header: Header::present(),
            value: MaybeUninit::new(t)
        }
    }

    pub(crate) fn get(&self) -> Option<&Entry<T>> {
        unsafe {
            match self.header.get_tag::<T>() {
                TaggedHeader::Present(_) =>
                    Some(std::mem::transmute(self)),
                _ => None,
            }
        }
    }
    pub(crate) fn get_mut(&mut self) -> Option<&mut Entry<T>> {
        unsafe {
            match self.header.get_tag::<T>() {
                TaggedHeader::Present(_) =>
                    Some(std::mem::transmute(self)),
                _ => None,
            }
        }
    }
    pub(crate) fn move_to(&mut self, other: Ix<T>) -> Spot<T> {
        if let Some(e) = self.get_mut() {
            e.move_to(other);
        };
        std::mem::replace(self,
            Spot {
                header: Header::broken_heart(other),
                value: MaybeUninit::uninit(),
            }
        )
    }
    pub(crate) fn variant(&mut self) -> SpotVariant<Entry<T>, T> {
        unsafe {
            match self.header.get_tag::<T>() {
                TaggedHeader::Present(rc) => {
                    forget(rc);
                    SpotVariant::Present(std::mem::transmute(self))
                },
                TaggedHeader::BrokenHeart(i) =>
                    SpotVariant::BrokenHeart(i)
            }
        }
    }
}

// NOTE for safety: Header *must*
// be present and data *must*
// be initialized
/**
 * A spot that is guaranted to be present
 * (both header and value).
 *
 * Although we can't enforce type invaraints,
 * every method ensures the invariants at each
 * point
 */
#[repr(transparent)]
pub(crate) struct Entry<T> {
    spot: Spot<T>
}
impl <T> Entry<T> {
    unsafe fn enforce_valid(&self) {
        match self.spot.header.get_tag::<T>() {
            TaggedHeader::Present(_) => (),
            _ => invariant_unreachable(),
        }
        //enforce valid T
        let _ = &*self.spot.value.as_ptr();
    }

    unsafe fn from_spot_unchecked(spot: Spot<T>) -> Self {
        let ret = Entry { spot };
        ret.enforce_valid();
        ret
    }

    pub(crate) fn weak(&mut self, ix: Ix<T>) -> Weak<T> {
        unsafe {
            self.enforce_valid();
            self.spot.header.weak(ix)
        }
    }

    pub(crate) fn move_to(&mut self, other: Ix<T>) {
        unsafe {
            self.enforce_valid();
            match self.spot.header.get_tag::<T>() {
                TaggedHeader::Present(Some(ptr)) => {
                    let rc = Rc::from_raw(ptr);
                    rc.set(other);
                    let _ = Rc::into_raw(rc);
                },
                TaggedHeader::Present(None) => (),
                _ => unreachable!()
            }
        }
    }

    pub(crate) fn check_clear_rc(&mut self) {
        unsafe {
            self.enforce_valid();
            self.spot.header.use_tag(|v: TaggedHeader<T>| { match v{
                TaggedHeader::Present(Some(ptr)) => {
                    let rc = Rc::from_raw(ptr);
                    if 0 == Rc::weak_count(&rc) {
                        (TaggedHeader::Present(None), ())
                        // rc dropped
                    } else {
                        // rc into pointer
                        let ptr = Rc::into_raw(rc);
                        (TaggedHeader::Present(Some(ptr)), ())
                    }
                },
                v => (v, ())
            }});
        }
    }

    pub fn get(&self) -> &T {
        unsafe {
            self.enforce_valid();
            &*self.spot.value.as_ptr()
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            self.enforce_valid();
            &mut *self.spot.value.as_mut_ptr()
        }
    }
}



pub struct Weak<T> {
    cell: rc::Weak<IxCell<T>>
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
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.cell.upgrade().fmt(f)
    }
}
