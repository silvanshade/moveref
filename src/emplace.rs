use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

use crate::new::{New, TryNew};

pub trait Emplace<T>: Sized + Deref {
    type Output: Deref<Target = Self::Target>;

    #[inline]
    fn emplace<N: New<Output = T>>(self, new: N) -> Self::Output {
        match self.try_emplace(new) {
            | Ok(val) => return val,
            | Err(err) => match err {},
        }
    }

    /// # Errors
    ///
    /// Should return `Err` if the `new` initializer fails with an error.
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error>;
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Box<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Box::new(MaybeUninit::<T>::uninit());
        let pin = unsafe { Pin::new_unchecked(&mut *uninit) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { Self::from_raw(crate::Box::into_raw(uninit).cast::<T>()) };
        return Ok(Self::into_pin(ptr));
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Rc<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Rc::new(MaybeUninit::<T>::uninit());
        let Some(ptr) = crate::Rc::get_mut(&mut uninit) else {
            unreachable!("No pointers to `uninit` exist since it was freshly created")
        };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { Self::from_raw(crate::Rc::into_raw(uninit).cast::<T>()) };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        return Ok(pin);
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Arc<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Arc::new(MaybeUninit::<T>::uninit());
        let Some(ptr) = crate::Arc::get_mut(&mut uninit) else {
            unreachable!("No pointers to `uninit` exist since it was freshly created")
        };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { Self::from_raw(crate::Arc::into_raw(uninit).cast::<T>()) };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        return Ok(pin);
    }
}
