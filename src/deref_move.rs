use core::{mem::MaybeUninit, ops::DerefMut, pin::Pin};

use crate::{into_move::IntoMove, move_ref::MoveRef, slot::Slot};

/// # Safety
///
/// Correctness for `DerefMove` impls require that the uniqueness invariant
/// of [`MoveRef`] is upheld. In particular, the following function *must not*
/// violate memory safety:
/// ```
/// # use moveref::{DerefMove, MoveRef, bind};
/// fn move_out_of<P>(p: P) -> P::Target
/// where
///   P: DerefMove,
///   P::Target: Sized,
/// {
///   unsafe {
///     // Replace `p` with a move reference into it.
///     bind!(p = &move *p);
///
///     // Move out of `p`. From this point on, the `P::Target` destructor must
///     // run when, and only when, the function's return value goes out of
///     // scope per the usual Rust rules.
///     //
///     // In particular, the original `p` or any pointer it came from must not
///     // run the destructor when they go out of scope, under any circumstance.
///     MoveRef::into_inner(p)
///   }
/// }
/// ```
pub unsafe trait DerefMove: DerefMut + IntoMove {
    fn deref_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame;
}

#[cfg(feature = "alloc")]
unsafe impl<T> DerefMove for crate::Box<T> {
    #[inline]
    fn deref_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        let cast = Self::into_raw(self).cast::<MaybeUninit<T>>();
        let cast = unsafe { crate::Box::from_raw(cast) };
        let (ptr, status) = storage.write(cast);
        let ptr = unsafe { ptr.assume_init_mut() };
        unsafe { MoveRef::new_unchecked(ptr, status) }
    }
}

unsafe impl<'f, T: ?Sized> DerefMove for MoveRef<'f, T> {
    #[inline]
    fn deref_move<'frame>(
        self,
        _storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
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
    fn deref_move<'frame>(
        self,
        storage: Slot<'frame, Self::Storage>,
    ) -> MoveRef<'frame, Self::Target>
    where
        Self: 'frame,
    {
        Pin::into_inner(self.into_move(storage))
    }
}
