/// Macro for binding a variable to a fresh [`MoveRef`](crate::MoveRef).
///
/// - `bind!(x = &move *ptr)` creates an `x: MoveRef<T>` given `ptr: impl (DerefMove +
///   DerefMut<Target = T>)`
///
/// The above invocation moves the referent of a *uniquely* owning poiner into a fresh
/// [`MoveRef`](crate::MoveRef) bound to `x`.
///
/// - `bind!(x = &move val)` creates an `x: MoveRef<T>` given `val: T`
///
/// The above invocation moves any value into a fresh [`MoveRef`](crate::MoveRef) bound to `x`.
///
/// - `bind!(x = con)` creates an `x: Pin<MoveRef<T>>` given `con: impl New<Output = T>`
///
/// The above invocaton constructs a [`New`](crate::New) value into a fresh
/// [`MoveRef`](crate::MoveRef) bound to `x`.
///
/// - `bind!(mut x: T = ...)` (with right-hand side of `&move *ptr` or `&move val` or `con`)
///
/// The above generalization can be used with any earlier invocation form to add mutability and
/// typing annotations.
#[macro_export]
macro_rules! bind {
    (mut $name:ident $(: $ty:ty)? = &move *$expr:expr) => {
        $crate::bind!(@move(mut) $name, $($ty)?, $expr)
    };
    ($name:ident $(: $ty:ty)? = &move *$expr:expr) => {
        $crate::bind!(@move $name, $($ty)?, $expr)
    };
    (mut $name:ident $(: $ty:ty)? = &move $expr:expr) => {
        $crate::bind!(@put(mut) $name, $($ty)?, $expr)
    };
    ($name:ident $(: $ty:ty)? = &move $expr:expr) => {
        $crate::bind!(@put $name, $($ty)?, $expr)
    };
    (mut $name:ident $(: $ty:ty)? = $expr:expr) => {
        $crate::bind!(@emplace(mut) $name, $($ty)?, $expr);
    };
    ($name:ident $(: $ty:ty)? = $expr:expr) => {
        $crate::bind!(@emplace $name, $($ty)?, $expr);
    };
    (@move $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(#[dropping] slot);
        #[allow(unused_mut)]
        let $($mut)? $name $(: $ty)? = $crate::DerefMove::deref_move($expr, slot);
    };
    (@put $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(slot);
        let $($mut)? $name $(: $ty)? = slot.put($expr);
    };
    (@emplace $(($mut:tt))? $name:ident, $($ty:ty)?, $expr:expr) => {
        $crate::bind_slot!(slot);
        let $($mut)? $name $(: $ty)? = slot.emplace($expr);
    };
}

/// Macro for creating a fresh [`MoveRef`](crate::MoveRef) expression.
///
/// Because a `v: MoveRef<'frame, T>` always has an associated lifetime `'frame`, this macro can
/// generally only be used where it would be immediately consumed by some enclosing expression, such
/// as in the position of a function argument.
///
/// Trying to use this as `let v = expr!(...)` will not work because the lifetime `'frame` will not
/// expand to the enclosing let binding. Hence the need for the separate `bind!(v = ...)` macro.
///
/// Otherwise, the usage is generally the same as `bind!(...)`:
///
/// - `expr!(&move *ptr)` creates a `MoveRef<T>` given `ptr: impl (DerefMove +
///   DerefMut<Target = T>)`
///
/// The above invocation moves the referent of a *uniquely* owning poiner into a fresh
/// [`MoveRef`](crate::MoveRef).
///
/// - `expr!(&move val)` creates an `MoveRef<T>` given `val: T`
///
/// The above invocation moves any value into a fresh [`MoveRef`](crate::MoveRef).
///
/// - `expr!(con)` creates an `x: Pin<MoveRef<T>>` given `con: impl New<Output = T>`
///
/// The above invocaton constructs a [`New`](crate::New) value into a fresh
/// [`MoveRef`](crate::MoveRef).
#[macro_export]
macro_rules! expr {
    (&move *$expr:expr) => {
        $crate::DerefMove::deref_move(
            $expr,
            $crate::expr_slot!(
                #[dropping]
            )
        )
    };
    (&move $expr:expr) => {
        $crate::expr_slot!().put($expr)
    };
    ($expr:expr) => {
        $crate::expr_slot!().emplace($expr)
    };
}

/// Macro for binding a variable to a fresh [`Slot`](crate::Slot) for storage of a
/// [`MoveRef`](crate::MoveRef).
///
/// - `bind_slot!(x)`
///
/// The above invocation binds a slot to the variable `x`.
///
/// - `bind_slot!(#[dropping] x)`
///
/// The above invocation binds a (dropping) slot to the variable `x`. A [`MoveRef`](crate::MoveRef)
/// using `x` as backing storage will drop its referent when `x` goes out of scope.
///
/// Both of the above invocation forms also allow typing annotations on `x` as with [`bind!`].
#[macro_export]
macro_rules! bind_slot {
    (#[dropping] $($name:ident : $ty:ty),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Drop;
            let mut storage = $crate::SlotStorage::<$ty>::new(kind);
            let $name = storage.slot();
        )*
    };
    (#[dropping] $($name:ident),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Drop;
            let mut storage = $crate::SlotStorage::new(kind);
            let $name = storage.slot();
        )*
    };
    ($($name:ident : $ty:ty),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Keep;
            let mut storage = $crate::SlotStorage::<$ty>::new(kind);
            let $name = storage.slot();
        )*
    };
    ($($name:ident),* $(,)?) => {
        $(
            let kind = $crate::SlotStorageKind::Keep;
            let mut storage = $crate::SlotStorage::new(kind);
            let $name = storage.slot();
        )*
    };
}

