//! Raydium LaunchLab 指令解析器
//!
//! 底层按 `idls/raydium_launchpad.json` 的真实 instruction discriminator
//! 和账户布局解析，对外事件名统一为 `RaydiumLaunchlab*`。

use super::program_ids;
use super::utils::*;
use crate::core::events::*;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Raydium LaunchLab instruction discriminators from `idls/raydium_launchpad.json`.
pub mod discriminators {
    pub const CREATE_PLATFORM_CONFIG: [u8; 8] = [176, 90, 196, 175, 253, 113, 220, 20];
    pub const BUY_EXACT_IN: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];
    pub const BUY_EXACT_OUT: [u8; 8] = [24, 211, 116, 40, 105, 3, 153, 56];
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];
    pub const INITIALIZE_WITH_TOKEN_2022: [u8; 8] = [37, 190, 126, 222, 44, 154, 171, 17];
    pub const MIGRATE_TO_AMM: [u8; 8] = [207, 82, 192, 145, 254, 207, 145, 223];
    pub const MIGRATE_TO_CPSWAP: [u8; 8] = [136, 92, 200, 103, 28, 218, 144, 140];
    pub const SELL_EXACT_IN: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];
    pub const SELL_EXACT_OUT: [u8; 8] = [95, 200, 71, 34, 8, 9, 11, 166];
}

/// Raydium LaunchLab 程序 ID
pub const PROGRAM_ID_PUBKEY: Pubkey = program_ids::RAYDIUM_LAUNCHLAB_PROGRAM_ID;

/// 主要的 Raydium LaunchLab 指令解析函数
pub fn parse_instruction(
    instruction_data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    if instruction_data.len() < 8 {
        return None;
    }

    let discriminator: [u8; 8] = instruction_data[0..8].try_into().ok()?;
    let data = &instruction_data[8..];

    match discriminator {
        discriminators::CREATE_PLATFORM_CONFIG => parse_create_platform_config_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
        ),
        discriminators::BUY_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            true,
        ),
        discriminators::BUY_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            true,
            false,
        ),
        discriminators::SELL_EXACT_IN => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            true,
        ),
        discriminators::SELL_EXACT_OUT => parse_trade_instruction(
            data,
            accounts,
            signature,
            slot,
            tx_index,
            block_time_us,
            false,
            false,
        ),
        discriminators::INITIALIZE
        | discriminators::INITIALIZE_V2
        | discriminators::INITIALIZE_WITH_TOKEN_2022 => {
            parse_pool_create_instruction(data, accounts, signature, slot, tx_index, block_time_us)
        }
        // The LaunchLab IDL does not expose enough fields to synthesize a
        // migration event with the SDK's migrate layout.
        discriminators::MIGRATE_TO_AMM | discriminators::MIGRATE_TO_CPSWAP => None,
        _ => None,
    }
}

