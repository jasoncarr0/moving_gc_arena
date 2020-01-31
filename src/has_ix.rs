
use crate::types::Ix;
use alloc::{
    boxed::Box,
    vec::Vec
};

/**
 * Trait to expose contained indices to the garbage collector.
 */
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
impl <T : 'static, S: HasIx<T>> HasIx<T> for Vec<S> {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.iter_mut().for_each(|o| {o.foreach_ix(&mut f)});
    }
}
impl <T : 'static> HasIx<T> for () {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut _f: F) where
        F: FnMut(&'b mut Ix<T>)
    { }
}
impl <T : 'static, S1: HasIx<T>, S2: HasIx<T>> HasIx<T> for (S1, S2) {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.0.foreach_ix(&mut f);
        self.1.foreach_ix(&mut f);
    }
}
impl <T : 'static, S1: HasIx<T>, S2: HasIx<T>, S3: HasIx<T>> HasIx<T> for (S1, S2, S3) {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.0.foreach_ix(&mut f);
        self.1.foreach_ix(&mut f);
        self.2.foreach_ix(&mut f);
    }
}
impl <T : 'static, S1: HasIx<T>, S2: HasIx<T>, S3: HasIx<T>, S4: HasIx<T>> HasIx<T> for (S1, S2, S3, S4) {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.0.foreach_ix(&mut f);
        self.1.foreach_ix(&mut f);
        self.2.foreach_ix(&mut f);
        self.3.foreach_ix(&mut f);
    }
}
impl <T : 'static, S: HasIx<T>> HasIx<T> for Option<S> {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.iter_mut().for_each(|o|{o.foreach_ix(&mut f)})
    }
}
impl <T : 'static, S: HasIx<T>> HasIx<T> for Box<S> {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        self.as_mut().foreach_ix(&mut f);
    }
}
impl <T : 'static, S: HasIx<T>> HasIx<T> for &mut S {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        (*self).foreach_ix(&mut f);
    }
}
impl <T : 'static> HasIx<T> for Ix<T> {
    fn foreach_ix<'b, 'a : 'b, F>(&'a mut self, mut f: F) where
        F: FnMut(&'b mut Ix<T>)
    {
        f(self);
    }
}
