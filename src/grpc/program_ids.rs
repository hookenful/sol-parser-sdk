use crate::grpc::types::Protocol;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

// Program IDs for supported DEX protocols (string format)
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMPSWAP_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMPSWAP_FEES_PROGRAM_ID: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";
pub const BONK_PROGRAM_ID: &str = "BSwp6bEBihVLdqJRKS58NaebUBSDNjN7MdpFwNaR6gn3";
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUQtcaMpgYqJPXBDvfE";
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const METEORA_POOLS_PROGRAM_ID: &str = "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB";
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

// Program IDs (Pubkey format for matching)
pub const PUMPFUN_PROGRAM: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPSWAP_PROGRAM: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const BONK_PROGRAM: Pubkey = pubkey!("BSwp6bEBihVLdqJRKS58NaebUBSDNjN7MdpFwNaR6gn3");
pub const RAYDIUM_CPMM_PROGRAM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const RAYDIUM_CLMM_PROGRAM: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUQtcaMpgYqJPXBDvfE");
pub const RAYDIUM_AMM_V4_PROGRAM: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const ORCA_WHIRLPOOL_PROGRAM: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
pub const METEORA_POOLS_PROGRAM: Pubkey = pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");
pub const METEORA_DAMM_V2_PROGRAM: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
pub const METEORA_DLMM_PROGRAM: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");

lazy_static::lazy_static! {
    pub static ref PROTOCOL_PROGRAM_IDS: HashMap<Protocol, Vec<&'static str>> = {
        let mut map = HashMap::new();
        map.insert(Protocol::PumpFun, vec![PUMPFUN_PROGRAM_ID]);
        map.insert(Protocol::PumpSwap, vec![PUMPSWAP_PROGRAM_ID]);
        map.insert(Protocol::Bonk, vec![BONK_PROGRAM_ID]);
        map.insert(Protocol::RaydiumCpmm, vec![RAYDIUM_CPMM_PROGRAM_ID]);
        map.insert(Protocol::RaydiumClmm, vec![RAYDIUM_CLMM_PROGRAM_ID]);
        map.insert(Protocol::RaydiumAmmV4, vec![RAYDIUM_AMM_V4_PROGRAM_ID]);
        map.insert(Protocol::MeteoraDammV2, vec![METEORA_DAMM_V2_PROGRAM_ID]);
        // 移除不存在的协议，只保留有实际常量的协议
        map
    };
}

pub fn get_program_ids_for_protocols(protocols: &[Protocol]) -> Vec<String> {
    let mut program_ids = Vec::new();
    for protocol in protocols {
        if let Some(ids) = PROTOCOL_PROGRAM_IDS.get(protocol) {
            for id in ids {
                program_ids.push(id.to_string());
            }
        }
    }
    program_ids.sort();
    program_ids.dedup();
    program_ids
}
