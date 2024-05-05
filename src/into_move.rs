use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

use crate::{deref_move::DerefMove, move_ref::MoveRef, slot::Slot};

pub trait IntoMove: Deref + Sized {
    type Storage: Sized;

    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
impl<T> IntoMove for crate::Box<T> {
    type Storage = crate::Box<MaybeUninit<T>>;

    #[inline]
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        return MoveRef::into_pin(self.deref_move(storage));
    }
}

impl<'f, T: ?Sized> IntoMove for MoveRef<'f, T> {
    type Storage = ();

    #[inline]
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        return MoveRef::into_pin(self.deref_move(storage));
    }
}

impl<P: DerefMove> IntoMove for Pin<P> {
    type Storage = P::Storage;

    #[inline]
    fn into_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        let inner = unsafe { Self::into_inner_unchecked(self) };
        let this = P::deref_move(inner, storage);
        return MoveRef::into_pin(this);
    }
}
