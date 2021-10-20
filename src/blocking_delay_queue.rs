use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

use crate::delay_item::Delayed;

type MinHeap<T> = BinaryHeap<Reverse<T>>;


trait DelayQueue<T>
    where T: Delayed
{
    fn add(&self, e: T);

    fn offer(&self, e: T, timeout: Duration) -> bool;

    fn peek(&self) -> Option<T>;

    fn take(&self) -> T;

    fn poll(&self, timeout: Duration) -> Option<T>;

    fn remove(&self, e: T) -> bool;

    fn size(&self) -> usize;

    fn clear(&self);
}

pub struct BlockingDelayQueue<T>
{
    heap: Mutex<MinHeap<T>>,
    condvar: Condvar,
}

impl<T> BlockingDelayQueue<T>
    where T: Delayed + Ord
{
    pub fn new_unbounded() -> Self {
        BlockingDelayQueue {
            heap: Mutex::new(BinaryHeap::new()),
            condvar: Condvar::new(),
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        BlockingDelayQueue {
            heap: Mutex::new(BinaryHeap::with_capacity(capacity)),
            condvar: Condvar::new(),
        }
    }

    fn heap_mutex(&self) -> MutexGuard<'_, MinHeap<T>> {
        self.heap.lock().expect("Queue lock poisoned")
    }
}


impl<T> DelayQueue<T> for BlockingDelayQueue<T>
    where T: Delayed + Ord
{
    fn add(&self, e: T) {
        let mut heap = self.heap_mutex();
        if heap.len() < heap.capacity() {
            heap.push(Reverse(e));
        } else {
            let cap = heap.capacity();
            let mut mutex = self
                .condvar
                .wait_while(heap, |h| h.len() >= cap)
                .expect("Queue lock poisoned");
            mutex.push(Reverse(e));
        }

        self.condvar.notify_one()
    }

    fn offer(&self, _e: T, _timeout: Duration) -> bool {
        todo!()
    }

    fn peek(&self) -> Option<T> {
        todo!()
    }

    fn take(&self) -> T {
        todo!()
    }

    fn poll(&self, _timeout: Duration) -> Option<T> {
        todo!()
    }

    fn remove(&self, _e: T) -> bool {
        todo!()
    }

    fn size(&self) -> usize {
        todo!()
    }

    fn clear(&self) {
        self.heap_mutex().clear();
    }
}

