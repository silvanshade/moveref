use core::{cell::Cell, mem::MaybeUninit};

use crate::slot::Slot;

/// State for tracking the status of a storage [`Slot`].
pub(crate) struct SlotStorageTracker {
    /// Whether the [`Slot`] is initialized.
    initialized: Cell<bool>,
    /// Whether the [`Slot`] is released. If released, [`Drop`] will be skipped.
    released: Cell<bool>,
    /// Number of references to the [`Slot`]. Used for checking various conditions.
    references: Cell<usize>,
}

impl SlotStorageTracker {
    /// Construct a new [`SlotStorageTracker`].
    #[inline]
    pub const fn new() -> Self {
        return Self {
            initialized: Cell::new(false),
            released: Cell::new(false),
            references: Cell::new(0),
        };
    }

    /// Project the the status by borrowing the internal state.
    #[inline]
    pub const fn status(&self) -> SlotStorageStatus<'_> {
        return SlotStorageStatus {
            initialized: &self.initialized,
            released: &self.released,
            references: &self.references, // tarpaulin
        };
    }
}

/// The borrowed form of [`SlotStorageTracker`].
#[derive(Clone, Copy)]
pub(crate) struct SlotStorageStatus<'frame> {
    /// Whether the [`Slot`] is initialized.
    initialized: &'frame Cell<bool>,
    /// Whether the [`Slot`] is released. If released, [`Drop`] will be skipped.
    released: &'frame Cell<bool>,
    /// Number of references to the [`Slot`]. Used for checking various conditions.
    references: &'frame Cell<usize>,
}

impl<'frame> SlotStorageStatus<'frame> {
    /// Set the status to initialized.
    #[inline]
    pub(crate) fn initialize(&self) {
        debug_assert!(!self.is_initialized());
        self.initialized.set(true);
        self.increment();
    }

    /// Increment the reference count.
    #[inline]
    pub(crate) fn increment(&self) {
        debug_assert!(self.is_reference_zeroed());
        self.references.set(self.references.get() + 1);
    }

    /// Decrement the reference count.
    #[inline]
    pub(crate) fn decrement(&self) {
        debug_assert!(!self.is_reference_zeroed());
        self.references.set(self.references.get() - 1);
    }

    /// Mark the storage as released. Once marked as released, [`Drop`] will be skipped.
    #[inline]
    pub(crate) unsafe fn release(&self) {
        self.released.set(true);
    }

    /// Mark the storage as terminated. This is just a decrement followed by an assertion that
    /// references are finally zeroed. It is intended to be called only when the storage is dropped.
    #[inline]
    pub(crate) fn terminate(&self) {
        self.decrement();
        debug_assert!(self.is_reference_zeroed());
    }

    /// Check if the storage is initialized.
    #[inline]
    pub(crate) fn is_initialized(&self) -> bool {
        return self.initialized.get();
    }

    /// Check if the storage is uninitialized.
    #[inline]
    pub(crate) fn is_uninitialized(&self) -> bool {
        return self.is_reference_zeroed() && !self.is_initialized();
    }

    /// Check if the storage is released.
    #[inline]
    pub(crate) fn is_released(&self) -> bool {
        return self.released.get();
    }

    /// Check if the storage is leaking.
    #[inline]
    pub(crate) fn is_leaking(&self) -> bool {
        return !self.is_released() && self.is_initialized() && !self.is_reference_zeroed();
    }

    /// Check if the references are zeroed.
    #[inline]
    pub(crate) fn is_reference_zeroed(&self) -> bool {
        return self.references.get() == 0;
    }
}

/// Kind dictacting whether the storage should drop its referent when leaving scope.
#[allow(clippy::module_name_repetitions)]
#[derive(Copy, Clone, Debug)]
pub enum SlotStorageKind {
    /// The storage should drop its referent.
    Drop,
    /// The storage should not drop its referent.
    Keep,
}

/// Type used for constructing the storage for a [`Slot`] backing a [`MoveRef`](crate::MoveRef).
pub struct SlotStorage<T> {
    /// The kind dictating the storage drop behavior.
    kind: SlotStorageKind,
    /// The raw underlying (possibily uninitialized) storage memory.
    memory: MaybeUninit<T>,
    /// Status flags for the storage which track initialization, dropping state, and reference count.
    tracker: SlotStorageTracker,
    /// Location for reporting panic data.
    #[cfg(debug_assertions)]
    location: &'static core::panic::Location<'static>,
}

impl<T> Drop for SlotStorage<T> {
    fn drop(&mut self) {
        let status = self.tracker.status();
        if status.is_uninitialized() {
            // NOTE: the only time this should happen is when the `SlotStorage` is created manually,
            // outside the use of the macros, since otherwise the storage is initialized immediately
            // after creation.
            return; // tarpaulin
        }
        if status.is_leaking() {
            self.non_unwinding_panic_abort();
        }
        if matches!(self.kind, SlotStorageKind::Drop) {
            unsafe { self.memory.assume_init_drop() }
        }
    }
}

impl<T> SlotStorage<T> {
    /// Construct a new [`SlotStorage<T>`] given a `kind`.
    #[must_use]
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

    /// Project the [`Slot`] for the storage.
    #[inline]
    pub fn slot(&mut self) -> Slot<'_, T> {
        let memory = &mut self.memory;
        let status = self.tracker.status();
        return Slot { memory, status };
    }

    #[inline]
    pub fn display_location(&self) -> &dyn core::fmt::Display {
        /// Placeholder location display.
        const UNKNOWN: &str = "<unknown>";

        if cfg!(debug_assertions) {
            return self.location;
        }
        return &UNKNOWN; // tarpaulin
    }

    /// Force an abort by triggering a panic mid-unwind.
    ///
    /// This is one way to force an LLVM abort from inside of `core` without using
    /// [`core::intrinsics::abort`] which requires `nightly`.
    fn non_unwinding_panic_abort(&self) {
        /// Helper type for triggering the double-panic by panicking on drop.
        struct DropAndPanic;

        #[rustfmt::skip]
        impl Drop for DropAndPanic { // tarpaulin
            #[inline]
            fn drop(&mut self) {
                #[allow(clippy::manual_assert)] // tarpaulin
                if cfg!(not(test)) {
                    panic!("initiating double-panic to trigger an LLVM abort") // tarpaulin
                }
            }
        }

        // Trigger the first panic.
        let _first_panic_trigger = DropAndPanic;

        // Trigger the second panic mid-unwind.
        panic!(
            "a critical reference counter at {} was not zeroed!",
            self.display_location() // tarpaulin
        );
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
