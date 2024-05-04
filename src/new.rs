use core::{mem::MaybeUninit, pin::Pin};

use crate::{into_move::IntoMove, move_ref::MoveRef};

pub trait New: Sized {
    type Output;

    /// # Safety
    ///
    /// - [`New::new()`] must not be used to mutate previously initialized data
    /// - `this` must be freshly-allocated memory
    /// - after invocation, the `this` placement argument is in a valid, initialized state
    #[allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]
    unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>);
}

#[allow(clippy::module_name_repetitions)]
pub trait TryNew {
    type Output;
    type Error;

    /// # Errors
    ///
    /// Should return `Err` if initialization failed.
    ///
    /// # Safety
    ///
    /// - [`TryNew::try_new()`] must not be used to mutate previously initialized data
    /// - `this` must be freshly-allocated memory
    /// - after invocation, the `this` placement argument is in a valid, initialized state
    unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error>;
}

impl<N: New> TryNew for N {
    type Output = N::Output;
    type Error = core::convert::Infallible;

    unsafe fn try_new(self, this: Pin<&mut MaybeUninit<Self::Output>>) -> Result<(), Self::Error> {
        self.new(this);
        return Ok(());
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait CopyNew: Sized {
    /// # Safety
    ///
    /// - the same safety requirements as [`New::new()`] apply with respect to `dst`
    unsafe fn copy_new(src: &Self, dst: Pin<&mut MaybeUninit<Self>>);
}

#[allow(clippy::module_name_repetitions)]
pub trait MoveNew: Sized {
    /// # Safety
    ///
    /// - the same safety requirements as [`New::new()`] apply with respect to `dst`
    unsafe fn move_new(src: Pin<MoveRef<Self>>, dst: Pin<&mut MaybeUninit<Self>>);
}

/// # Safety
///
/// - `initializer` must satisfy the same safety requirements as [`New::new()`]
#[inline]
pub unsafe fn by_raw<T, F>(initializer: F) -> impl New<Output = T>
where
    F: FnOnce(Pin<&mut MaybeUninit<T>>),
{
    struct FnNew<F, T> {
        initializer: F,
        _type: core::marker::PhantomData<fn(Pin<&mut MaybeUninit<T>>)>,
    }

    impl<F, T> New for FnNew<F, T>
    where
        F: FnOnce(Pin<&mut MaybeUninit<T>>),
    {
        type Output = T;
        #[inline]
        unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>) {
            (self.initializer)(this);
        }
    }

    return FnNew {
        initializer,
        _type: core::marker::PhantomData,
    };
}

#[inline]
pub fn by<T, F>(f: F) -> impl New<Output = T>
where
    F: FnOnce() -> T,
{
    let val = f();
    unsafe { return by_raw(|mut dst| dst.set(MaybeUninit::new(val))) }
}

#[inline]
pub fn of<T>(val: T) -> impl New<Output = T> {
    return by(|| return val);
}

#[inline]
pub fn default<T: Default>() -> impl New<Output = T> {
    return by(Default::default);
}

#[inline]
pub fn mov<P>(ptr: P) -> impl New<Output = P::Target>
where
    P: IntoMove,
    P::Target: MoveNew,
{
    unsafe {
        return by_raw(move |dst| {
            bind_slot!(
                #[dropping]
                storage
            );
            let src = ptr.into_move(storage);
            MoveNew::move_new(src, dst);
        });
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
                return crate::new::default();
            }
        }

        impl crate::MoveNew for Pinned {
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
