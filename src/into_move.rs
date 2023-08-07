use crate::{deref_move::DerefMove, move_ref::MoveRef, slot::Slot};
use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

pub trait IntoMove: Deref + Sized {
    type Storage: Sized;

    fn into_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
impl<T> IntoMove for crate::Box<T> {
    type Storage = crate::Box<MaybeUninit<T>>;

    #[inline]
    fn into_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        MoveRef::into_pin(self.deref_move(storage))
    }
}

impl<'f, T: ?Sized> IntoMove for MoveRef<'f, T> {
    type Storage = ();

    #[inline]
    fn into_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        MoveRef::into_pin(self.deref_move(storage))
    }
}

impl<P: DerefMove> IntoMove for Pin<P> {
    type Storage = P::Storage;

    #[inline]
    fn into_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        let inner = unsafe { Pin::into_inner_unchecked(self) };
        let this = P::deref_move(inner, storage);
        MoveRef::into_pin(this)
    }
}

#[cfg(feature = "cxx")]
pub unsafe trait CxxUniquePtrAllocate {
    unsafe fn allocate_uninitialized_cxx_storage() -> *mut Self;
    unsafe fn free_uninitialized_cxx_storage(ptr: *mut Self);
}

#[cfg(feature = "cxx")]
pub struct CxxUniquePtrStorage<T: CxxUniquePtrAllocate>(*mut T);

#[cfg(feature = "cxx")]
impl<T> CxxUniquePtrStorage<T>
where
    T: CxxUniquePtrAllocate,
{
    #[inline]
    fn assume_init_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

#[cfg(feature = "cxx")]
impl<T> IntoMove for cxx::UniquePtr<T>
where
    T: CxxUniquePtrAllocate,
    T: cxx::memory::UniquePtrTarget,
{
    type Storage = CxxUniquePtrStorage<T>;

    fn into_move<'frame>(self, storage: Slot<'frame, Self::Storage>) -> Pin<MoveRef<'frame, Self::Target>>
    where
        Self: 'frame,
    {
        let cast = CxxUniquePtrStorage(self.into_raw());
        let (ptr, status) = storage.write(cast);
        let this = unsafe { MoveRef::new_unchecked(ptr.assume_init_mut(), status) };
        MoveRef::into_pin(this)
    }
}
