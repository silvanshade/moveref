use core::{mem::MaybeUninit, ops::DerefMut};

use crate::{into_move::IntoMove, move_ref::MoveRef, slot::Slot};

/// Derefencing move operations for [`MoveRef`].
///
/// This trait serves a similar purpose for [`MoveRef`] as [`Deref`](core::ops::Deref) does for
/// normal references.
///
/// In order to implement this trait, the `Self` pointer type must be the *unique owner* of its
/// referent, such that dropping `Self` would cause its referent's destructor to run.
///
/// This is a subtle condition that depends upon the semantics of the `Self` pointer type and must
/// be verified by the implementer, hence the unsafety.
///
/// Examples:
///
/// - [`MoveRef<T>`] implements [`DerefMove`] by definition.
/// - [`Box<T>`](crate::Box<T>) implements [`DerefMove`] because when it drops it destructs `T`.
/// - `&mut T` does *not* implement [`DerefMove`] because it is non-owning.
/// - [`Arc<T>`](crate::Arc<T>) does *not* implement [`DerefMove`] because it is not *uniquely*
///   owning.
/// - [`Rc<T>`](crate::Rc<T>) does *not* implement [`DerefMove`] because it is not *uniquely*
///   owning.
/// - [`Pin<P>`](core::pin::Pin<T>) given `P: DerefMove`, implements [`DerefMove`] only when
///   `P::Target: Unpin`, because `DerefMove: DerefMut` and `Pin<P>: DerefMut` requires `P::Target:
///   Unpin`.
///
/// # Safety
///
/// Correctness for [`DerefMove`] impls require that the unique ownership invariant of [`MoveRef`]
/// is upheld. In particular, the following function *must not* violate memory safety:
/// ```
/// # use moveref::{DerefMove, MoveRef, bind};
/// fn move_out_of<P>(ptr: P) -> P::Target
/// where
///   P: DerefMove,
///   P::Target: Sized,
/// {
///   unsafe {
///     // Move out of `ptr` into a fresh `MoveRef` (backed by a fresh storage `Slot`).
///     bind!(mvp = &move *ptr);
///
///     // From this point on, the `P::Target` destructor must run when, and only when,
///     // the function's return value goes out of scope per the usual Rust rules.
///     //
///     // In particular, the original `ptr` or any pointer it came from must not
///     // run the destructor when they go out of scope, under any circumstance.
///     MoveRef::into_inner(mvp)
///   }
/// }
/// ```
pub unsafe trait DerefMove: DerefMut + IntoMove {
    /// Construct a [`MoveRef`] by dereferencing `self` and moving its contents into `storage`.
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
        return unsafe { MoveRef::new_unchecked(ptr, status) };
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
        return self;
    }
}