/// Macro for creating a fresh [`Slot`](crate::Slot) expression.
///
/// This macro has the same relationship to [`bind_slot!`] as [`expr!`] does to [`bind!`].
#[macro_export]
macro_rules! expr_slot {
    (#[dropping]) => {{
        let kind = $crate::SlotStorageKind::Drop;
        $crate::SlotStorage::new(kind).slot()
    }};
    () => {{
        let kind = $crate::SlotStorageKind::Keep;
        $crate::SlotStorage::new(kind).slot()
    }};
}

/// Boilerplate macro for defining trivial [`CopyNew`](crate::CopyNew) instances.
macro_rules! trivial_copy {
    ($($ty:ty $(where [$($targs:tt)*])?),* $(,)?) => {
        $(
            impl<$($($targs)*)?> $crate::new::CopyNew for $ty where Self: ::core::clone::Clone {
                unsafe fn copy_new(
                    this: &Self,
                    that: ::core::pin::Pin<&mut ::core::mem::MaybeUninit<Self>>,
                ) {
                    let that = ::core::pin::Pin::into_inner_unchecked(that);
                    let data = this.clone();
                    that.write(data);
                }
            }
        )*
    }
}

/// Boilerplate macro for defining trivial [`MoveNew`](crate::CopyNew) instances.
macro_rules! trivial_move {
    ($($ty:ty $(where [$($targs:tt)*])?),* $(,)?) => {
        $(
            impl<$($($targs)*)?> $crate::new::MoveNew for $ty {
                unsafe fn move_new(
                    this: ::core::pin::Pin<$crate::move_ref::MoveRef<'_, Self>>,
                    that: ::core::pin::Pin<&mut ::core::mem::MaybeUninit<Self>>,
                ) {
                    let this = ::core::pin::Pin::into_inner_unchecked(this);
                    let that = ::core::pin::Pin::into_inner_unchecked(that);
                    let data = $crate::move_ref::MoveRef::into_inner(this);
                    that.write(data);
                }
            }
        )*
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    mod macros {
        use super::*;

        const VAL: bool = true;

        #[test]
        fn deref_move_expr() {
            assert_eq!(VAL, *expr!(&move *Box::new(VAL)));
        }

        #[test]
        fn trivial_copy() {
            let this = &true;
            let that = ::core::mem::MaybeUninit::uninit();
            let mut that = ::core::pin::pin!(that);
            unsafe { new::CopyNew::copy_new(this, that.as_mut()) };
            let that = unsafe { that.assume_init() };
            assert_eq!(this, &that);
        }

        #[test]
        fn trivial_move() {
            bind!(this = new::of(VAL));
            let that = ::core::mem::MaybeUninit::uninit();
            let mut that = ::core::pin::pin!(that);
            unsafe { new::MoveNew::move_new(this, that.as_mut()) };
            let that = unsafe { that.assume_init() };
            assert_eq!(VAL, that);
        }
    }
}
