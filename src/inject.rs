/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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
