
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::delay_item::Delayed;

type MinHeap<T> = BinaryHeap<Reverse<T>>;


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

        println!("added element");
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

    fn take(&self) -> T {
        let item = self.take_inner();
        self.condvar.notify_one();

        item
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

    fn take_inner(&self) -> T {
        let heap = self.heap_mutex();
        let current_time = Instant::now();

        let condition = |heap: &mut MinHeap<T>| {
            heap.peek()
                .map_or(true, |item| {
                    println!("eval {}", item.0.delay() >= current_time);
                    !(item.0.delay() >= current_time)
                } )
        };

        if let Some(item) = heap.peek() {
            if Self::is_expired(&item.0) {
                Self::pop(heap)
            } else {
                let delay = item.0.delay() - current_time;
                let (heap, cond) = self
                // let heap = self
                    .condvar
                    .wait_timeout_while(heap, delay, condition)
                    // .wait_while(heap, condition)
                    .expect("Queue lock poisoned");

                println!("wait timeout res = {:?}", cond.timed_out());
                Self::pop(heap)
            }
        } else {
            let guard = self
                .condvar
                .wait_while(heap, condition)
                .expect("Queue lock poisoned");
            Self::pop(guard)
        }
    }

    fn pop(mut mutex: MutexGuard<MinHeap<T>>) -> T {
        mutex.pop().unwrap().0
    }

    fn is_expired(e: &T) -> bool {
        e.delay() <= Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::thread::{sleep, Thread};
    use std::time::{Duration, Instant};

    use crate::blocking_delay_queue::{BlockingDelayQueue};
    use crate::delay_item::DelayItem;

    #[test]
    fn should_put_() {
        let q =  Arc::new(BlockingDelayQueue::new_unbounded());
        q.add(DelayItem {
            data: "111",
            delay: Instant::now() + Duration::from_millis(3_000),
        });

        let q_clone = q.clone();
        thread::Builder::new()
            .spawn(move || {
                sleep(Duration::from_millis(500));
                q_clone.add(DelayItem {
                    data: "222",
                    delay: Instant::now(),
                });
            }
            );


        let start = Instant::now();
        println!("take from q");
        let item = q.take();
        let end = Instant::now() - start;
        println!("{:?}", item);
        println!("duration {:?}", end);

    }
}