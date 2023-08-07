use crate::slot_storage::SlotStorageStatus;
use core::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

pub struct MoveRef<'frame, T: ?Sized> {
    pub(crate) ptr: &'frame mut T,
    pub(crate) status: SlotStorageStatus<'frame>,
}

impl<T: ?Sized> Deref for MoveRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<T: ?Sized> DerefMut for MoveRef<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
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
    pub(crate) unsafe fn new_unchecked(ptr: &'frame mut T, status: SlotStorageStatus<'frame>) -> Self {
        Self { ptr, status }
    }

    #[inline]
    pub fn into_pin(self) -> Pin<Self> {
        unsafe { Pin::new_unchecked(self) }
    }

    #[inline]
    #[must_use]
    pub fn release(pin: Pin<Self>) -> *mut T {
        let mov = unsafe { Pin::into_inner_unchecked(pin) };
        unsafe { mov.status.release() };
        mov.ptr
    }
}

impl<'frame, T> MoveRef<'frame, T> {
    #[inline]
    pub fn into_inner(self) -> T {
        let pin = unsafe { Pin::new_unchecked(self) };
        let ptr = MoveRef::release(pin);
        unsafe { core::ptr::read(ptr) }
    }

    #[inline]
    pub unsafe fn as_ptr(&self) -> *const T {
        self.ptr
    }

    #[inline]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
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
    #[should_panic]
    fn forget_move_ref() {
        bind!(x: MoveRef<i32> = &move 42);
        core::mem::forget(x);
    }

    #[test]
    #[should_panic]
    fn forget_move_ref_temporary() {
        core::mem::forget(expr!(&move 42));
    }

    #[cfg(feature = "alloc")]
    #[test]
    #[should_panic]
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
