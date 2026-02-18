use sol_parser_sdk::grpc::{
    AccountFilter, ClientConfig, Protocol, TransactionFilter, YellowstoneGrpc,
};
use sol_parser_sdk::DexEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    println!("🚀 Quick Test - Subscribing to ALL PumpFun events...");

    let mut config: ClientConfig = ClientConfig::default();
    config.enable_metrics = true;

    let grpc = YellowstoneGrpc::new_with_config(
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string(),
        None,
        config,
    )?;

    let protocols = vec![Protocol::PumpFun];
    let transaction_filter = TransactionFilter::for_protocols(&protocols);
    let account_filter = AccountFilter::for_protocols(&protocols);

    println!("✅ Subscribing... (no event filter - will show ALL events)");

    // 无过滤器 - 订阅所有事件
    let queue = grpc
        .subscribe_dex_events(
            vec![transaction_filter],
            vec![account_filter],
            None, // 无过滤 - 所有事件都会显示
        )
        .await?;

    println!("🎧 Listening for events... (waiting up to 60 seconds)\n");

    let mut event_count = 0;
    let start = std::time::Instant::now();

    // 简单循环，打印前 10 个事件
    loop {
        if let Some(event) = queue.pop() {
            event_count += 1;
            let event_type = match &event {
                DexEvent::PumpFunCreate(_) => "PumpFunCreate",
                DexEvent::PumpFunTrade(_) => "PumpFunTrade",
                DexEvent::PumpFunBuy(_) => "PumpFunBuy",
                DexEvent::PumpFunSell(_) => "PumpFunSell",
                DexEvent::PumpFunMigrate(_) => "PumpFunMigrate",
                _ => "Other",
            };

            println!("✅ Event #{}: {} (Queue: {})", event_count, event_type, queue.len());

            if event_count >= 10 {
                println!("\n🎉 Received {} events! Test successful!", event_count);
                break;
            }
        } else {
            tokio::task::yield_now().await;
        }

        // 60 秒超时
        if start.elapsed().as_secs() > 60 {
            if event_count == 0 {
                println!("⏰ Timeout: No events received in 60 seconds.");
                println!("   This might indicate:");
                println!("   - Network connectivity issues");
                println!("   - gRPC endpoint is down");
                println!("   - Very low market activity (rare)");
            } else {
                println!("\n✅ Received {} events in 60 seconds", event_count);
            }
            break;
        }
    }

    Ok(())
}
