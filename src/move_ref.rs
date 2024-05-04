use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use crate::slot_storage::SlotStorageStatus;

pub struct MoveRef<'frame, T: ?Sized> {
    pub(crate) ptr: &'frame mut T,
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
            return;
        }
        self.status.terminate();
        unsafe { core::ptr::drop_in_place(self.ptr) }
    }
}

impl<'frame, T: ?Sized> MoveRef<'frame, T> {
    #[inline]
    pub(crate) unsafe fn new_unchecked(
        ptr: &'frame mut T,
        status: SlotStorageStatus<'frame>,
    ) -> Self {
        return Self { ptr, status };
    }

    #[must_use]
    #[inline]
    pub fn into_pin(self) -> Pin<Self> {
        return unsafe { Pin::new_unchecked(self) };
    }

    #[inline]
    #[must_use]
    pub fn release(pin: Pin<Self>) -> *mut T {
        let mov = unsafe { Pin::into_inner_unchecked(pin) };
        unsafe { mov.status.release() };
        return mov.ptr;
    }
}

impl<'frame, T> MoveRef<'frame, T> {
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> T {
        let pin = unsafe { Pin::new_unchecked(self) };
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
    use crate::MoveRef;

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
}
