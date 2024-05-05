use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use crate::slot_storage::SlotStorageStatus;

/// A "reference" type which *uniquely* owns its referent type `T` with respect to external storage
/// with lifetime `'frame`.
///
/// Conceptually, it has these characteristics:
///
/// - similar to `&'frame mut` because it *uniquely* references other data with lifetime `'frame`
/// - similar to `Box` because it is *owning*
///
/// The distinguishing characteristic of [`MoveRef`] from `&mut` and [`Box`](crate::Box) is how it
/// is created with a backing storage [`Slot`](crate::Slot) and how that defines its ownership of
/// the referent data, and ultimately how the backing storage is responsible for running the
/// destructor for its referent when it finally goes out of scope.
///
/// A motivating example for [`MoveRef`] is the concept of placement-initialization in C++:
///
/// Imagine we define FFI bindings for a C++ class we intend to use in Rust.
///
/// Creating instances for this class on the heap is straightforward and well understood: we can use
/// raw pointers and eventually either convert to a reference or [`Box`](crate::Box).
///
/// Creating instances for this class on the stack is more difficult. We can use
/// [`MaybeUninit`](core::mem::MaybeUninit) to create a chunk of data and initialize into that.
///
/// But we have to be particularly careful when using the result because in Rust, data moves by
/// default, rather than copies by default as in C++. So any access of the data in Rust could
/// potentially move the data out from under some expected location in C++ and cause a crash when
/// execution proceeds again in C++.
///
/// So we need a type which acts like a (mutable) reference but does not let us move simply by
/// accessing it. This would be similar to a [`Pin<&mut T>`], where the [`Pin`] prevents movement,
/// but the inner `&mut` still allows mutation.
///
/// But we also want the possibility to *actually* move the data in some cases, like we would
/// explicitly do in C++ with a move constructor or move assignment operation.
///
/// This interface is exactly what [`MoveRef`] provides, along with [`DerefMove`](crate::DerefMove).
pub struct MoveRef<'frame, T: ?Sized> {
    /// The underlying mutable reference with referent stored in some external [`Slot`](crate::Slot).
    pub(crate) ptr: &'frame mut T,
    /// Status flags for the storage which track initialization, dropping state, and reference count.
    pub(crate) status: SlotStorageStatus<'frame>,
}

impl<'frame, T: ?Sized> core::fmt::Debug for MoveRef<'frame, T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        return core::fmt::Debug::fmt(self.ptr, f);
    }
}

impl<T: ?Sized> Deref for MoveRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        return self.ptr;
    }
}

impl<T: ?Sized> DerefMut for MoveRef<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.ptr;
    }
}

impl<T: ?Sized> Drop for MoveRef<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if self.status.is_released() {
            return; // tarpaulin
        }
        self.status.terminate();
        unsafe { core::ptr::drop_in_place(self.ptr) }
    }
}

impl<'frame, T: ?Sized> MoveRef<'frame, T> {
    /// Create a new unchecked [`MoveRef`] from a mutable ref and [`SlotStorageStatus`].
    #[inline]
    pub(crate) unsafe fn new_unchecked(
        ptr: &'frame mut T,
        status: SlotStorageStatus<'frame>,
    ) -> Self {
        return Self { ptr, status };
    }

    /// Transform a [`MoveRef<T>`] into a [`Pin<MoveRef<T>>`]. This is safe because the interface
    /// for [`MoveRef`] enforces that its referent will not be implicitly moved or have its storage
    /// invalidated until the [`MoveRef<T>`] (and its backing [`Slot`](crate::Slot)) is dropped.
    #[must_use]
    #[inline]
    pub fn into_pin(self) -> Pin<Self> {
        return unsafe { Pin::new_unchecked(self) }; // tarpaulin
    }

    /// Consume a [`Pin<Self>`] and return a raw `*mut T`. This operation inhibits destruction of
    /// `T` by implicit [`Drop`] and the caller becomes responsible for eventual explicit
    /// destruction and cleanup, otherwise the memory will leak.
    #[inline]
    #[must_use]
    pub fn release(pin: Pin<Self>) -> *mut T {
        let mov = unsafe { Pin::into_inner_unchecked(pin) }; // tarpaulin
        unsafe { mov.status.release() };
        return mov.ptr;
    }
}

impl<'frame, T> MoveRef<'frame, T> {
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> T {
        let pin = unsafe { Pin::new_unchecked(self) }; // tarpaulin
        let ptr = MoveRef::release(pin);
        return unsafe { core::ptr::read(ptr) };
    }

    #[must_use]
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        return self.ptr;
    }

    #[must_use]
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        return self.ptr;
    }
}

impl<'s, 't, S, T: ?Sized> PartialEq<MoveRef<'s, S>> for MoveRef<'t, T>
where
    T: PartialEq<S>,
{
    #[inline]
    fn eq(&self, other: &MoveRef<'s, S>) -> bool {
        return self.ptr == other.ptr;
    }
}

impl<'t, T> Eq for MoveRef<'t, T> where T: PartialEq
{
}

