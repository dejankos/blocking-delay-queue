use std::cmp::Ordering;
use std::time::Instant;

pub trait Delayed {
    fn delay(&self) -> Instant;
}

#[derive(Debug)]
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
