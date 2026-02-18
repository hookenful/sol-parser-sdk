//! PumpFun instruction parser
//!
//! Parse PumpFun instructions using discriminator pattern matching

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// PumpFun discriminator constants
pub mod discriminators {
    /// Buy instruction: buy tokens with SOL
    pub const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
    /// Sell instruction: sell tokens for SOL
    pub const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
    /// Create instruction: create a new bonding curve
    pub const CREATE: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
    /// buy_exact_sol_in: Given a budget of spendable SOL, buy at least min_tokens_out
    pub const BUY_EXACT_SOL_IN: [u8; 8] = [56, 252, 116, 8, 158, 223, 205, 95];
    /// Migrate event log discriminator (CPI)
    pub const MIGRATE_EVENT_LOG: [u8; 8] = [189, 233, 93, 185, 92, 148, 234, 148];
    /// Migrate instruction discriminator (global:migrate)
    pub const MIGRATE: [u8; 8] = [155, 234, 231, 146, 236, 158, 162, 30];
}

/// PumpFun Program ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::PUMPFUN_PROGRAM_ID;

/// Main PumpFun instruction parser
///
/// Note: Full event data (amounts, fees, reserves) is parsed from logs.
/// Instruction parsing handles migrate instruction and MIGRATE_EVENT_LOG CPI.
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // BUY/SELL/CREATE events are parsed from logs for complete data
    // Parse migrate instruction (8-byte discriminator) and MIGRATE_EVENT_LOG (CPI).
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    if discriminator == discriminators::MIGRATE {
        return parse_migrate_instruction(
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }

    if instruction_data.len() < 16 {
        return None;
    }
    let cpi_discriminator: [u8; 8] = instruction_data[8..16].try_into().ok()?;
    if cpi_discriminator == discriminators::MIGRATE_EVENT_LOG {
        return parse_migrate_log_instruction(
            &instruction_data[16..],
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            grpc_recv_us,
        );
    }
    None
}

/// Parse buy/buy_exact_sol_in instruction
///
/// Account indices (from pump.json):
/// 0: global, 1: fee_recipient, 2: mint, 3: bonding_curve,
/// 4: associated_bonding_curve, 5: associated_user, 6: user
#[allow(dead_code)]
fn parse_buy_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if accounts.len() < 7 {
        return None;
    }

    // Parse args: amount/spendable_sol_in (u64), max_sol_cost/min_tokens_out (u64)
    let (sol_amount, token_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(accounts, 2)?;
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        is_buy: true,
        bonding_curve: get_account(accounts, 3).unwrap_or_default(),
        user: get_account(accounts, 6).unwrap_or_default(),
        sol_amount,
        token_amount,
        fee_recipient: get_account(accounts, 1).unwrap_or_default(),
        ..Default::default()
    }))
}

/// Parse sell instruction
///
/// Account indices (from pump.json):
/// 0: global, 1: fee_recipient, 2: mint, 3: bonding_curve,
/// 4: associated_bonding_curve, 5: associated_user, 6: user
#[allow(dead_code)]
fn parse_sell_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if accounts.len() < 7 {
        return None;
    }

    // Parse args: amount (u64), min_sol_output (u64)
    let (token_amount, sol_amount) = if data.len() >= 16 {
        (read_u64_le(data, 0).unwrap_or(0), read_u64_le(data, 8).unwrap_or(0))
    } else {
        (0, 0)
    };

    let mint = get_account(accounts, 2)?;
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunTrade(PumpFunTradeEvent {
        metadata,
        mint,
        is_buy: false,
        bonding_curve: get_account(accounts, 3).unwrap_or_default(),
        user: get_account(accounts, 6).unwrap_or_default(),
        sol_amount,
        token_amount,
        fee_recipient: get_account(accounts, 1).unwrap_or_default(),
        ..Default::default()
    }))
}

/// Parse create instruction
///
/// Account indices (from pump.json):
/// 0: mint, 1: mint_authority, 2: bonding_curve, 3: associated_bonding_curve,
/// 4: global, 5: mpl_token_metadata, 6: metadata, 7: user
#[allow(dead_code)]
fn parse_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    if accounts.len() < 8 {
        return None;
    }

    let mut offset = 0;

    // Parse args: name (string), symbol (string), uri (string), creator (pubkey)
    // String format: 4-byte length prefix + content
    let name = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    let symbol = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    let uri = if let Some((s, len)) = read_str_unchecked(data, offset) {
        offset += len;
        s.to_string()
    } else {
        String::new()
    };

    let creator = if offset + 32 <= data.len() {
        read_pubkey(data, offset).unwrap_or_default()
    } else {
        Pubkey::default()
    };

    let mint = get_account(accounts, 0)?;
    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);

    Some(DexEvent::PumpFunCreate(PumpFunCreateTokenEvent {
        metadata,
        name,
        symbol,
        uri,
        mint,
        bonding_curve: get_account(accounts, 2).unwrap_or_default(),
        user: get_account(accounts, 7).unwrap_or_default(),
        creator,
        ..Default::default()
    }))
}

/// Parse Migrate CPI instruction
#[allow(unused_variables)]
fn parse_migrate_log_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    rpc_recv_us: i64,
) -> Option<DexEvent> {
    let mut offset = 0;

    // user (Pubkey - 32 bytes)
    let user = read_pubkey(data, offset)?;
    offset += 32;

    // mint (Pubkey - 32 bytes)
    let mint = read_pubkey(data, offset)?;
    offset += 32;

    // mintAmount (u64 - 8 bytes)
    let mint_amount = read_u64_le(data, offset)?;
    offset += 8;

    // solAmount (u64 - 8 bytes)
    let sol_amount = read_u64_le(data, offset)?;
    offset += 8;

    // poolMigrationFee (u64 - 8 bytes)
    let pool_migration_fee = read_u64_le(data, offset)?;
    offset += 8;

    // bondingCurve (Pubkey - 32 bytes)
    let bonding_curve = read_pubkey(data, offset)?;
    offset += 32;

    // timestamp (i64 - 8 bytes)
    let timestamp = read_u64_le(data, offset)? as i64;
    offset += 8;

    // pool (Pubkey - 32 bytes)
    let pool = read_pubkey(data, offset)?;

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), rpc_recv_us);

    Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
        metadata,
        user,
        mint,
        mint_amount,
        sol_amount,
        pool_migration_fee,
        bonding_curve,
        timestamp,
        pool,
    }))
}

/// Parse migrate instruction (global:migrate)
fn parse_migrate_instruction(
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    grpc_recv_us: i64,
) -> Option<DexEvent> {
    // Need at least up to pool account per IDL.
    if accounts.len() < 10 {
        return None;
    }

    let metadata =
        create_metadata(signature, slot, tx_index, block_time_us.unwrap_or_default(), grpc_recv_us);
    let timestamp = block_time_us.unwrap_or_default().saturating_div(1_000_000);

    Some(DexEvent::PumpFunMigrate(PumpFunMigrateEvent {
        metadata,
        user: get_account(accounts, 5).unwrap_or_default(),
        mint: get_account(accounts, 2).unwrap_or_default(),
        mint_amount: 0,
        sol_amount: 0,
        pool_migration_fee: 0,
        bonding_curve: get_account(accounts, 3).unwrap_or_default(),
        timestamp,
        pool: get_account(accounts, 9).unwrap_or_default(),
    }))
}
