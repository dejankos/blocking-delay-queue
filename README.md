# Blocking delay queue

A thread safe blocking delay queue.

## Example
```rust
use std::time::{Duration, Instant};

use blocking_delay_queue::{BlockingDelayQueue, DelayItem};

fn main() {
// bounded queue
let queue = BlockingDelayQueue::new_with_capacity(16);
// there is also an unbounded impl -> BlockingDelayQueue::new_unbounded()

    // add element - blocks until item can be added respecting queue capacity
    queue.add(DelayItem::new(123, Instant::now()));
    // offer element - blocks until item can be added respecting queue capacity or the specified wait time expires
    let success = queue.offer(DelayItem::new(456, Instant::now()), Duration::from_secs(1));

    // take element - removes the head of this queue, waiting until an element is available
    let take = queue.take();
    // poll element - removes the head of this queue, waiting until an element is available or the specified wait time expires
    let poll = queue.poll(Duration::from_secs(1));

    println!("Offering element status {}", success);
    println!("First element data {}", take.data);
    println!("Second element data {}", poll.unwrap().data);
    println!("Queue size {}", queue.size());
}
```
## Benchmark


## License

blocking-delay-queue is licensed under the [MIT License](https://opensource.org/licenses/MIT)