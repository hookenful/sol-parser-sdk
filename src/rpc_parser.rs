//! RPC Transaction Parser
//!
//! 提供独立的 RPC 交易解析功能，不依赖 gRPC streaming
//! 可以用于测试验证和离线分析

use crate::core::events::DexEvent;
use crate::grpc::instruction_parser::parse_instructions_enhanced;
use crate::grpc::types::EventTypeFilter;
use crate::instr::read_pubkey_fast;
use base64::{engine::general_purpose, Engine as _};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiTransactionEncoding,
};
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude::{
    CompiledInstruction, InnerInstruction, InnerInstructions, Message, MessageAddressTableLookup,
    MessageHeader, Transaction, TransactionStatusMeta,
};

/// Parse a transaction from RPC by signature
///
/// # Arguments
/// * `rpc_client` - RPC client to fetch the transaction
/// * `signature` - Transaction signature
/// * `filter` - Optional event type filter
///
/// # Returns
/// Vector of parsed DEX events
///
/// # Example
/// ```no_run
/// use solana_client::rpc_client::RpcClient;
/// use solana_sdk::signature::Signature;
/// use sol_parser_sdk::parse_transaction_from_rpc;
/// use std::str::FromStr;
///
/// let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
/// let sig = Signature::from_str("your-signature-here").unwrap();
/// let events = parse_transaction_from_rpc(&client, &sig, None).unwrap();
/// ```
pub fn parse_transaction_from_rpc(
    rpc_client: &RpcClient,
    signature: &Signature,
    filter: Option<&EventTypeFilter>,
) -> Result<Vec<DexEvent>, ParseError> {
    // Fetch transaction from RPC with V0 transaction support
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: None,
        max_supported_transaction_version: Some(0),
    };

    let rpc_tx = rpc_client
        .get_transaction_with_config(signature, config)
        .map_err(|e| ParseError::RpcError(e.to_string()))?;

    parse_rpc_transaction(&rpc_tx, filter)
}

/// Parse a RPC transaction structure
///
/// # Arguments
/// * `rpc_tx` - RPC transaction to parse
/// * `filter` - Optional event type filter
///
/// # Returns
/// Vector of parsed DEX events
///
/// # Example
/// ```no_run
/// use sol_parser_sdk::parse_rpc_transaction;
///
/// // Assuming you have an rpc_tx from RPC
/// // let events = parse_rpc_transaction(&rpc_tx, None).unwrap();
/// ```
pub fn parse_rpc_transaction(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
    filter: Option<&EventTypeFilter>,
) -> Result<Vec<DexEvent>, ParseError> {
    // Convert RPC format to gRPC format
    let (grpc_meta, grpc_tx) = convert_rpc_to_grpc(rpc_tx)?;

    // Extract metadata
    let signature = extract_signature(rpc_tx)?;
    let slot = rpc_tx.slot;
    let block_time_us = rpc_tx.block_time.map(|t| t * 1_000_000);
    let grpc_recv_us =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros()
            as i64;

    // Wrap grpc_tx in Option for reuse
    let grpc_tx_opt = Some(grpc_tx);

    // Build program_invokes HashMap for account filling
    // Use string keys to match gRPC parsing logic
    let mut program_invokes: HashMap<&str, Vec<(i32, i32)>> = HashMap::new();

    if let Some(ref tx) = grpc_tx_opt {
        if let Some(ref msg) = tx.message {
            // Build account key lookup
            let keys_len = msg.account_keys.len();
            let writable_len = grpc_meta.loaded_writable_addresses.len();
            let get_key = |i: usize| -> Option<&Vec<u8>> {
                if i < keys_len {
                    msg.account_keys.get(i)
                } else if i < keys_len + writable_len {
                    grpc_meta.loaded_writable_addresses.get(i - keys_len)
                } else {
                    grpc_meta.loaded_readonly_addresses.get(i - keys_len - writable_len)
                }
            };

            // Record outer instructions
            for (i, ix) in msg.instructions.iter().enumerate() {
                let pid = get_key(ix.program_id_index as usize)
                    .map_or(Pubkey::default(), |k| read_pubkey_fast(k));
                let pid_str = pid.to_string();
                let pid_static: &'static str = pid_str.leak();
                program_invokes.entry(pid_static).or_default().push((i as i32, -1));
            }

            // Record inner instructions
            for inner in &grpc_meta.inner_instructions {
                let outer_idx = inner.index as usize;
                for (j, inner_ix) in inner.instructions.iter().enumerate() {
                    let pid = get_key(inner_ix.program_id_index as usize)
                        .map_or(Pubkey::default(), |k| read_pubkey_fast(k));
                    let pid_str = pid.to_string();
                    let pid_static: &'static str = pid_str.leak();
                    program_invokes
                        .entry(pid_static)
                        .or_default()
                        .push((outer_idx as i32, j as i32));
                }
            }
        }
    }

    // Parse instructions
    let mut events = parse_instructions_enhanced(
        &grpc_meta,
        &grpc_tx_opt,
        signature,
        slot,
        0, // tx_idx
        block_time_us,
        grpc_recv_us,
        filter,
    );

    // Parse logs (for protocols like PumpFun that emit events in logs)
    let mut is_created_buy = false;

    for log in &grpc_meta.log_messages {
        if let Some(mut event) = crate::logs::parse_log(
            log,
            signature,
            slot,
            0, // tx_index
            block_time_us,
            grpc_recv_us,
            filter,
            is_created_buy,
        ) {
            // Check if this is a PumpFun create event to set is_created_buy flag
            if matches!(event, DexEvent::PumpFunCreate(_)) {
                is_created_buy = true;
            }

            // Fill account fields - use same function as gRPC parsing
            crate::core::account_dispatcher::fill_accounts_from_transaction_data(
                &mut event,
                &grpc_meta,
                &grpc_tx_opt,
                &program_invokes,
            );

            // Fill additional data fields (e.g., PumpSwap is_pump_pool)
            crate::core::common_filler::fill_data(
                &mut event,
                &grpc_meta,
                &grpc_tx_opt,
                &program_invokes,
            );

            events.push(event);
        }
    }

    Ok(events)
}

