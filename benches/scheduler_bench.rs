use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

// Mock Scheduler for Benchmarking
struct MockScheduler {
    tasks: Vec<u32>,
}

impl MockScheduler {
    fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    fn enqueue(&mut self, task_id: u32) {
        self.tasks.push(task_id);
    }

    fn dispatch(&mut self) -> Option<u32> {
        self.tasks.pop()
    }
}

fn benchmark_scheduler_enqueue(c: &mut Criterion) {
    let mut scheduler = MockScheduler::new();
    
    c.bench_function("scheduler_enqueue", |b| {
        b.iter(|| {
            scheduler.enqueue(black_box(1));
        })
    });
}

fn benchmark_scheduler_dispatch(c: &mut Criterion) {
    let mut scheduler = MockScheduler::new();
    // Pre-fill
    for i in 0..1000 {
        scheduler.enqueue(i);
    }

    c.bench_function("scheduler_dispatch", |b| {
        b.iter(|| {
            scheduler.dispatch();
        })
    });
}

criterion_group!(benches, benchmark_scheduler_enqueue, benchmark_scheduler_dispatch);
criterion_main!(benches);