/// Parse `create_platform_config` instructions.
///
/// Observed on real mainnet transactions:
/// - 5Pp8jWtS15RQaTLyXcgrJZTM5WpBqd7CbT7jXfyrMCG59SuS1DV131S8yVW3W7wuomtPGFDYHvTuQU27ajvYvorT
/// - 5X9jfR9NddudRXacPZ3V1U4Suy2AUGjWCuLG36yzV8rrepiZg3crFD2Chi8pKCY6t2s4cbH629Jq6bJ7NgZYzq1o
///
/// The instruction data after the discriminator is the Borsh-encoded `PlatformParams` printed
/// by the program logs. The created `PlatformConfig` account itself is accounts[3].
fn parse_create_platform_config_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let mut offset = 0usize;
    let platform_scale = read_u64_le(data, offset)?;
    offset += 8;
    let creator_scale = read_u64_le(data, offset)?;
    offset += 8;
    let burn_scale = read_u64_le(data, offset)?;
    offset += 8;
    let fee_rate = read_u64_le(data, offset)?;
    offset += 8;
    let name = read_borsh_string(data, &mut offset)?;
    let web = read_borsh_string(data, &mut offset)?;
    let img = read_borsh_string(data, &mut offset)?;
    let creator_fee_rate = read_u64_le(data, offset)?;
    offset += 8;
    let platform_vesting_scale = read_u64_le(data, offset)?;

    let platform_config_pubkey = get_account(accounts, 3)?;
    let metadata =
        create_metadata_simple(signature, slot, tx_index, block_time_us, platform_config_pubkey);

    let platform_config = RaydiumLaunchlabPlatformConfig {
        // The on-chain account epoch is not present in the create instruction payload.
        epoch: 0,
        platform_fee_wallet: get_account(accounts, 0).unwrap_or_default(),
        platform_nft_wallet: get_account(accounts, 1).unwrap_or_default(),
        platform_scale,
        creator_scale,
        burn_scale,
        fee_rate,
        name: fixed_bytes::<64>(&name),
        web: fixed_bytes::<256>(&web),
        img: fixed_bytes::<256>(&img),
        cpswap_config: get_account(accounts, 4).unwrap_or_default(),
        creator_fee_rate,
        transfer_fee_extension_auth: get_account(accounts, 2).unwrap_or_default(),
        platform_vesting_wallet: Pubkey::default(),
        platform_vesting_scale,
        platform_cp_creator: Pubkey::default(),
        padding: [0u8; 108],
        curve_params: Vec::new(),
    };

    Some(DexEvent::RaydiumLaunchlabPlatformConfigAccount(Box::new(
        RaydiumLaunchlabPlatformConfigAccountEvent {
            metadata,
            pubkey: platform_config_pubkey,
            platform_config,
        },
    )))
}

/// 解析 buy/sell 指令.
///
/// 外层指令只携带用户输入的 amount / min-max amount；真实成交量由 log 事件覆盖。
fn parse_trade_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
    is_buy: bool,
    exact_in: bool,
) -> Option<DexEvent> {
    let first_amount = read_u64_le(data, 0)?;
    let second_amount = read_u64_le(data, 8)?;

    let (amount_in, amount_out) =
        if exact_in { (first_amount, second_amount) } else { (second_amount, first_amount) };

    let pool_state = get_account(accounts, 4)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabTrade(RaydiumLaunchlabTradeEvent {
        metadata,
        pool_state,
        user: get_account(accounts, 0).unwrap_or_default(),
        amount_in,
        amount_out,
        is_buy,
        trade_direction: if is_buy { TradeDirection::Buy } else { TradeDirection::Sell },
        exact_in,
    }))
}

/// 解析 initialize / initialize_v2 / initialize_with_token_2022 指令。
fn parse_pool_create_instruction(
    data: &[u8],
    accounts: &[Pubkey],
    signature: Signature,
    slot: u64,
    tx_index: u64,
    block_time_us: Option<i64>,
) -> Option<DexEvent> {
    let base_mint_param = parse_mint_params(data)?;

    let pool_state = get_account(accounts, 5)?;
    let metadata = create_metadata_simple(signature, slot, tx_index, block_time_us, pool_state);

    Some(DexEvent::RaydiumLaunchlabPoolCreate(RaydiumLaunchlabPoolCreateEvent {
        metadata,
        base_mint_param,
        pool_state,
        creator: get_account(accounts, 1).unwrap_or_default(),
    }))
}

fn parse_mint_params(data: &[u8]) -> Option<BaseMintParam> {
    let mut offset = 0usize;
    let decimals = *data.get(offset)?;
    offset += 1;
    let name = read_borsh_string(data, &mut offset)?;
    let symbol = read_borsh_string(data, &mut offset)?;
    let uri = read_borsh_string(data, &mut offset)?;
    Some(BaseMintParam { symbol, name, uri, decimals })
}

