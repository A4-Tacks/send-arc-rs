use alloc::sync::Arc;

/// Compare strong_count ordered arc
#[derive(Debug)]
pub(crate) struct OwnArc<T: ?Sized>(pub(crate) Arc<T>);

impl<T: ?Sized> OwnArc<T> {
    pub(crate) fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    pub(crate) fn can_drop(&self) -> bool {
        debug_assert_ne!(self.strong_count(), 0);
        self.strong_count() == 1
    }
}

impl<T: ?Sized> Ord for OwnArc<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Arc::strong_count(&other.0).cmp(&Arc::strong_count(&self.0))
    }
}

impl<T: ?Sized> PartialOrd for OwnArc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> PartialEq for OwnArc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<T: ?Sized> Eq for OwnArc<T> {}
