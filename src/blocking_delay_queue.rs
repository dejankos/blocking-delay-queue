use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::delay_item::Delayed;

type MinHeap<T> = BinaryHeap<Reverse<T>>;

/// A blocking queue of [Delayed](delay_item::Delayed) elements in which an element can only be
/// taken when its delay has expired.
/// Supports adding and removing expired items by blocking until operation can be performed (['add'] / ['take'])
/// or by waiting util timeout (['offer'] / ['poll']).
///
/// #Examples
/// Basic usage:
/// ```
/// use std::time::Instant;
/// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
/// let bounded_q = BlockingDelayQueue::new_with_capacity(16);
/// bounded_q.add(DelayItem::new(123, Instant::now()));
/// let item = bounded_q.take();
/// println!("{}", item.data);
/// ```
pub struct BlockingDelayQueue<T> {
    heap: Mutex<MinHeap<T>>,
    condvar: Condvar,
    capacity: usize,
}

impl<T> BlockingDelayQueue<T>
where
    T: Delayed + Ord,
{
    /// Creates a new unbounded blocking delay queue.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::<DelayItem<&str>>::new_unbounded();
    /// ```
    pub fn new_unbounded() -> Self {
        BlockingDelayQueue {
            heap: Mutex::new(BinaryHeap::new()),
            condvar: Condvar::new(),
            capacity: 0,
        }
    }

    /// Creates a new bounded blocking delay queue with provided capacity where '0' is treated
    /// as unbounded.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::<DelayItem<&str>>::new_with_capacity(4);
    /// ```
    ///
    /// When capacity is zero queue will be unbounded:
    /// ```
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::<DelayItem<&str>>::new_with_capacity(0);
    /// ```
    pub fn new_with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            Self::new_unbounded()
        } else {
            BlockingDelayQueue {
                heap: Mutex::new(BinaryHeap::with_capacity(capacity)),
                condvar: Condvar::new(),
                capacity,
            }
        }
    }

    /// Adds an element to this queue waiting if necessary until space becomes available.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::Instant;
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::new_with_capacity(1);
    /// queue.add(DelayItem::new(123, Instant::now()));
    /// ```
    pub fn add(&self, e: T) {
        let mut heap = self.heap_mutex();
        if self.can_accept_element(&heap) {
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

    /// Adds an element to this queue waiting up to the specified wait time if necessary for space to become available.
    /// Returns 'true' if insertion was successful within specified wait time 'false' otherwise.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::{Duration, Instant};
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::new_with_capacity(1);
    /// queue.offer(DelayItem::new(123, Instant::now()), Duration::from_millis(5));
    /// ```
    pub fn offer(&self, e: T, timeout: Duration) -> bool {
        let mut heap = self.heap_mutex();
        if self.can_accept_element(&heap) {
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

    /// Retrieves and removes the head of this queue, waiting if necessary until an element with an expired delay is available on this queue.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::{Duration, Instant};
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::new_with_capacity(1);
    /// queue.add(DelayItem::new(123, Instant::now()));
    /// let item = queue.take();
    /// println!("{}", item.data);
    /// ```
    pub fn take(&self) -> T {
        match self.wait_for_element(Duration::ZERO) {
            Some(e) => e,
            _ => unreachable!(),
        }
    }

    /// Retrieves and removes the head of this queue, waiting if necessary until an element with an expired delay is available on this queue, or the specified wait time expires.
    /// Returns [None](core::option::Option::None) if no element is available within the specified wait time.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::{Duration, Instant};
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::new_with_capacity(1);
    /// queue.add(DelayItem::new(123, Instant::now()));
    /// let polled = queue.poll(Duration::from_secs(1));
    /// assert!(polled.is_some());
    /// println!("{}", polled.unwrap().data);
    /// ```
    pub fn poll(&self, timeout: Duration) -> Option<T> {
        self.wait_for_element(timeout)
    }

    /// Returns the number of elements in this queue.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::{Duration, Instant};
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::<DelayItem<&str>>::new_with_capacity(1);
    /// println!("{}", queue.size());
    /// ```
    pub fn size(&self) -> usize {
        self.heap_mutex().len()
    }

    /// Removes all of the elements from this queue.
    ///
    /// #Examples
    /// Basic usage:
    /// ```
    /// use std::time::{Duration, Instant};
    /// use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
    /// let  queue = BlockingDelayQueue::<DelayItem<&str>>::new_with_capacity(1);
    /// queue.clear();
    /// ```
    pub fn clear(&self) {
        let mut heap = self.heap_mutex();
        heap.clear();
        self.condvar.notify_all();
    }

    fn heap_mutex(&self) -> MutexGuard<'_, MinHeap<T>> {
        self.heap.lock().expect("Queue lock poisoned")
    }

    fn wait_for_element(&self, timeout: Duration) -> Option<T> {
        let heap = self.heap_mutex();
        if let Some(e) = heap.peek() {
            let current_time = Instant::now();
            if Self::is_expired(&e.0) {
                // remove head
                Some(self.pop_and_notify(heap))
            } else {
                let delay = match timeout {
                    // delay until head expiration
                    Duration::ZERO => e.0.delay() - current_time,
                    // delay until timeout
                    _ => timeout,
                };

                // wait until condition is satisfied respecting timeout (delay)
                match self.wait_for_element_with_timeout(heap, delay) {
                    // available element within timeout
                    Some(e) => Some(e),
                    _ => {
                        match timeout {
                            // unreachable code but let's keep it in case Q.remove(e) is added
                            Duration::ZERO => self.wait_for_element(timeout),
                            // when within timeout there is no available element
                            _ => None,
                        }
                    }
                }
            }
        } else {
            match timeout {
                Duration::ZERO => {
                    let guard = self
                        .condvar
                        .wait_while(heap, Self::wait_condition())
                        .expect("Condvar lock poisoned");

                    Some(self.pop_and_notify(guard))
                }
                _ => self.wait_for_element_with_timeout(heap, timeout),
            }
        }
    }

    fn wait_for_element_with_timeout(
        &self,
        heap: MutexGuard<MinHeap<T>>,
        timeout: Duration,
    ) -> Option<T> {
        let (heap, res) = self
            .condvar
            .wait_timeout_while(heap, timeout, Self::wait_condition())
            .expect("Condvar lock poisoned");

        match res.timed_out() {
            true => None,
            _ => Some(self.pop_and_notify(heap)),
        }
    }

    fn pop_and_notify(&self, mut mutex: MutexGuard<MinHeap<T>>) -> T {
        let e = mutex.pop().unwrap().0;
        self.condvar.notify_one();
        e
    }

    fn can_accept_element(&self, m: &MutexGuard<MinHeap<T>>) -> bool {
        if self.capacity == 0 {
            true
        } else {
            m.len() < m.capacity()
        }
    }

    fn wait_condition() -> impl Fn(&mut MinHeap<T>) -> bool {
        move |heap: &mut MinHeap<T>| {
            heap.peek()
                .map_or(true, |item| item.0.delay() > Instant::now())
        }
    }

    fn is_expired(e: &T) -> bool {
        e.delay() <= Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Sub;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::blocking_delay_queue::BlockingDelayQueue;
    use crate::delay_item::DelayItem;

    type MeasuredResult<T> = (T, Duration);

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
        queue.add(DelayItem::new(
            1,
            Instant::now() + Duration::from_millis(10),
        ));
        queue.add(DelayItem::new(2, Instant::now()));

        assert_eq!(2, queue.take().data);
        assert_eq!(1, queue.take().data);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_block_until_item_is_available_take() {
        let queue = Arc::new(BlockingDelayQueue::new_unbounded());
        let queue_rc = queue.clone();
        let handle = thread::spawn(move || queue_rc.take());
        queue.add(DelayItem::new(
            1,
            Instant::now() + Duration::from_millis(50),
        ));
        let res = handle.join().unwrap().data;
        assert_eq!(1, res);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_block_until_item_is_available_poll() {
        let queue = Arc::new(BlockingDelayQueue::new_unbounded());
        let queue_rc = queue.clone();
        let handle = thread::spawn(move || queue_rc.poll(Duration::from_millis(10)));
        queue.add(DelayItem::new(1, Instant::now() + Duration::from_millis(5)));
        let res = handle.join().unwrap().unwrap().data;
        assert_eq!(1, res);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_block_until_item_can_be_added() {
        let queue = Arc::new(BlockingDelayQueue::new_with_capacity(1));
        queue.add(DelayItem::new(
            1,
            Instant::now() + Duration::from_millis(50),
        ));
        let queue_rc = queue.clone();
        let handle = thread::spawn(move || queue_rc.add(DelayItem::new(2, Instant::now())));
        assert_eq!(1, queue.take().data);
        handle.join().unwrap();
        assert_eq!(1, queue.size());
        assert_eq!(2, queue.take().data);
    }

    #[test]
    fn should_timeout_if_element_cant_be_added() {
        let queue = BlockingDelayQueue::new_with_capacity(1);
        let accepted = queue.offer(DelayItem::new(1, Instant::now()), Duration::from_millis(5));
        // fill capacity
        assert!(accepted);

        // q is full here should block until timeout without inserting
        let timeout = Duration::from_millis(50);
        let res = measure_time_millis(|| queue.offer(DelayItem::new(2, Instant::now()), timeout));
        // element is not accepted - timeout occurred
        assert!(!res.0);
        // timeout is respected with some delta
        assert!(res.1 >= timeout && res.1.sub(timeout) <= Duration::from_millis(10));

        assert_eq!(1, queue.take().data);
        assert_eq!(0, queue.size());
    }

    #[test]
    fn should_timeout_if_element_cant_be_polled() {
        let queue: BlockingDelayQueue<DelayItem<u8>> = BlockingDelayQueue::new_unbounded();
        let e = queue.poll(Duration::from_millis(5));
        assert!(e.is_none());
    }

    #[test]
    fn should_clear_queue() {
        let queue = BlockingDelayQueue::new_unbounded();
        queue.add(DelayItem::new(1, Instant::now()));
        queue.add(DelayItem::new(2, Instant::now()));
        queue.clear();

        assert_eq!(0, queue.size());
    }

    fn measure_time_millis<T>(f: impl Fn() -> T) -> MeasuredResult<T> {
        let now = Instant::now();
        let t = f();
        (t, now.elapsed())
    }
}
