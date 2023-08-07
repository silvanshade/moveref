use crate::new::{New, TryNew};
use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

pub trait Emplace<T>: Sized + Deref {
    type Output: Deref<Target = Self::Target>;

    #[inline]
    fn emplace<N: New<Output = T>>(self, new: N) -> Self::Output {
        match self.try_emplace(new) {
            Ok(val) => val,
            Err(err) => match err {},
        }
    }

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
        let ptr = unsafe { crate::Box::from_raw(crate::Box::into_raw(uninit).cast::<T>()) };
        Ok(crate::Box::into_pin(ptr))
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Rc<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Rc::new(MaybeUninit::<T>::uninit());
        let ptr = match crate::Rc::get_mut(&mut uninit) {
            Some(ptr) => ptr,
            None => unreachable!("No pointers to `uninit` exist since it was freshly created"),
        };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { crate::Rc::from_raw(crate::Rc::into_raw(uninit).cast::<T>()) };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        Ok(pin)
    }
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Arc<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Arc::new(MaybeUninit::<T>::uninit());
        let ptr = match crate::Arc::get_mut(&mut uninit) {
            Some(ptr) => ptr,
            None => unreachable!("No pointers to `uninit` exist since it was freshly created"),
        };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { crate::Arc::from_raw(crate::Arc::into_raw(uninit).cast::<T>()) };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        Ok(pin)
    }
}
