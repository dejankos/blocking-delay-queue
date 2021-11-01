use std::cmp::Ordering;
use std::time::Instant;

/// A mix-in style trait for marking structures that can be used as expiring items.
/// An implementation of this trait must define a [`delay`] method providing delay
/// associated with this structure.
///
/// An example of such structure is [DelayItem](delay_item::DelayItem) providing a generic implementation
/// used for wrapping delayed structures / data.
///
/// #Examples
/// Basic usage:
/// ```
/// use std::time::Instant;
/// use blocking_delay_queue::{Delayed, DelayItem};
/// let delayed = DelayItem::new(1, Instant::now());
/// let instant = delayed.delay();
/// ```
///
pub trait Delayed {
    /// Return the delay associated with this structure.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::Instant;
    /// use blocking_delay_queue::{Delayed, DelayItem};
    /// let delayed = DelayItem::new(1, Instant::now());
    /// let instant = delayed.delay();
    /// ```
    fn delay(&self) -> Instant;
}

/// A provided convenient structure for delayed data.
///
/// #Examples
/// Basic usage:
/// ```
/// use std::time::Instant;
/// use blocking_delay_queue::{Delayed, DelayItem};
/// let delayed = DelayItem::new(1, Instant::now());
pub struct DelayItem<T> {
    pub data: T,
    pub delay: Instant,
}

impl<T> Ord for DelayItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.delay.cmp(&other.delay)
    }
}

impl<T> PartialOrd for DelayItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.delay.cmp(&other.delay))
    }
}

impl<T> PartialEq for DelayItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.delay == other.delay
    }
}

impl<T> Eq for DelayItem<T> {}

impl<T> DelayItem<T> {
    pub fn new(data: T, delay: Instant) -> Self {
        DelayItem { data, delay }
    }
}

impl<T> Delayed for DelayItem<T> {
    fn delay(&self) -> Instant {
        self.delay
    }
}
