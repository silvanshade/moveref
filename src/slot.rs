use crate::{
    move_ref::MoveRef,
    new::{New, TryNew},
    slot_storage::SlotStorageStatus,
};
use core::{mem::MaybeUninit, pin::Pin};

pub struct Slot<'frame, T> {
    pub(crate) memory: &'frame mut MaybeUninit<T>,
    pub(crate) status: SlotStorageStatus<'frame>,
}

impl<'frame, T> Slot<'frame, T> {
    #[inline]
    pub fn emplace<N: New<Output = T>>(self, new: N) -> Pin<MoveRef<'frame, T>> {
        match self.try_emplace(new) {
            Ok(pin) => pin,
            Err(err) => match err {},
        }
    }

    #[inline]
    pub fn try_emplace<N: TryNew<Output = T>>(self, new: N) -> Result<Pin<MoveRef<'frame, T>>, N::Error> {
        self.status.initialize();
        unsafe { new.try_new(Pin::new_unchecked(self.memory))? };
        let ptr = unsafe { self.memory.assume_init_mut() };
        let mov = unsafe { MoveRef::new_unchecked(ptr, self.status) };
        let pin = mov.into_pin();
        Ok(pin)
    }

    #[inline]
    pub fn pin(self, val: T) -> Pin<MoveRef<'frame, T>> {
        self.emplace(crate::new::of(val))
    }

    #[inline]
    pub fn put(self, val: T) -> MoveRef<'frame, T> {
        let pin = self.pin(val);
        unsafe { Pin::into_inner_unchecked(pin) }
    }

    #[inline]
    pub(crate) fn write(self, val: T) -> (&'frame mut T, SlotStorageStatus<'frame>) {
        self.status.initialize();
        let ptr = self.memory.write(val);
        (ptr, self.status)
    }
}
