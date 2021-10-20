use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::delay_item::Delayed;

type MinHeap<T> = BinaryHeap<Reverse<T>>;


trait DelayQueue<T>
    where T: Delayed
{
    fn add(&self, e: T) -> bool;

    fn clear(&self);

    fn offer(&self, e: T, timeout: Duration) -> bool;

    fn peek(&self) -> Option<T>;

    fn take(&self) -> T;

    fn poll(&self, timeout: Duration) -> Option<T>;

    fn remove(&self, e: T) -> bool;

    fn size(&self) -> usize;
}

pub struct BlockingDelayQueue<T>
{
    heap: Mutex<MinHeap<T>>,
    condvar: Condvar
}

impl<T> BlockingDelayQueue<T>
    where T: Delayed + Ord
{
    fn new_unbounded() -> Self {
        BlockingDelayQueue {
            heap: Mutex::new(BinaryHeap::new()),
            condvar: Condvar::new()
        }
    }


    fn new_with_capacity(capacity: usize) -> Self {
        BlockingDelayQueue {
            heap: Mutex::new(BinaryHeap::with_capacity(capacity)),
            condvar: Condvar::new()
        }
    }
}

