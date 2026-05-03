#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

use core::{fmt, mem, ops};
use alloc::{sync::Arc, vec::Vec};

mod own_arc;

use crate::own_arc::OwnArc;

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

impl<T: ?Sized> Arena<T> {
    pub fn new() -> Self {
        Self { own_datas: Vec::new() }
    }

    pub fn garbage_collection(&mut self) {
        self.own_datas.retain(|it| !it.can_drop());
    }
}

impl<T> Arena<T> {
    pub fn alloc(&mut self, data: T) -> SendArc<T> {
        self.garbage_collection();
        let data = Arc::new(data);
        self.own_datas.push(OwnArc(data.clone()));
        SendArc { data }
    }
}

/// Create from [`Arena::alloc`]
#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
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
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

// NOTE: Do not implement DerefMut, otherwise you can swap a normal Arc

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
}
