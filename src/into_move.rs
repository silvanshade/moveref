use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

use crate::{deref_move::DerefMove, move_ref::MoveRef, slot::Slot};

/// A trait for transforming a [`Deref`] type into a pinned [`MoveRef`] with respect to a specified
/// backing storage type [`IntoMove::Storage`].
pub trait IntoMove: Deref + Sized {
    type Storage: Sized;

    /// Consume `self` and create a pinned [`MoveRef`] with `self`'s contents placed into `storage`.
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

#[cfg(test)]
mod tests {
    use crate::*;

    mod coverage {
        use super::*;

        mod into_move {
            use super::*;

            const VAL: &str = "value";

            #[cfg(feature = "alloc")]
            #[test]
            fn r#box() {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                let mref = IntoMove::into_move(Box::new(VAL), slot);
                assert_eq!(VAL, *mref);
            }

            #[test]
            fn move_ref() {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                bind!(val: MoveRef<&str> = &move VAL);
                let mref = IntoMove::into_move(val, slot);
                assert_eq!(VAL, *mref);
            }

            #[test]
            fn pin() {
                let kind = SlotStorageKind::Drop;
                let mut storage = SlotStorage::new(kind);
                let slot = storage.slot();
                let mref = IntoMove::into_move(Box::pin(VAL), slot);
                assert_eq!(VAL, *mref);
            }
        }
    }
}
