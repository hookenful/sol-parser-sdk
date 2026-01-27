//! gRPC queue consumption benchmarks
//!
//! Compare:
//! - Busy-wait (spin + yield) on ArrayQueue
//! - Notify-based async wait on EventQueue
//!
//! Run with:
//!   cargo bench --bench grpc_notify_vs_spin

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use crossbeam_queue::ArrayQueue;
use sol_parser_sdk::grpc::EventQueue;
use sol_parser_sdk::DexEvent;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn make_event() -> DexEvent {
    DexEvent::PumpFunTrade(Default::default())
}

fn run_spin(count: usize) -> Duration {
    let queue = Arc::new(ArrayQueue::new(count * 2));
    let producer = queue.clone();
    let consumer = queue.clone();

    let start = Instant::now();
    let prod_handle = std::thread::spawn(move || {
        for _ in 0..count {
            let _ = producer.push(make_event());
        }
    });

    let cons_handle = std::thread::spawn(move || {
        let mut remaining = count;
        let mut spin_count = 0u32;
        while remaining > 0 {
            if consumer.pop().is_some() {
                remaining -= 1;
                spin_count = 0;
            } else {
                spin_count += 1;
                if spin_count < 1000 {
                    std::hint::spin_loop();
                } else {
                    std::thread::yield_now();
                    spin_count = 0;
                }
            }
        }
    });

    let _ = prod_handle.join();
    let _ = cons_handle.join();
    start.elapsed()
}

fn run_notify(rt: &Runtime, count: usize) -> Duration {
    let queue = Arc::new(EventQueue::new(count * 2));
    let producer = queue.clone();
    let consumer = queue.clone();

    let start = Instant::now();
    rt.block_on(async move {
        let prod = tokio::spawn(async move {
            for _ in 0..count {
                let _ = producer.push(make_event());
            }
        });

        let cons = tokio::spawn(async move {
            let mut remaining = count;
            while remaining > 0 {
                let _ = consumer.recv().await;
                remaining -= 1;
                while remaining > 0 {
                    if consumer.pop().is_some() {
                        remaining -= 1;
                    } else {
                        break;
                    }
                }
            }
        });

        let _ = tokio::join!(prod, cons);
    });
    start.elapsed()
}

fn bench_queue_consumption(c: &mut Criterion) {
    let mut group = c.benchmark_group("Queue Consumption");
    let rt = Runtime::new().expect("tokio runtime");

    for count in [1_000usize, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::new("Spin", count), &count, |b, &count| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += run_spin(count);
                }
                total
            });
        });

        group.bench_with_input(BenchmarkId::new("Notify", count), &count, |b, &count| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += run_notify(&rt, count);
                }
                total
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_queue_consumption);
criterion_main!(benches);
