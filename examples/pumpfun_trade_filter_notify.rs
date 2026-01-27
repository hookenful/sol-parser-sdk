//! Non-blocking Notify example (no busy wait)
//! - Subscribe to PumpFun protocol events
//! - Use EventQueue::recv() for async waiting

use sol_parser_sdk::core::now_micros; // Use SDK high-performance clock
use sol_parser_sdk::grpc::{AccountFilter, EventType, EventTypeFilter, Protocol, TransactionFilter, YellowstoneGrpc};
use sol_parser_sdk::DexEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    run_example().await
}

struct Stats {
    event_count: u64,
    buy_count: u64,
    sell_count: u64,
    buy_exact_count: u64,
    create_count: u64,
    total_latency_us: i64,
}

impl Stats {
    fn new() -> Self {
        Self {
            event_count: 0,
            buy_count: 0,
            sell_count: 0,
            buy_exact_count: 0,
            create_count: 0,
            total_latency_us: 0,
        }
    }
}

fn handle_event(event: DexEvent, stats: &mut Stats) {
    stats.event_count += 1;
    let now_us = now_micros();

    match &event {
        DexEvent::PumpFunBuy(e) => {
            stats.buy_count += 1;
            let latency_us = now_us - e.metadata.grpc_recv_us;
            stats.total_latency_us += latency_us;

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ PumpFun BUY #{}", stats.event_count);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Signature  : {}", e.metadata.signature);
            println!("│ Slot       : {} | TxIndex: {}", e.metadata.slot, e.metadata.tx_index);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Mint       : {}", e.mint);
            println!("│ SOL Amount : {} lamports", e.sol_amount);
            println!("│ Token Amt  : {}", e.token_amount);
            println!("│ User       : {}", e.user);
            println!("│ ix_name    : {}", e.ix_name);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Latency    : {} us", latency_us);
            println!("│ Stats      : Buy={} Sell={} BuyExact={}", stats.buy_count, stats.sell_count, stats.buy_exact_count);
            println!("└─────────────────────────────────────────────────────────────\n");
        }

        DexEvent::PumpFunSell(e) => {
            stats.sell_count += 1;
            let latency_us = now_us - e.metadata.grpc_recv_us;
            stats.total_latency_us += latency_us;

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ PumpFun SELL #{}", stats.event_count);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Signature  : {}", e.metadata.signature);
            println!("│ Slot       : {} | TxIndex: {}", e.metadata.slot, e.metadata.tx_index);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Mint       : {}", e.mint);
            println!("│ SOL Amount : {} lamports", e.sol_amount);
            println!("│ Token Amt  : {}", e.token_amount);
            println!("│ User       : {}", e.user);
            println!("│ ix_name    : {}", e.ix_name);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Latency    : {} us", latency_us);
            println!("│ Stats      : Buy={} Sell={} BuyExact={}", stats.buy_count, stats.sell_count, stats.buy_exact_count);
            println!("└─────────────────────────────────────────────────────────────\n");
        }

        DexEvent::PumpFunBuyExactSolIn(e) => {
            stats.buy_exact_count += 1;
            let latency_us = now_us - e.metadata.grpc_recv_us;
            stats.total_latency_us += latency_us;

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ PumpFun BUY_EXACT_SOL_IN #{}", stats.event_count);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Signature  : {}", e.metadata.signature);
            println!("│ Slot       : {} | TxIndex: {}", e.metadata.slot, e.metadata.tx_index);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Mint       : {}", e.mint);
            println!("│ SOL Amount : {} lamports (exact input)", e.sol_amount);
            println!("│ Token Amt  : {} (min output)", e.token_amount);
            println!("│ User       : {}", e.user);
            println!("│ ix_name    : {}", e.ix_name);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Latency    : {} us", latency_us);
            println!("│ Stats      : Buy={} Sell={} BuyExact={}", stats.buy_count, stats.sell_count, stats.buy_exact_count);
            println!("└─────────────────────────────────────────────────────────────\n");
        }

        DexEvent::PumpFunTrade(e) => {
            let latency_us = now_us - e.metadata.grpc_recv_us;
            stats.total_latency_us += latency_us;

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ PumpFun TRADE (unknown type) #{}", stats.event_count);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ ix_name    : {} (is_buy={})", e.ix_name, e.is_buy);
            println!("│ Signature  : {}", e.metadata.signature);
            println!("└─────────────────────────────────────────────────────────────\n");
        }

        DexEvent::PumpFunCreate(e) => {
            stats.create_count += 1;
            let latency_us = now_us - e.metadata.grpc_recv_us;
            stats.total_latency_us += latency_us;

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ PumpFun CREATE #{}", stats.event_count);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Signature  : {}", e.metadata.signature);
            println!("│ Slot       : {} | TxIndex: {}", e.metadata.slot, e.metadata.tx_index);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Name       : {}", e.name);
            println!("│ Symbol     : {}", e.symbol);
            println!("│ Mint       : {}", e.mint);
            println!("│ Creator    : {}", e.creator);
            println!("├─────────────────────────────────────────────────────────────");
            println!("│ Latency    : {} us", latency_us);
            println!("│ Creates    : {}", stats.create_count);
            println!("└─────────────────────────────────────────────────────────────\n");
        }

        _ => {}
    }
}

async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Subscribing to Yellowstone gRPC events (Notify mode)...");

    let grpc = YellowstoneGrpc::new(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
    )?;

    let protocols = vec![Protocol::PumpFun];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    let event_filter = EventTypeFilter::include_only(vec![
        EventType::PumpFunBuy,
        EventType::PumpFunSell,
        EventType::PumpFunBuyExactSolIn,
        EventType::PumpFunCreate,
    ]);

    println!("Event Filter: Buy, Sell, BuyExactSolIn, Create");
    println!("Starting subscription...\n");

    let queue = grpc
        .subscribe_dex_events_notify(vec![transaction_filter], vec![account_filter], Some(event_filter))
        .await?;

    // Non-blocking Notify consumer (no busy wait)
    tokio::spawn(async move {
        let mut stats = Stats::new();
        loop {
            let event = queue.recv().await;
            handle_event(event, &mut stats);

            // Drain backlog quickly after wake-up
            while let Some(event) = queue.pop() {
                handle_event(event, &mut stats);
            }
        }
    });

    println!("Press Ctrl+C to stop...\n");
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down gracefully...");

    Ok(())
}
