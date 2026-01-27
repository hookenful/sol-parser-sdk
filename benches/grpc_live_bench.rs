//! Live gRPC benchmark (real Yellowstone stream)
//!
//! Run with:
//!   GRPC_ENDPOINT=... MODE=notify SECONDS=60 cargo bench --bench grpc_live_bench
//!
//! Modes:
//!   MODE=notify | spin
//! Order modes:
//!   ORDER_MODE=unordered | ordered | streaming | microbatch
//!
//! Optional:
//!   REPORT_SECS=10

use sol_parser_sdk::core::now_micros;
use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, EventType, EventTypeFilter, OrderMode, Protocol, TransactionFilter,
    YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchMode {
    Spin,
    Notify,
}

struct Stats {
    event_count: AtomicU64,
    total_latency_us: AtomicU64,
    min_latency_us: AtomicU64,
    max_latency_us: AtomicU64,
    spin_ticks: AtomicU64,
    wakeups: AtomicU64,
}

impl Stats {
    fn new() -> Self {
        Self {
            event_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
            spin_ticks: AtomicU64::new(0),
            wakeups: AtomicU64::new(0),
        }
    }
}

fn update_min_max(min: &AtomicU64, max: &AtomicU64, value: u64) {
    let mut current_min = min.load(Ordering::Relaxed);
    while value < current_min {
        match min.compare_exchange(current_min, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current_min = x,
        }
    }

    let mut current_max = max.load(Ordering::Relaxed);
    while value > current_max {
        match max.compare_exchange(current_max, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current_max = x,
        }
    }
}

fn parse_mode(value: &str) -> BenchMode {
    match value.to_ascii_lowercase().as_str() {
        "spin" => BenchMode::Spin,
        _ => BenchMode::Notify,
    }
}

fn parse_order_mode(value: &str) -> OrderMode {
    match value.to_ascii_lowercase().as_str() {
        "ordered" => OrderMode::Ordered,
        "streaming" | "streamingordered" => OrderMode::StreamingOrdered,
        "microbatch" | "micro" => OrderMode::MicroBatch,
        _ => OrderMode::Unordered,
    }
}

fn extract_grpc_recv_us(event: &DexEvent) -> Option<i64> {
    match event {
        DexEvent::PumpFunTrade(e) => Some(e.metadata.grpc_recv_us),
        DexEvent::PumpFunBuy(e) => Some(e.metadata.grpc_recv_us),
        DexEvent::PumpFunSell(e) => Some(e.metadata.grpc_recv_us),
        DexEvent::PumpFunBuyExactSolIn(e) => Some(e.metadata.grpc_recv_us),
        DexEvent::PumpFunCreate(e) => Some(e.metadata.grpc_recv_us),
        DexEvent::PumpFunMigrate(e) => Some(e.metadata.grpc_recv_us),
        _ => None,
    }
}

fn record_event(stats: &Stats, event: &DexEvent) {
    let Some(grpc_recv_us) = extract_grpc_recv_us(event) else { return };
    let now_us = now_micros();
    let latency_us = (now_us - grpc_recv_us) as u64;
    stats.event_count.fetch_add(1, Ordering::Relaxed);
    stats.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    update_min_max(&stats.min_latency_us, &stats.max_latency_us, latency_us);
}

async fn report_loop_array(
    queue: Arc<crossbeam_queue::ArrayQueue<DexEvent>>,
    stats: Arc<Stats>,
    stop: Arc<AtomicBool>,
    interval: Duration,
) {
    let mut last_count = 0u64;
    let start = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        tokio::time::sleep(interval).await;
        let count = stats.event_count.load(Ordering::Relaxed);
        let total = stats.total_latency_us.load(Ordering::Relaxed);
        let min = stats.min_latency_us.load(Ordering::Relaxed);
        let max = stats.max_latency_us.load(Ordering::Relaxed);
        let spins = stats.spin_ticks.swap(0, Ordering::Relaxed);
        let avg = if count > 0 { total / count } else { 0 };
        let delta = count - last_count;
        let rate = delta as f64 / interval.as_secs_f64();
        let elapsed = start.elapsed().as_secs();
        let qlen = queue.len();

        println!(
            "[{}s] events={} rate={:.1}/s avg={}us min={}us max={}us queue={} spin_ticks/s={}",
            elapsed,
            count,
            rate,
            avg,
            if min == u64::MAX { 0 } else { min },
            max,
            qlen,
            spins
        );

        last_count = count;
    }
}

