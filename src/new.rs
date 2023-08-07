use crate::{into_move::IntoMove, move_ref::MoveRef};
use core::{mem::MaybeUninit, pin::Pin};

pub unsafe trait New: Sized {
    type Output;

    unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>);
}

pub unsafe trait TryNew {
    type Output;
    type Error;

    unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error>;
}

unsafe impl<N: New> TryNew for N {
    type Output = N::Output;
    type Error = core::convert::Infallible;

    unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error> {
        self.new(this);
        Ok(())
    }
}

pub unsafe trait CopyNew: Sized {
    unsafe fn copy_new(src: &Self, dst: Pin<&mut MaybeUninit<Self>>);
}

pub unsafe trait MoveNew: Sized {
    unsafe fn move_new(src: Pin<MoveRef<Self>>, dst: Pin<&mut MaybeUninit<Self>>);
}

#[inline]
pub unsafe fn by_raw<T, F>(f: F) -> impl New<Output = T>
where
    F: FnOnce(Pin<&mut MaybeUninit<T>>),
{
    struct FnNew<F, T> {
        f: F,
        _type: core::marker::PhantomData<fn(Pin<&mut MaybeUninit<T>>)>,
    }

    unsafe impl<F, T> New for FnNew<F, T>
    where
        F: FnOnce(Pin<&mut MaybeUninit<T>>),
    {
        type Output = T;
        #[inline]
        unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>) {
            (self.f)(this)
        }
    }

    FnNew {
        f,
        _type: core::marker::PhantomData,
    }
}

#[inline]
pub fn by<T, F>(f: F) -> impl New<Output = T>
where
    F: FnOnce() -> T,
{
    let val = f();
    unsafe { by_raw(|mut dst| dst.set(MaybeUninit::new(val))) }
}

#[inline]
pub fn of<T>(val: T) -> impl New<Output = T> {
    by(|| val)
}

#[inline]
pub fn default<T: Default>() -> impl New<Output = T> {
    by(Default::default)
}

#[inline]
pub fn mov<P>(ptr: P) -> impl New<Output = P::Target>
where
    P: IntoMove,
    P::Target: MoveNew,
{
    unsafe {
        by_raw(move |dst| {
            bind_slot!(
                #[dropping]
                storage
            );
            let src = ptr.into_move(storage);
            MoveNew::move_new(src, dst);
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mov() {
        #[derive(Default)]
        struct Pinned {
            _pinned: core::marker::PhantomPinned,
        }

        impl Pinned {
            fn new() -> impl New<Output = Self> {
                crate::new::default()
            }
        }

        unsafe impl crate::MoveNew for Pinned {
            unsafe fn move_new(
                _src: core::pin::Pin<MoveRef<Self>>,
                _dst: core::pin::Pin<&mut core::mem::MaybeUninit<Self>>,
            ) {
            }
        }

        bind!(pinned = Pinned::new());
        let _pinned = crate::new::mov(pinned);
    }
}
