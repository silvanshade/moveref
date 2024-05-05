use core::{mem::MaybeUninit, ops::Deref, pin::Pin};

use crate::new::{New, TryNew};

pub trait Emplace<T>: Sized + Deref {
    type Output: Deref<Target = Self::Target>;

    #[inline]
    fn emplace<N: New<Output = T>>(new: N) -> Self::Output {
        match Self::try_emplace(new) {
            | Ok(val) => return val,
            | Err(err) => match err {},
        }
    }

    /// # Errors
    ///
    /// Should return `Err` if the `new` initializer fails with an error.
    fn try_emplace<N: TryNew<Output = T>>(new: N) -> Result<Self::Output, N::Error>;
}

#[cfg(feature = "alloc")]
impl<T> Emplace<T> for crate::Box<T> {
    type Output = Pin<Self>;

    #[inline]
    fn try_emplace<N: TryNew<Output = T>>(new: N) -> Result<Self::Output, N::Error> {
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
    fn try_emplace<N: TryNew<Output = T>>(new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Rc::new(MaybeUninit::<T>::uninit());
        let ptr = crate::Rc::get_mut(&mut uninit).expect("unreachable: freshly allocated");
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
    fn try_emplace<N: TryNew<Output = T>>(new: N) -> Result<Self::Output, N::Error> {
        let mut uninit = crate::Arc::new(MaybeUninit::<T>::uninit());
        #[rustfmt::skip]
        let ptr = crate::Arc::get_mut(&mut uninit).expect("unreachable: freshly allocated");
        let pin = unsafe { Pin::new_unchecked(ptr) };
        unsafe { new.try_new(pin)? };
        let ptr = unsafe { Self::from_raw(crate::Arc::into_raw(uninit).cast::<T>()) };
        let pin = unsafe { Pin::new_unchecked(ptr) };
        return Ok(pin);
    }
}

#[cfg(test)]
mod tests {
    mod coverage {
        mod emplace {
            #[cfg(feature = "alloc")]
            #[test]
            fn arc() {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Arc<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }

            #[cfg(feature = "alloc")]
            #[test]
            fn r#box() {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Box<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }

            #[cfg(feature = "alloc")]
            #[test]
            fn rc() {
                const VAL: u8 = 128;
                let new = crate::new::by(move || return VAL);
                let out = <crate::Rc<_> as crate::Emplace<_>>::emplace(new);
                assert_eq!(VAL, *out);
            }
        }
    }
}
