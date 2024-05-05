use core::{cell::Cell, mem::MaybeUninit};

use crate::slot::Slot;

pub(crate) struct SlotStorageTracker {
    initialized: Cell<bool>,
    released: Cell<bool>,
    references: Cell<usize>,
}

impl SlotStorageTracker {
    #[inline]
    pub const fn new() -> Self {
        return Self {
            initialized: Cell::new(false),
            released: Cell::new(false),
            references: Cell::new(0),
        };
    }

    #[inline]
    pub const fn status(&self) -> SlotStorageStatus<'_> {
        return SlotStorageStatus {
            initialized: &self.initialized,
            released: &self.released,
            references: &self.references, // tarpaulin
        };
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SlotStorageStatus<'frame> {
    initialized: &'frame Cell<bool>,
    released: &'frame Cell<bool>,
    references: &'frame Cell<usize>,
}

impl<'frame> SlotStorageStatus<'frame> {
    #[allow(unused)] // NOTE: used (indirectly) in macros
    #[inline]
    pub(crate) const fn new(
        initialized: &'frame Cell<bool>,
        released: &'frame Cell<bool>,
        references: &'frame Cell<usize>,
    ) -> Self {
        #[rustfmt::skip]
        return Self {
            initialized, // tarpaulin
            released,    // tarpaulin
            references,  // tarpaulin
        };
    }

    #[inline]
    pub(crate) fn increment(&self) {
        debug_assert!(self.references_are_zeroed());
        self.references.set(self.references.get() + 1);
    }

    #[inline]
    pub(crate) fn initialize(&self) {
        debug_assert!(!self.is_initialized());
        self.initialized.set(true);
        self.increment();
    }

    #[inline]
    pub(crate) fn decrement(&self) {
        debug_assert!(!self.references_are_zeroed());
        self.references.set(self.references.get() - 1);
    }

    #[inline]
    pub(crate) fn is_leaking(&self) -> bool {
        return !self.is_released() && self.is_initialized() && !self.references_are_zeroed();
    }

    #[inline]
    pub(crate) fn is_initialized(&self) -> bool {
        return self.initialized.get();
    }

    #[inline]
    pub(crate) fn is_uninitialized(&self) -> bool {
        return self.references_are_zeroed() && !self.is_initialized();
    }

    #[inline]
    pub(crate) fn is_released(&self) -> bool {
        return self.released.get();
    }

    #[inline]
    pub(crate) fn references_are_zeroed(&self) -> bool {
        return self.references.get() == 0;
    }

    #[inline]
    pub(crate) unsafe fn release(&self) {
        self.released.set(true);
    }

    pub(crate) fn terminate(&self) {
        self.decrement();
        debug_assert!(self.references_are_zeroed());
    }
}

#[allow(clippy::module_name_repetitions)]
#[allow(unused)] // NOTE: used in macros
#[derive(Copy, Clone, Debug)]
pub enum SlotStorageKind {
    Drop,
    Keep,
}

pub struct SlotStorage<T> {
    kind: SlotStorageKind,
    memory: MaybeUninit<T>,
    tracker: SlotStorageTracker,
    #[cfg(debug_assertions)]
    location: &'static core::panic::Location<'static>,
}

impl<T> Drop for SlotStorage<T> {
    #[inline]
    fn drop(&mut self) {
        let status = self.tracker.status();
        if status.is_uninitialized() {
            // NOTE: the only time this should happen is when the `SlotStorage` is created manually,
            // outside the use of the macros, since otherwise the storage is initialized immediately
            // after creation.
            return; // tarpaulin
        }
        if status.is_leaking() {
            self.double_panic();
        }
        if matches!(self.kind, SlotStorageKind::Drop) {
            unsafe { self.memory.assume_init_drop() }
        }
    }
}

impl<T> SlotStorage<T> {
    #[inline]
    fn double_panic(&self) {
        struct DoublePanic;

        #[rustfmt::skip]
        impl Drop for DoublePanic { // tarpaulin
            #[inline]
            fn drop(&mut self) {
                assert!(!cfg!(not(test))); // tarpaulin
            }
        }

        let _double_panic = DoublePanic;

        #[cfg(debug_assertions)] // tarpaulin
        panic!(
            "a critical reference counter at {} was not zeroed!",
            self.location // tarpaulin
        );

        #[cfg(not(debug_assertions))] // tarpaulin
        panic!("a critical reference counter was not zeroed!"); // tarpaulin
    }
}

impl<T> SlotStorage<T> {
    #[must_use]
    #[allow(unused)] // NOTE: used in macros
    #[inline]
    pub const fn new(kind: SlotStorageKind) -> Self {
        return Self {
            kind,                          // tarpaulin
            memory: MaybeUninit::uninit(), // tarpaulin
            tracker: SlotStorageTracker::new(),
            #[cfg(debug_assertions)]       // tarpaulin
            location: core::panic::Location::caller(),
        };
    }

    #[allow(unused)] // NOTE: used in macros
    #[inline]
    pub fn slot(&mut self) -> Slot<'_, T> {
        let memory = &mut self.memory;
        let status = self.tracker.status();
        return Slot { memory, status };
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    mod coverage {
        use super::*;

        #[test]
        fn slot_storage_drop() {
            let kind = SlotStorageKind::Drop;
            let storage = SlotStorage::<()>::new(kind);
            drop(storage);
        }
    }
}
