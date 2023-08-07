use crate::{into_move::IntoMove, move_ref::MoveRef, slot::Slot};
use core::{mem::MaybeUninit, ops::DerefMut, pin::Pin};

pub unsafe trait DerefMove: DerefMut + IntoMove {
    fn deref_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
unsafe impl<T> DerefMove for crate::Box<T> {
    #[inline]
    fn deref_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        let cast = crate::Box::into_raw(self).cast::<MaybeUninit<T>>();
        let cast = unsafe { crate::Box::from_raw(cast) };
        let (ptr, status) = storage.write(cast);
        let ptr = unsafe { ptr.assume_init_mut() };
        unsafe { MoveRef::new_unchecked(ptr, status) }
    }
}

unsafe impl<'f, T: ?Sized> DerefMove for MoveRef<'f, T> {
    #[inline]
    fn deref_move<'frame>(self, _storage: Slot<'frame, Self::Storage>) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        self
    }
}

#[cfg(feature = "cxx")]
unsafe impl<T> DerefMove for cxx::UniquePtr<T>
where
    T: crate::into_move::CxxUniquePtrAllocate,
    T: cxx::memory::UniquePtrTarget,
    T: Unpin,
{
    fn deref_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        Pin::into_inner(self.into_move(storage))
    }
}
