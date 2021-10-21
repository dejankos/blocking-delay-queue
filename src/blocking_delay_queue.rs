use std::borrow::Borrow;
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

    fn peek(&self) -> Option<&T>;

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
        if capacity == 0 {
            Self::new_unbounded()
        } else {
            BlockingDelayQueue {
                heap: Mutex::new(BinaryHeap::with_capacity(capacity)),
                condvar: Condvar::new(),
            }
        }
    }

    fn heap_mutex(&self) -> MutexGuard<'_, MinHeap<T>> {
        self.heap.lock().expect("Queue lock poisoned")
    }

    fn can_accept_element(m: &MutexGuard<MinHeap<T>>) -> bool {
        if m.capacity() == 0 {
            true
        } else {
            m.len() < m.capacity()
        }
    }
}

impl<T> DelayQueue<T> for BlockingDelayQueue<T>
    where T: Delayed + Ord
{
    fn add(&self, e: T) {
        let mut heap = self.heap_mutex();
        if Self::can_accept_element(&heap) {
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

    fn offer(&self, e: T, timeout: Duration) -> bool {
        let mut heap = self.heap_mutex();
        if Self::can_accept_element(&heap) {
            heap.push(Reverse(e));
            self.condvar.notify_one();
            true
        } else {
            let cap = heap.capacity();
            let mut mutex = self
                .condvar
                .wait_timeout_while(heap, timeout, |h| h.len() >= cap)
                .expect("Queue lock poisoned");
            if mutex.1.timed_out() {
                false
            } else {
                mutex.0.push(Reverse(e));
                self.condvar.notify_one();
                true
            }
        }
    }

    fn peek(&self) -> Option<&T> {
        let option = self.peeker();
        None
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::blocking_delay_queue::{BlockingDelayQueue, DelayQueue};
    use crate::delay_item::DelayItem;

    #[test]
    fn should_put_() {
        let q = BlockingDelayQueue::new_unbounded();

        q.add(DelayItem {
            data: "123",
            delay: Instant::now(),
        });
    }
}