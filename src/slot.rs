use core::{mem::MaybeUninit, pin::Pin};

use crate::{
    move_ref::MoveRef,
    new::{New, TryNew},
    slot_storage::SlotStorageStatus,
};

/// Backing storage for a [`MoveRef`].
pub struct Slot<'frame, T> {
    /// The raw underlying (possibily uninitialized) storage memory.
    pub(crate) memory: &'frame mut MaybeUninit<T>,
    /// Status flags for the storage which track initialization, dropping state, and reference count.
    pub(crate) status: SlotStorageStatus<'frame>,
}

impl<'frame, T> Slot<'frame, T> {
    /// Construct and pin `new` into the slot and return the associated owning [`MoveRef`].
    #[inline]
    pub fn emplace<N: New<Output = T>>(self, new: N) -> Pin<MoveRef<'frame, T>> {
        match self.try_emplace(new) {
            | Ok(pin) => return pin,
            | Err(err) => match err {},
        }
    }

    /// Try to construct and pin `new` into the slot and return the associated owning [`MoveRef`].
    ///
    /// # Errors
    ///
    /// Should return `Err` if the `new` initializer fails with an error.
    #[inline]
    pub fn try_emplace<N: TryNew<Output = T>>(
        self,
        new: N,
    ) -> Result<Pin<MoveRef<'frame, T>>, N::Error> {
        self.status.initialize();
        unsafe { new.try_new(Pin::new_unchecked(self.memory))? };
        let ptr = unsafe { self.memory.assume_init_mut() };
        let mov = unsafe { MoveRef::new_unchecked(ptr, self.status) };
        let pin = mov.into_pin();
        return Ok(pin);
    }

    /// Move and pin `val` into the slot and return the associated owning [`MoveRef`].
    #[inline]
    pub fn pin(self, val: T) -> Pin<MoveRef<'frame, T>> {
        return self.emplace(crate::new::of(val));
    }

    /// Move `val` into the slot and return the associated owning [`MoveRef`].
    #[inline]
    pub fn put(self, val: T) -> MoveRef<'frame, T> {
        let pin = self.pin(val);
        return unsafe { Pin::into_inner_unchecked(pin) }; // tarpaulin
    }

    /// Write `val` into the slot and returns a `&mut ref` to its location and its storage status.
    #[inline]
    pub(crate) fn write(self, val: T) -> (&'frame mut T, SlotStorageStatus<'frame>) {
        self.status.initialize();
        let ptr = self.memory.write(val);
        return (ptr, self.status);
    }
}
