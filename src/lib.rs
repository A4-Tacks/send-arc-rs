#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

use core::{fmt, mem, ops};
use alloc::{sync::Arc, vec::Vec};

mod own_arc;

use crate::own_arc::OwnArc;

/// Create some [`SendArc`], which must be dropped after [`SendArc`]
///
/// # Panics
/// When there is an instance of `SendArc`, the drop will panic
#[derive(Debug)]
pub struct Arena<T: ?Sized> {
    own_datas: Vec<OwnArc<T>>,
}

impl<T: ?Sized> Drop for Arena<T> {
    fn drop(&mut self) {
        self.garbage_collection();
        if !self.own_datas.is_empty() {
            let own_datas = mem::take(&mut self.own_datas);
            let len = own_datas.len();
            // NOTE: assume unwind safe
            mem::forget(own_datas);
            panic!("SendArcArena can only be drop after all SendArc have been drop, \
                    there are still {len} that have not been dropped!")
        }
    }
}

impl<T: ?Sized> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> Arena<T> {
    pub fn new() -> Self {
        Self { own_datas: Vec::new() }
    }

    /// Drop [`SendArc`]s that are only referenced by [`Arena`]
    pub fn garbage_collection(&mut self) {
        self.own_datas.retain(|it| !it.can_drop());
    }

    /// Create from an existing unique [`Arc`], and run garbage collection
    ///
    /// # Panics
    ///
    /// Panics if exists other strong or weak references.
    ///
    /// # Examples
    ///
    /// ```
    /// use send_arc::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let data = std::sync::Arc::new(2);
    /// let data = arena.tracking(data);
    /// assert_eq!(*data, 2);
    /// ```
    #[track_caller]
    pub fn tracking(&mut self, mut data: Arc<T>) -> SendArc<T> {
        assert!(
            Arc::get_mut(&mut data).is_some(),
            "Arc is not unique, {} strongs, {} weaks",
            Arc::strong_count(&data),
            Arc::weak_count(&data),
        );
        let own_arc = OwnArc(data.clone());
        self.garbage_collection();
        self.own_datas.push(own_arc);
        SendArc { data }
    }
}

impl<T> Arena<T> {
    /// Create from value, and run garbage collection
    pub fn alloc(&mut self, data: T) -> SendArc<T> {
        self.garbage_collection();
        let data = Arc::new(data);
        self.own_datas.push(OwnArc(data.clone()));
        SendArc { data }
    }
}

/// Create from [`Arena::alloc`] or [`Arena::tracking`]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SendArc<T: ?Sized> {
    data: Arc<T>,
}

impl<T: ?Sized> Clone for SendArc<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone() }
    }
}

impl<T: ?Sized> ops::Deref for SendArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: ?Sized> SendArc<T> {
    // NOTE: Do not implement mutable version, otherwise you can swap a normal Arc
    pub fn inner(this: &Self) -> &Arc<T> {
        &this.data
    }

    pub fn into_inner(this: Self) -> Arc<T> {
        this.data
    }
}

impl<T: ?Sized> From<SendArc<T>> for Arc<T> {
    fn from(value: SendArc<T>) -> Self {
        SendArc::into_inner(value)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for SendArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SendArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T: ?Sized + fmt::Pointer> fmt::Pointer for SendArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

// SAFETY: SendArc always drop in original thread
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T: ?Sized + Sync> Send for SendArc<T> {}

// SAFETY: like reference Sync implement
unsafe impl<T: ?Sized + Sync> Sync for SendArc<T> {}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::marker::PhantomData;
    use super::*;

    struct Ty {
        id: std::thread::ThreadId,
        _non_send: PhantomData<std::sync::MutexGuard<'static, ()>>,
    }

    impl Drop for Ty {
        fn drop(&mut self) {
            assert_eq!(self.id, std::thread::current().id());
        }
    }

    impl Ty {
        fn new() -> Self {
            Self {
                id: std::thread::current().id(),
                _non_send: PhantomData,
            }
        }
    }

    fn needs_send(x: impl Send) {
        std::thread::scope(move |scope| {
            scope.spawn(move || {
                drop(x);
            });
            std::thread::sleep(std::time::Duration::from_millis(100));
        });
    }

    fn _check_ref_sync_projection(value: impl Sync) {
        fn needs_sync(_: impl Sync) {}
        needs_sync(&value);
    }

    #[test]
    fn it_works() {
        let mut arena = Arena::new();
        let a = arena.alloc(Ty::new());
        let b = arena.alloc(Ty::new());
        needs_send(a);
        let c = b.clone();
        needs_send(c);
    }

    #[test]
    #[should_panic = "is not unique, 2 strongs, 0 weaks"]
    fn tracking_shared_panic() {
        let mut arena = Arena::new();
        let data = Arc::new(Ty::new());
        let _other = data.clone();
        let _a = arena.tracking(data);
    }

    #[test]
    #[should_panic = "is not unique, 1 strongs, 1 weaks"]
    fn tracking_weak_references_panic() {
        let mut arena = Arena::new();
        let data = Arc::new(Ty::new());
        let _other_weak = Arc::downgrade(&data);
        let _a = arena.tracking(data);
    }
}
