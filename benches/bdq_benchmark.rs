use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::{Duration, Instant};

fn add_and_take_bench(c: &mut Criterion) {
    let queue = BlockingDelayQueue::new_unbounded();
    c.bench_function("add element", |b| {
        b.iter(|| queue.add(DelayItem::new(Instant::now(), Instant::now())))
    });
    c.bench_function("take element", |b| b.iter(|| queue.take()));
}

criterion_group!(benches, add_and_take_bench);
criterion_main!(benches);
