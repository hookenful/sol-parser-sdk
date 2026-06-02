use sol_parser_sdk::grpc::{ClientConfig, CommitmentMode};

#[test]
fn client_config_exposes_mutable_commitment_mode() {
    let mut config = ClientConfig::low_latency();

    assert_eq!(config.commitment, CommitmentMode::Processed);
    config.commitment = CommitmentMode::Confirmed;
    assert_eq!(config.commitment, CommitmentMode::Confirmed);
}