/// Parse error types
#[derive(Debug)]
pub enum ParseError {
    RpcError(String),
    ConversionError(String),
    MissingField(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            ParseError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
            ParseError::MissingField(msg) => write!(f, "Missing field: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

// ============================================================================
// Internal conversion functions
// ============================================================================

fn extract_signature(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Signature, ParseError> {
    let ui_tx = &rpc_tx.transaction.transaction;

    match ui_tx {
        EncodedTransaction::Binary(data, _encoding) => {
            let bytes = general_purpose::STANDARD.decode(data).map_err(|e| {
                ParseError::ConversionError(format!("Failed to decode base64: {}", e))
            })?;

            let versioned_tx: solana_sdk::transaction::VersionedTransaction =
                bincode::deserialize(&bytes).map_err(|e| {
                    ParseError::ConversionError(format!("Failed to deserialize transaction: {}", e))
                })?;

            Ok(versioned_tx.signatures[0])
        }
        _ => Err(ParseError::ConversionError("Unsupported transaction encoding".to_string())),
    }
}

pub fn convert_rpc_to_grpc(
    rpc_tx: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<(TransactionStatusMeta, Transaction), ParseError> {
    let rpc_meta = rpc_tx
        .transaction
        .meta
        .as_ref()
        .ok_or_else(|| ParseError::MissingField("meta".to_string()))?;

    // Convert meta
    let mut grpc_meta = TransactionStatusMeta {
        err: None,
        fee: rpc_meta.fee,
        pre_balances: rpc_meta.pre_balances.clone(),
        post_balances: rpc_meta.post_balances.clone(),
        inner_instructions: Vec::new(),
        log_messages: {
            let opt: Option<Vec<String>> = rpc_meta.log_messages.clone().into();
            opt.unwrap_or_default()
        },
        pre_token_balances: Vec::new(),
        post_token_balances: Vec::new(),
        rewards: Vec::new(),
        loaded_writable_addresses: {
            let loaded_opt: Option<solana_transaction_status::UiLoadedAddresses> =
                rpc_meta.loaded_addresses.clone().into();
            loaded_opt
                .map(|addrs| {
                    addrs
                        .writable
                        .iter()
                        .map(|pk_str| {
                            use std::str::FromStr;
                            solana_sdk::pubkey::Pubkey::from_str(pk_str)
                                .unwrap()
                                .to_bytes()
                                .to_vec()
                        })
                        .collect()
                })
                .unwrap_or_default()
        },
        loaded_readonly_addresses: {
            let loaded_opt: Option<solana_transaction_status::UiLoadedAddresses> =
                rpc_meta.loaded_addresses.clone().into();
            loaded_opt
                .map(|addrs| {
                    addrs
                        .readonly
                        .iter()
                        .map(|pk_str| {
                            use std::str::FromStr;
                            solana_sdk::pubkey::Pubkey::from_str(pk_str)
                                .unwrap()
                                .to_bytes()
                                .to_vec()
                        })
                        .collect()
                })
                .unwrap_or_default()
        },
        return_data: None,
        compute_units_consumed: rpc_meta.compute_units_consumed.clone().into(),
        cost_units: None,
        inner_instructions_none: {
            let opt: Option<Vec<_>> = rpc_meta.inner_instructions.clone().into();
            opt.is_none()
        },
        log_messages_none: {
            let opt: Option<Vec<String>> = rpc_meta.log_messages.clone().into();
            opt.is_none()
        },
        return_data_none: {
            let opt: Option<solana_transaction_status::UiTransactionReturnData> =
                rpc_meta.return_data.clone().into();
            opt.is_none()
        },
    };

    // Convert inner instructions
    let inner_instructions_opt: Option<Vec<_>> = rpc_meta.inner_instructions.clone().into();
    if let Some(ref inner_instructions) = inner_instructions_opt {
        for inner in inner_instructions {
            let mut grpc_inner =
                InnerInstructions { index: inner.index as u32, instructions: Vec::new() };

            for ix in &inner.instructions {
                if let solana_transaction_status::UiInstruction::Compiled(compiled) = ix {
                    // Decode base58 data
                    let data = bs58::decode(&compiled.data).into_vec().map_err(|e| {
                        ParseError::ConversionError(format!(
                            "Failed to decode instruction data: {}",
                            e
                        ))
                    })?;

                    grpc_inner.instructions.push(InnerInstruction {
                        program_id_index: compiled.program_id_index as u32,
                        accounts: compiled.accounts.clone(),
                        data,
                        stack_height: compiled.stack_height.map(|h| h as u32),
                    });
                }
            }

            grpc_meta.inner_instructions.push(grpc_inner);
        }
    }

    // Convert transaction
    let ui_tx = &rpc_tx.transaction.transaction;

    let (message, signatures) = match ui_tx {
        EncodedTransaction::Binary(data, _encoding) => {
            // Decode base64
            let bytes = general_purpose::STANDARD.decode(data).map_err(|e| {
                ParseError::ConversionError(format!("Failed to decode base64: {}", e))
            })?;

            // Parse as versioned transaction
            let versioned_tx: solana_sdk::transaction::VersionedTransaction =
                bincode::deserialize(&bytes).map_err(|e| {
                    ParseError::ConversionError(format!("Failed to deserialize transaction: {}", e))
                })?;

            let sigs: Vec<Vec<u8>> =
                versioned_tx.signatures.iter().map(|s| s.as_ref().to_vec()).collect();

            let message = match versioned_tx.message {
                solana_sdk::message::VersionedMessage::Legacy(legacy_msg) => {
                    convert_legacy_message(&legacy_msg)?
                }
                solana_sdk::message::VersionedMessage::V0(v0_msg) => convert_v0_message(&v0_msg)?,
            };

            (message, sigs)
        }
        EncodedTransaction::Json(_) => {
            return Err(ParseError::ConversionError(
                "JSON encoded transactions not supported yet".to_string(),
            ));
        }
        _ => {
            return Err(ParseError::ConversionError(
                "Unsupported transaction encoding".to_string(),
            ));
        }
    };

    let grpc_tx = Transaction { signatures, message: Some(message) };

    Ok((grpc_meta, grpc_tx))
}

fn convert_legacy_message(
    msg: &solana_sdk::message::legacy::Message,
) -> Result<Message, ParseError> {
    let account_keys: Vec<Vec<u8>> =
        msg.account_keys.iter().map(|k| k.to_bytes().to_vec()).collect();

    let instructions: Vec<CompiledInstruction> = msg
        .instructions
        .iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u32,
            accounts: ix.accounts.clone(),
            data: ix.data.clone(),
        })
        .collect();

    Ok(Message {
        header: Some(MessageHeader {
            num_required_signatures: msg.header.num_required_signatures as u32,
            num_readonly_signed_accounts: msg.header.num_readonly_signed_accounts as u32,
            num_readonly_unsigned_accounts: msg.header.num_readonly_unsigned_accounts as u32,
        }),
        account_keys,
        recent_blockhash: msg.recent_blockhash.to_bytes().to_vec(),
        instructions,
        versioned: false,
        address_table_lookups: Vec::new(),
    })
}

fn convert_v0_message(msg: &solana_sdk::message::v0::Message) -> Result<Message, ParseError> {
    let account_keys: Vec<Vec<u8>> =
        msg.account_keys.iter().map(|k| k.to_bytes().to_vec()).collect();

    let instructions: Vec<CompiledInstruction> = msg
        .instructions
        .iter()
        .map(|ix| CompiledInstruction {
            program_id_index: ix.program_id_index as u32,
            accounts: ix.accounts.clone(),
            data: ix.data.clone(),
        })
        .collect();

    Ok(Message {
        header: Some(MessageHeader {
            num_required_signatures: msg.header.num_required_signatures as u32,
            num_readonly_signed_accounts: msg.header.num_readonly_signed_accounts as u32,
            num_readonly_unsigned_accounts: msg.header.num_readonly_unsigned_accounts as u32,
        }),
        account_keys,
        recent_blockhash: msg.recent_blockhash.to_bytes().to_vec(),
        instructions,
        versioned: true,
        address_table_lookups: msg
            .address_table_lookups
            .iter()
            .map(|lookup| MessageAddressTableLookup {
                account_key: lookup.account_key.to_bytes().to_vec(),
                writable_indexes: lookup.writable_indexes.clone(),
                readonly_indexes: lookup.readonly_indexes.clone(),
            })
            .collect(),
    })
}