async fn report_loop_notify(
    queue: Arc<sol_parser_sdk::grpc::EventQueue>,
    stats: Arc<Stats>,
    stop: Arc<AtomicBool>,
    interval: Duration,
) {
    let mut last_count = 0u64;
    let start = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        tokio::time::sleep(interval).await;
        let count = stats.event_count.load(Ordering::Relaxed);
        let total = stats.total_latency_us.load(Ordering::Relaxed);
        let min = stats.min_latency_us.load(Ordering::Relaxed);
        let max = stats.max_latency_us.load(Ordering::Relaxed);
        let wakeups = stats.wakeups.swap(0, Ordering::Relaxed);
        let avg = if count > 0 { total / count } else { 0 };
        let delta = count - last_count;
        let rate = delta as f64 / interval.as_secs_f64();
        let elapsed = start.elapsed().as_secs();
        let qlen = queue.len();

        println!(
            "[{}s] events={} rate={:.1}/s avg={}us min={}us max={}us queue={} wakeups/s={}",
            elapsed,
            count,
            rate,
            avg,
            if min == u64::MAX { 0 } else { min },
            max,
            qlen,
            wakeups
        );

        last_count = count;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let endpoint = std::env::var("GRPC_ENDPOINT")
        .unwrap_or_else(|_| "https://solana-yellowstone-grpc.publicnode.com:443".to_string());
    let mode = parse_mode(&std::env::var("MODE").unwrap_or_else(|_| "notify".to_string()));
    let seconds: u64 = std::env::var("SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    let report_secs: u64 = std::env::var("REPORT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let order_mode = std::env::var("ORDER_MODE")
        .ok()
        .map(|v| parse_order_mode(&v))
        .unwrap_or(OrderMode::Unordered);

    let mut config = ClientConfig::default();
    config.order_mode = order_mode;

    println!("gRPC live benchmark");
    println!("endpoint      : {}", endpoint);
    println!("mode          : {:?}", mode);
    println!("order_mode    : {:?}", config.order_mode);
    println!("duration_secs : {}", seconds);
    println!("report_secs   : {}", report_secs);
    println!("event_filter  : PumpFunTrade");

    let grpc = YellowstoneGrpc::new_with_config(endpoint, None, config)?;

    let protocols = vec![Protocol::PumpFun];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);
    let event_filter = EventTypeFilter::include_only(vec![EventType::PumpFunTrade]);

    let stats = Arc::new(Stats::new());
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = stop.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(seconds)).await;
        stop_signal.store(true, Ordering::Relaxed);
    });

    match mode {
        BenchMode::Spin => {
            let queue = grpc
                .subscribe_dex_events(vec![transaction_filter], vec![account_filter], Some(event_filter))
                .await?;

            let stats_consumer = stats.clone();
            let stop_consumer = stop.clone();
            let queue_consumer = queue.clone();
            let consumer = tokio::spawn(async move {
                let mut spin_count = 0u32;
                while !stop_consumer.load(Ordering::Relaxed) {
                    if let Some(event) = queue_consumer.pop() {
                        record_event(&stats_consumer, &event);
                        spin_count = 0;
                    } else {
                        spin_count += 1;
                        stats_consumer.spin_ticks.fetch_add(1, Ordering::Relaxed);
                        if spin_count < 1000 {
                            std::hint::spin_loop();
                        } else {
                            tokio::task::yield_now().await;
                            spin_count = 0;
                        }
                    }
                }
            });

            let reporter = tokio::spawn(report_loop_array(
                queue,
                stats.clone(),
                stop.clone(),
                Duration::from_secs(report_secs),
            ));

            let _ = tokio::join!(consumer, reporter);
        }
        BenchMode::Notify => {
            let queue = grpc
                .subscribe_dex_events_notify(vec![transaction_filter], vec![account_filter], Some(event_filter))
                .await?;

            let stats_consumer = stats.clone();
            let stop_consumer = stop.clone();
            let queue_consumer = queue.clone();
            let consumer = tokio::spawn(async move {
                while !stop_consumer.load(Ordering::Relaxed) {
                    if let Some(event) = queue_consumer.recv_timeout(Duration::from_millis(200)).await {
                        stats_consumer.wakeups.fetch_add(1, Ordering::Relaxed);
                        record_event(&stats_consumer, &event);
                        while let Some(event) = queue_consumer.pop() {
                            record_event(&stats_consumer, &event);
                        }
                    }
                }
            });

            let reporter = tokio::spawn(report_loop_notify(
                queue,
                stats.clone(),
                stop.clone(),
                Duration::from_secs(report_secs),
            ));

            let _ = tokio::join!(consumer, reporter);
        }
    }

    grpc.stop().await;

    let count = stats.event_count.load(Ordering::Relaxed);
    let total = stats.total_latency_us.load(Ordering::Relaxed);
    let min = stats.min_latency_us.load(Ordering::Relaxed);
    let max = stats.max_latency_us.load(Ordering::Relaxed);
    let avg = if count > 0 { total / count } else { 0 };

    println!("summary: events={} avg={}us min={}us max={}us", count, avg, if min == u64::MAX { 0 } else { min }, max);
    Ok(())
}
