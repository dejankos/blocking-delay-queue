use blocking_delay_queue::{BlockingDelayQueue, DelayItem};
use criterion::{criterion_group, criterion_main, Criterion};
use std::time::{Duration, Instant};

fn add_and_take_bench(c: &mut Criterion) {
    let queue = BlockingDelayQueue::new_unbounded();
    c.bench_function("add element", |b| {
        b.iter(|| queue.add(DelayItem::new(Instant::now(), Instant::now())))
    });
    c.bench_function("take element", |b| b.iter(|| queue.take()));
}

fn offer_and_poll_bench(c: &mut Criterion) {
    let queue = BlockingDelayQueue::new_unbounded();
    let timeout = Duration::from_secs(1);
    c.bench_function("offer element", |b| {
        b.iter(|| queue.offer(DelayItem::new(Instant::now(), Instant::now()), timeout))
    });
    c.bench_function("poll element", |b| b.iter(|| queue.poll(timeout)));
}

criterion_group!(benches, add_and_take_bench, offer_and_poll_bench);
criterion_main!(benches);
