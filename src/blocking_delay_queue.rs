use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Debug;
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
    where T: Delayed + Ord + Debug
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
                .wait_timeout_while(heap, timeout, |heap| heap.len() >= cap)
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
        self.take_inner()
    }

    fn poll(&self, timeout: Duration) -> Option<T> {
        let heap = self.heap_mutex();

        if let Some(e) = self.wait_with_timeout_for_element(heap, timeout) {
            self.condvar.notify_one();
            Some(e)
        } else {
            None
        }
    }

    fn size(&self) -> usize {
        self.heap_mutex().len()
    }

    fn clear(&self) {
        self.heap_mutex().clear();
    }

    fn take_inner(&self) -> T {
        self.wait_for_element()
    }

    fn wait_for_element(&self) -> T {
        let heap = self.heap_mutex();
        if let Some(e) = heap.peek() {
            let current_time = Instant::now();
            if Self::is_expired(&e.0) {
                // remove head
                self.pop_and_notify(heap)
            } else {
                // delay until head expiration
                let delay = e.0.delay() - current_time;

                // wait until head expires or new head wakes this thread
                let opt = self.wait_with_timeout_for_element(heap, delay);

                match opt {
                    // should always be some
                    Some(e) => e,
                    // unreachable code but let's keep it in case Q.remove(e) is added
                    _ => self.wait_for_element()
                }
            }
        } else {
            // wait to be notified until condition is met
            let guard = self
                .condvar
                .wait_while(heap, Self::wait_condition())
                .expect("Condvar lock poisoned");
            self.pop_and_notify(guard)
        }
    }

    fn wait_with_timeout_for_element(&self, heap: MutexGuard<MinHeap<T>>, timeout: Duration) -> Option<T> {
        let (heap, res) = self
            .condvar
            .wait_timeout_while(heap, timeout, Self::wait_condition())
            .expect("Condvar lock poisoned");

        match res.timed_out() {
            true => None,
            _ => Some(self.pop_and_notify(heap))
        }
    }

    fn wait_condition() -> impl Fn(&mut MinHeap<T>) -> bool {
        move |heap: &mut MinHeap<T>| {
            heap.peek()
                .map_or(true, |item| item.0.delay() > Instant::now())
        }
    }

    fn pop_and_notify(&self, mut mutex: MutexGuard<MinHeap<T>>) -> T {
        let e = mutex.pop().unwrap().0;
        self.condvar.notify_one();
        e
    }

    fn is_expired(e: &T) -> bool {
        e.delay() <= Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::blocking_delay_queue::BlockingDelayQueue;
    use crate::delay_item::DelayItem;

    #[test]
    fn should_put_and_take_ordered() {
        let queue = BlockingDelayQueue::new_unbounded();
        queue.add(DelayItem::new(1, Instant::now()));
        queue.add(DelayItem::new(2, Instant::now()));

        assert_eq!(1, queue.take().data);
        assert_eq!(2, queue.take().data);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_put_and_take_delayed_items() {
        let queue = BlockingDelayQueue::new_unbounded();
        queue.add(DelayItem::new(1, Instant::now() + Duration::from_millis(10)));
        queue.add(DelayItem::new(2, Instant::now()));

        assert_eq!(2, queue.take().data);
        assert_eq!(1, queue.take().data);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_block_until_item_is_available() {
        let queue = Arc::new(BlockingDelayQueue::new_unbounded());
        let queue_rc = queue.clone();
        let handle = thread::spawn(move || queue_rc.take());
        queue.add(DelayItem::new(1, Instant::now() + Duration::from_millis(50)));
        let res = handle.join().unwrap().data;
        assert_eq!(1, res);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_block_until_item_can_be_added() {
        let queue = Arc::new(BlockingDelayQueue::new_with_capacity(1));
        queue.add(DelayItem::new(1, Instant::now() + Duration::from_millis(50)));
        let queue_rc = queue.clone();
        let handle = thread::spawn(move || queue_rc.add(DelayItem::new(2, Instant::now())));
        assert_eq!(1, queue.take().data);
        handle.join().unwrap();
        assert_eq!(1, queue.size());
        assert_eq!(2, queue.take().data);
    }
}