fn read_borsh_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = read_u32_le(data, *offset)? as usize;
    *offset += 4;
    let end = (*offset).checked_add(len)?;
    let bytes = data.get(*offset..end)?;
    *offset = end;
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn fixed_bytes<const N: usize>(s: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let bytes = s.as_bytes();
    let len = bytes.len().min(N);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn fixed_string(bytes: &[u8]) -> String {
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).trim().to_string()
    }

    fn parse_platform_case(
        data_b58: &str,
        platform_config: &str,
    ) -> RaydiumLaunchlabPlatformConfigAccountEvent {
        let data = bs58::decode(data_b58).into_vec().expect("valid base58 ix data");
        let wallet = Pubkey::from_str("FJPEQs64G2wbX5vCu9gkfGjv9Kd3JGUnkLM5aX6Fy4oT").unwrap();
        let accounts = [
            wallet,
            wallet,
            wallet,
            Pubkey::from_str(platform_config).unwrap(),
            Pubkey::from_str("C7Cx2pMLtjybS3mDKSfsBj4zQ3PRZGkKt7RCYTTbCSx2").unwrap(),
            Pubkey::default(),
            wallet,
            Pubkey::default(),
        ];
        let event = parse_instruction(&data, &accounts, Signature::default(), 1, 0, Some(0))
            .expect("create_platform_config should parse");
        let DexEvent::RaydiumLaunchlabPlatformConfigAccount(event) = event else {
            panic!("expected platform config account event");
        };
        *event
    }

    #[test]
    fn parses_real_create_platform_config_based_fail() {
        let event = parse_platform_case(
            "Du14A4ohBY2wfggPsT2owJGM75mTrfUPwcCY3a6HNuM5y2iHJVqXJH1NHmSjuUo1yBce6YuqNA9fVTsGc595RwMEVpC7kfRsbUwdFJk9foPthZgBbeD3Rs4ERvqyZgAQw47nFMoRBBabJJhWSoK9DFwK4Rm9sHE8wD5BAoT21QQe5bKqioFLhJ4FAAbQbv2FeLcxxgBCrXDKKVqVjJicSgQ92qjk9gZb91hrizDshEpijiYQ2b",
            "BPMs3R4drYbZ2xhg9cmGiawgsoE786AignNR72aKQ1oF",
        );
        assert_eq!(event.pubkey.to_string(), "BPMs3R4drYbZ2xhg9cmGiawgsoE786AignNR72aKQ1oF");
        assert_eq!(fixed_string(&event.platform_config.name), "based.fail");
        assert_eq!(fixed_string(&event.platform_config.web), "https://based.fail/");
        assert_eq!(event.platform_config.fee_rate, 10_000);
        assert_eq!(event.platform_config.creator_fee_rate, 5_000);
        assert_eq!(event.platform_config.creator_scale, 1_000_000);
    }

    #[test]
    fn parses_real_create_platform_config_based_fun() {
        let event = parse_platform_case(
            "3vUCbE6bUmBrzhi6dogNxoWJhM7T2HRLFBhDvbgiKZFX8FtVcgcToWTN4ViEKbP98d7bnr2B2tzt197CGow28WiMaDCcCSKEYVQ5wP6fcdThHUh7BH34DjBGfYxcfwYodCZkgeDtbj6NgFWUS4H9Qe6LV5sSifgQKBGP3e5TD8gJZtc58n9U8WCyvCPjSPyPCSn9hrVjCqpeXd7ibp1VbdT2eqBsSPuwaUG5qVrYsnsZ3DgoR",
            "ErgsDEX6zKxszu7nr384G1DNMwTPDB48PuD6MM92V5ZA",
        );
        assert_eq!(event.pubkey.to_string(), "ErgsDEX6zKxszu7nr384G1DNMwTPDB48PuD6MM92V5ZA");
        assert_eq!(fixed_string(&event.platform_config.name), "based.fun");
        assert_eq!(fixed_string(&event.platform_config.web), "https://based.fail/");
        assert_eq!(event.platform_config.fee_rate, 10_000);
        assert_eq!(event.platform_config.creator_fee_rate, 5_000);
        assert_eq!(event.platform_config.creator_scale, 1_000_000);
    }
}
