/**
 * Trait for indicating that this type can be contained in a larger type.
 */
pub trait InjectInto<T> : Sized {
    fn inject(self) -> T;
    fn project(t: T) -> Option<Self>;
    fn project_ref(t: &T) -> Option<&Self>;
    fn project_mut(t: &mut T) -> Option<&mut Self>;
}
impl <T> InjectInto<T> for T {
    fn inject(self) -> Self { self }
    fn project(t: T) -> Option<Self> { Some(t) }
    fn project_ref(t: &T) -> Option<&Self> { Some(t) }
    fn project_mut(t: &mut T) -> Option<&mut Self> { Some(t) }
}

