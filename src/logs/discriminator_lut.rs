//! Discriminator Lookup Table (LUT) - Compile-time constant array
//!
//! Zero-latency optimization: Use const array with binary search for O(log n) discriminator -> event type mapping
//! Expected latency reduction: 1-10ns (binary search on sorted array, better cache locality than match)

use crate::core::events::{DexEvent, EventMetadata};
use crate::grpc::types::EventType;

/// Discriminator type alias for clarity
pub type Discriminator = u64;

/// Parser function type - takes decoded data and metadata, returns parsed event
pub type ParserFn = fn(&[u8], EventMetadata) -> Option<DexEvent>;

/// Event metadata for discriminator lookup
#[derive(Debug, Clone, Copy)]
pub struct DiscriminatorInfo {
    pub discriminator: u64,
    pub parser: ParserFn,
    pub protocol: Protocol,
    pub name: &'static str, // Human-readable name for debugging
}

/// Protocol enum for quick protocol identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    PumpFun,
    PumpSwap,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumAmm,
    OrcaWhirlpool,
    MeteoraAmm,
    MeteoraDamm,
    MeteoraDlmm,
}

// ============================================================================
// Parser function wrappers - delegate to protocol-specific parsers
// ============================================================================

// PumpFun parsers
#[inline(always)]
fn parse_pumpfun_create(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_create_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpfun_trade(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_trade_from_data(data, metadata, false)
}

#[inline(always)]
fn parse_pumpfun_migrate(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump::parse_migrate_from_data(data, metadata)
}

// PumpSwap parsers
#[inline(always)]
fn parse_pumpswap_buy(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_buy_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_sell(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_sell_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_add_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_pumpswap_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::pump_amm::parse_remove_liquidity_from_data(data, metadata)
}

// Raydium CLMM parsers
#[inline(always)]
fn parse_raydium_clmm_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_swap_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_increase_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_increase_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_decrease_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_decrease_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_clmm_collect_fee(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_clmm::parse_collect_fee_from_data(data, metadata)
}

// Raydium CPMM parsers
#[inline(always)]
fn parse_raydium_cpmm_swap_base_in(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_swap_base_in_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_swap_base_out(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_swap_base_out_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_create_pool(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_create_pool_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_deposit_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_cpmm_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_cpmm::parse_withdraw_from_data(data, metadata)
}

// Raydium AMM V4 parsers
#[inline(always)]
fn parse_raydium_amm_swap_base_in(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_swap_base_in_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_swap_base_out(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_swap_base_out_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_deposit(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_deposit_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_withdraw(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_withdraw_from_data(data, metadata)
}

#[inline(always)]
fn parse_raydium_amm_initialize2(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::raydium_amm::parse_initialize2_from_data(data, metadata)
}

// Orca Whirlpool parsers
#[inline(always)]
fn parse_orca_traded(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_traded_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_liquidity_increased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_liquidity_increased_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_liquidity_decreased(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_liquidity_decreased_from_data(data, metadata)
}

#[inline(always)]
fn parse_orca_pool_initialized(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::orca_whirlpool::parse_pool_initialized_from_data(data, metadata)
}

// Meteora AMM parsers
#[inline(always)]
fn parse_meteora_amm_swap(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_swap_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_add_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_add_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_remove_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_remove_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_bootstrap_liquidity(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_bootstrap_liquidity_from_data(data, metadata)
}

#[inline(always)]
fn parse_meteora_amm_pool_created(data: &[u8], metadata: EventMetadata) -> Option<DexEvent> {
    crate::logs::meteora_amm::parse_pool_created_from_data(data, metadata)
}

// ============================================================================
// Const lookup table - Sorted by discriminator for binary search
// ============================================================================

macro_rules! disc_entry {
    ($disc:expr, $name:expr, $parser:expr, $protocol:expr) => {
        DiscriminatorInfo {
            discriminator: $disc,
            parser: $parser,
            protocol: $protocol,
            name: $name,
        }
    };
}

/// Compile-time constant array: discriminator -> parser info
/// MUST be kept sorted by discriminator for binary search!
///
/// Expected latency: 3-8ns (binary search on 31 entries = max 5 comparisons)
pub const DISCRIMINATOR_LUT: &[DiscriminatorInfo] = &[
    // Raydium AMM V4 events (sorted first - smallest discriminators)
    disc_entry!(
        0x0100000000000000,
        "Raydium AMM Initialize2",
        parse_raydium_amm_initialize2,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0300000000000000,
        "Raydium AMM Deposit",
        parse_raydium_amm_deposit,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0400000000000000,
        "Raydium AMM Withdraw",
        parse_raydium_amm_withdraw,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0900000000000000,
        "Raydium AMM Swap Base In",
        parse_raydium_amm_swap_base_in,
        Protocol::RaydiumAmm
    ),
    disc_entry!(
        0x0B00000000000000,
        "Raydium AMM Swap Base Out",
        parse_raydium_amm_swap_base_out,
        Protocol::RaydiumAmm
    ),
    // Raydium CLMM events
    disc_entry!(
        0x012C5B686FD026A0,
        "Raydium CLMM Decrease Liquidity",
        parse_raydium_clmm_decrease_liquidity,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x0AB0EE45DF591D85,
        "Raydium CLMM Increase Liquidity",
        parse_raydium_clmm_increase_liquidity,
        Protocol::RaydiumClmm
    ),
    // Raydium CPMM events
    disc_entry!(
        0x22A16D949C4612B7,
        "Raydium CPMM Withdraw",
        parse_raydium_cpmm_withdraw,
        Protocol::RaydiumCpmm
    ),
    // PumpSwap events
    disc_entry!(0x2ADC03A50A372F3E, "PumpSwap Sell", parse_pumpswap_sell, Protocol::PumpSwap),
    // Meteora AMM events
    disc_entry!(
        0x3A981F67E861F474,
        "Meteora AMM Remove Liquidity",
        parse_meteora_amm_remove_liquidity,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0x529DDC6858292CCA,
        "Meteora AMM Pool Created",
        parse_meteora_amm_pool_created,
        Protocol::MeteoraAmm
    ),
    // PumpSwap and PumpFun events
    disc_entry!(
        0x74A776A0D20C31B1,
        "PumpSwap Create Pool",
        parse_pumpswap_create_pool,
        Protocol::PumpSwap
    ),
    disc_entry!(0x7663EBDE4DA91B1B, "PumpFun Create", parse_pumpfun_create, Protocol::PumpFun),
    disc_entry!(0x7777F52C1F52F467, "PumpSwap Buy", parse_pumpswap_buy, Protocol::PumpSwap),
    disc_entry!(
        0x77AB68BB63CF98A4,
        "Raydium CLMM Collect Fee",
        parse_raydium_clmm_collect_fee,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0x906B8E1F533DF878,
        "PumpSwap Add Liquidity",
        parse_pumpswap_add_liquidity,
        Protocol::PumpSwap
    ),
    disc_entry!(0x94EA945DB95DE9BD, "PumpFun Migrate", parse_pumpfun_migrate, Protocol::PumpFun),
    disc_entry!(
        0x96A02B93AF49CAE1,
        "Orca Whirlpool Traded",
        parse_orca_traded,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0xA19BFE6690071E1E,
        "Orca Whirlpool Liquidity Increased",
        parse_orca_liquidity_increased,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0xABB5CA7047241A6,
        "Orca Whirlpool Liquidity Decreased",
        parse_orca_liquidity_decreased,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(
        0xADB44AA35662D937,
        "Raydium CPMM Swap Base Out",
        parse_raydium_cpmm_swap_base_out,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xB6F2E15289C623F2,
        "Raydium CPMM Deposit",
        parse_raydium_cpmm_deposit,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xBA3D34E35A7D5E1F,
        "Meteora AMM Add Liquidity",
        parse_meteora_amm_add_liquidity,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0xBC4068CF8ED192E9,
        "Raydium CLMM Create Pool",
        parse_raydium_clmm_create_pool,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xC0472CA01A850916,
        "PumpSwap Remove Liquidity",
        parse_pumpswap_remove_liquidity,
        Protocol::PumpSwap
    ),
    disc_entry!(
        0xC40AD0CDBE6CE351,
        "Meteora AMM Swap",
        parse_meteora_amm_swap,
        Protocol::MeteoraAmm
    ),
    disc_entry!(
        0xC887759EE19EC6F8,
        "Raydium CLMM Swap",
        parse_raydium_clmm_swap,
        Protocol::RaydiumClmm
    ),
    disc_entry!(
        0xDE331EC4DA5ABE8F,
        "Raydium CPMM Swap Base In",
        parse_raydium_cpmm_swap_base_in,
        Protocol::RaydiumCpmm
    ),
    disc_entry!(
        0xE5FEC60C57AD7664,
        "Orca Whirlpool Initialize",
        parse_orca_pool_initialized,
        Protocol::OrcaWhirlpool
    ),
    disc_entry!(0xEE61E64ED37FDBBD, "PumpFun Trade", parse_pumpfun_trade, Protocol::PumpFun),
    disc_entry!(
        0xF70E375C88267F79,
        "Meteora AMM Bootstrap Liquidity",
        parse_meteora_amm_bootstrap_liquidity,
        Protocol::MeteoraAmm
    ),
];

/// Fast lookup by discriminator - O(log n) binary search
///
/// With 31 entries, this requires at most 5 comparisons
#[inline(always)]
pub fn lookup_discriminator(discriminator: u64) -> Option<&'static DiscriminatorInfo> {
    DISCRIMINATOR_LUT
        .binary_search_by_key(&discriminator, |info| info.discriminator)
        .ok()
        .map(|idx| &DISCRIMINATOR_LUT[idx])
}

/// Get event name from discriminator
#[inline(always)]
pub fn discriminator_to_name(discriminator: u64) -> Option<&'static str> {
    lookup_discriminator(discriminator).map(|info| info.name)
}

/// Get protocol from discriminator
#[inline(always)]
pub fn discriminator_to_protocol(discriminator: u64) -> Option<Protocol> {
    lookup_discriminator(discriminator).map(|info| info.protocol)
}

/// Parse event using discriminator lookup
#[inline(always)]
pub fn parse_with_discriminator(
    discriminator: u64,
    data: &[u8],
    metadata: EventMetadata,
) -> Option<DexEvent> {
    let info = lookup_discriminator(discriminator)?;
    (info.parser)(data, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_is_sorted() {
        // Verify the LUT is sorted (required for binary search)
        for i in 1..DISCRIMINATOR_LUT.len() {
            assert!(
                DISCRIMINATOR_LUT[i - 1].discriminator < DISCRIMINATOR_LUT[i].discriminator,
                "LUT not sorted at index {}: {:x} >= {:x}",
                i,
                DISCRIMINATOR_LUT[i - 1].discriminator,
                DISCRIMINATOR_LUT[i].discriminator
            );
        }
    }

    #[test]
    fn test_discriminator_lookup() {
        // PumpFun Create
        let disc = 0x7663EBDE4DA91B1B;
        let info = lookup_discriminator(disc).unwrap();
        assert_eq!(info.name, "PumpFun Create");
        assert_eq!(info.protocol, Protocol::PumpFun);

        // Raydium CLMM Swap
        let disc = 0xC887759EE19EC6F8;
        let info = lookup_discriminator(disc).unwrap();
        assert_eq!(info.name, "Raydium CLMM Swap");
        assert_eq!(info.protocol, Protocol::RaydiumClmm);

        // Unknown discriminator
        let disc = 0xFFFFFFFFFFFFFFFF;
        assert!(lookup_discriminator(disc).is_none());
    }

    #[test]
    fn test_event_name_lookup() {
        // PumpSwap Buy
        let disc = 0x7777F52C1F52F467;
        assert_eq!(discriminator_to_name(disc), Some("PumpSwap Buy"));

        // Raydium AMM Swap Base In
        let disc = 0x0900000000000000;
        assert_eq!(discriminator_to_name(disc), Some("Raydium AMM Swap Base In"));
    }

    #[test]
    fn test_protocol_lookup() {
        assert_eq!(discriminator_to_protocol(0x7663EBDE4DA91B1B), Some(Protocol::PumpFun));
        assert_eq!(discriminator_to_protocol(0xC887759EE19EC6F8), Some(Protocol::RaydiumClmm));
        assert_eq!(discriminator_to_protocol(0x96A02B93AF49CAE1), Some(Protocol::OrcaWhirlpool));
    }
}
