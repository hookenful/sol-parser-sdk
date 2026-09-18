use std::time::Duration;

use std::sync::Arc;

use sol_parser_sdk::grpc::{TxDeduper, YellowstoneGrpc, YellowstoneGrpcClient};
use solana_sdk::signature::Signature;

#[test]
fn tx_deduper_rejects_duplicate_signature_in_same_slot_within_ttl() {
    let deduper = TxDeduper::new(Duration::from_secs(5), None);
    let sig = Signature::new_unique();

    assert!(deduper.check(sig, 42, 1_000_000));
    assert!(!deduper.check(sig, 42, 1_000_001));
}

#[test]
fn tx_deduper_allows_same_signature_after_ttl_or_different_slot() {
    let deduper = TxDeduper::new(Duration::from_secs(5), Some("test".to_string()));
    let sig = Signature::new_unique();

    assert!(deduper.check(sig, 42, 1_000_000));
    assert!(deduper.check(sig, 43, 1_000_001));
    assert!(deduper.check(sig, 42, 6_000_000));
    assert_eq!(deduper.log_label(), Some("test"));
}

#[test]
fn yellowstone_client_accepts_shared_tx_deduper() {
    fn attach_deduper(client: YellowstoneGrpc, deduper: Arc<TxDeduper>) -> YellowstoneGrpc {
        client.with_tx_deduper(deduper)
    }

    let _api_check: fn(YellowstoneGrpc, Arc<TxDeduper>) -> YellowstoneGrpc = attach_deduper;
}

#[test]
fn yellowstone_grpc_client_alias_remains_available() {
    let _client_type_check: Option<YellowstoneGrpcClient> = None;
}
