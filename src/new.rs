use core::{mem::MaybeUninit, pin::Pin};

use crate::{into_move::IntoMove, move_ref::MoveRef};

/// Types which can be constructed (initialized) into some provided storage.
pub trait New: Sized {
    type Output;

    /// Initialize `Self` using `this` for storage.
    ///
    /// # Safety
    ///
    /// - [`New::new()`] must not be used to mutate previously initialized data
    /// - `this` must be freshly-allocated memory
    /// - after invocation, the `this` placement argument is in a valid, initialized state
    #[allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]
    unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>);
}

/// Types which can be constructed (initialized) into some provided storage. Construction may fail.
#[allow(clippy::module_name_repetitions)]
pub trait TryNew {
    type Output;
    type Error;

    /// Try to initialize `Self` using `this` for storage.
    ///
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
        return Ok(()); // tarpaulin
    }
}

/// Types which can be copy-constructed from an existing value into some provided storage.
#[allow(clippy::module_name_repetitions)]
pub trait CopyNew: Sized {
    /// # Safety
    ///
    /// - the same safety requirements as [`New::new()`] apply with respect to `dst`
    unsafe fn copy_new(src: &Self, dst: Pin<&mut MaybeUninit<Self>>);
}

/// Types which can be move-constructed from an existing value into some provided storage.
#[allow(clippy::module_name_repetitions)]
pub trait MoveNew: Sized {
    /// # Safety
    ///
    /// - the same safety requirements as [`New::new()`] apply with respect to `dst`
    unsafe fn move_new(src: Pin<MoveRef<Self>>, dst: Pin<&mut MaybeUninit<Self>>);
}

/// Constructs a [`New`] value using a thunk which initializes its data into some pinned,
/// uninitialized memory.
///
/// # Safety
///
/// - `initializer` must satisfy the same safety requirements as [`New::new()`]
#[inline]
pub unsafe fn by_raw<T, F>(initializer: F) -> impl New<Output = T>
where
    F: FnOnce(Pin<&mut MaybeUninit<T>>),
{
    /// Helper type for converting into the abstract `impl New`.
    struct FnNew<F, T> {
        /// The underlying thunk.
        initializer: F,
        /// Phantom type holding `T`, respecting variance.
        _type: core::marker::PhantomData<fn(Pin<&mut MaybeUninit<T>>)>,
    }

    #[rustfmt::skip]
    impl<F, T> New for FnNew<F, T> // tarpaulin
    where
        F: FnOnce(Pin<&mut MaybeUninit<T>>),
    {
        type Output = T; // tarpaulin
        #[inline]
        unsafe fn new(self, this: Pin<&mut MaybeUninit<Self::Output>>) {
            (self.initializer)(this);
        }
    }

    return FnNew {
        initializer,                      // tarpaulin
        _type: core::marker::PhantomData, // tarpaulin
    };
}

/// Constructs a [`New`] value using a value-producing thunk `f`.
#[inline]
pub fn by<T, F>(f: F) -> impl New<Output = T>
where
    F: FnOnce() -> T,
{
    let val = f();
    unsafe { return by_raw(|mut dst| dst.set(MaybeUninit::new(val))) }
}

/// Constructs a [`New`] value using a given value `val`.
#[inline]
pub fn of<T>(val: T) -> impl New<Output = T> {
    return by(|| return val);
}

/// Constructs a [`New`] value for a type `T` using it's default value.
#[inline]
pub fn default<T: Default>() -> impl New<Output = T> {
    return by(Default::default);
}

/// Constructs a [`New`] value from a *uniquely* owning pointer `P` by moving its referent into the
/// eventually provided storage.
#[inline]
pub fn mov<P>(ptr: P) -> impl New<Output = P::Target>
where
    P: IntoMove,
    P::Target: MoveNew,
{
    unsafe {
        #[rustfmt::skip] // tarpaulin
        return by_raw(move |dst| {
            bind_slot!(      // tarpaulin
                #[dropping]  // tarpaulin
                storage      // tarpaulin
            );
            let src = ptr.into_move(storage); // tarpaulin
            MoveNew::move_new(src, dst);      // tarpaulin
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