impl<'s, 't, S, T: ?Sized> PartialOrd<MoveRef<'s, S>> for MoveRef<'t, T>
where
    T: PartialOrd<S>,
{
    #[inline]
    fn partial_cmp(&self, other: &MoveRef<'s, S>) -> Option<core::cmp::Ordering> {
        return self.ptr.partial_cmp(&other.ptr);
    }

    #[inline]
    fn lt(&self, other: &MoveRef<'s, S>) -> bool {
        return self.ptr.lt(&other.ptr);
    }

    #[inline]
    fn le(&self, other: &MoveRef<'s, S>) -> bool {
        return self.ptr.le(&other.ptr);
    }

    #[inline]
    fn gt(&self, other: &MoveRef<'s, S>) -> bool {
        return self.ptr.gt(&other.ptr);
    }

    #[inline]
    fn ge(&self, other: &MoveRef<'s, S>) -> bool {
        return self.ptr.ge(&other.ptr);
    }
}

impl<'t, T> Ord for MoveRef<'t, T>
where
    T: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        return self.ptr.cmp(&other.ptr);
    }
}

impl<'t, T: ?Sized> core::hash::Hash for MoveRef<'t, T>
where
    T: core::hash::Hash,
{
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn deref_move_of_move_ref() {
        bind!(x: MoveRef<crate::Box<i32>> = &move crate::Box::new(5));
        bind!(y: MoveRef<crate::Box<i32>> = &move *x);
        let z = y;
        assert_eq!(**z, 5);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn deref_move_of_box() {
        let x = crate::Box::new(5);
        bind!(y: MoveRef<i32> = &move *x);
        let z = y;
        assert_eq!(*z, 5);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn move_ref_into_inner() {
        bind!(x: MoveRef<crate::Box<i32>> = &move crate::Box::new(5));
        let y = x.into_inner();
        assert_eq!(*y, 5);
    }

    #[test]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_move_ref() {
        bind!(x: MoveRef<i32> = &move 42);
        core::mem::forget(x);
    }

    #[test]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_move_ref_temporary() {
        core::mem::forget(expr!(&move 42));
    }

    #[cfg_attr(miri, ignore)]
    #[cfg(all(feature = "alloc", not(feature = "valgrind")))]
    #[test]
    #[should_panic(expected = "a critical reference counter at")]
    fn forget_deref_moved_box() {
        let mut x = crate::Box::new(5);
        let ptr = x.as_mut() as *mut i32;
        core::mem::forget(expr!(&move *x));
        unsafe {
            alloc::alloc::dealloc(ptr as *mut u8, alloc::alloc::Layout::new::<i32>());
        }
    }

    #[test]
    fn release_inhibits_drop() {
        struct T;
        impl Drop for T {
            fn drop(&mut self) {
                panic!();
            }
        }
        let val = T;
        bind!(t = crate::new::of(val));
        let _ = MoveRef::release(t);
    }

    mod coverage {
        use super::*;

        mod move_ref {
            use super::*;

            const VAL1: &str = "value1";
            const VAL2: &str = "value2";

            #[test]
            fn as_ptr() {
                bind!(val = &move *Box::new(VAL1));
                let ptr = val.as_ptr();
                assert_eq!(VAL1, unsafe { *ptr });
            }

            #[test]
            fn as_mut_ptr() {
                bind!(mut val = &move *Box::new(VAL1));
                let ptr = val.as_mut_ptr();
                assert_eq!(VAL1, unsafe { *ptr });
                unsafe { ptr.write(VAL2) };
                assert_eq!(VAL2, unsafe { *ptr });
            }

            #[test]
            fn deref_mut() {
                bind!(mut val = &move VAL1);
                assert_eq!(VAL1, *val);
                *val = VAL2;
                assert_eq!(VAL2, *val);
            }

            #[test]
            fn fmt() {
                use crate::alloc::format;
                bind!(val = &move VAL1);
                assert_eq!(format!("{VAL1:#?}"), format!("{val:#?}"));
            }

            #[test]
            fn partial_eq() {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL1);
                assert!(lhs.eq(&rhs));
            }

            #[test]
            fn partial_cmp() {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL1);
                assert!(matches!(
                    lhs.partial_cmp(&rhs),
                    Some(core::cmp::Ordering::Equal)
                ));
            }

            #[test]
            fn lt() {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(lhs.lt(&rhs));
            }

            #[test]
            fn le() {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(lhs.le(&rhs));
            }

            #[test]
            fn gt() {
                bind!(lhs = &move VAL2);
                bind!(rhs = &move VAL1);
                assert!(lhs.gt(&rhs));
            }

            #[test]
            fn ge() {
                bind!(lhs = &move VAL2);
                bind!(rhs = &move VAL1);
                assert!(lhs.ge(&rhs));
            }

            #[test]
            fn cmp() {
                bind!(lhs = &move VAL1);
                bind!(rhs = &move VAL2);
                assert!(matches!(lhs.cmp(&rhs), core::cmp::Ordering::Less));
            }

            #[cfg(feature = "default")]
            #[test]
            fn hash() {
                use core::hash::{Hash, Hasher};
                bind!(lhs = &move VAL1);
                let hash1 = {
                    let mut hasher = seahash::SeaHasher::new();
                    lhs.hash(&mut hasher);
                    hasher.finish()
                };
                bind!(rhs = &move VAL1);
                let hash2 = {
                    let mut hasher = seahash::SeaHasher::new();
                    rhs.hash(&mut hasher);
                    hasher.finish()
                };
                assert_eq!(hash1, hash2);
            }
        }
    }
}
