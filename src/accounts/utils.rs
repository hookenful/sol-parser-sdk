//! 账户解析工具函数
//!
//! 提供账户数据解析的通用工具函数

use solana_sdk::pubkey::Pubkey;

/// 从字节数组中读取 Pubkey
#[inline]
pub fn read_pubkey(data: &[u8], offset: usize) -> Option<Pubkey> {
    if data.len() < offset + 32 {
        return None;
    }
    let bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
    Some(Pubkey::new_from_array(bytes))
}

/// 从字节数组中读取 u64（小端序）
#[inline]
pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    if data.len() < offset + 8 {
        return None;
    }
    Some(u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?))
}

/// 从字节数组中读取 u16（小端序）
#[inline]
pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    if data.len() < offset + 2 {
        return None;
    }
    Some(u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?))
}

/// 从字节数组中读取 u8
#[inline]
pub fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

/// 检查账户是否是 Nonce Account
///
/// Nonce accounts 有一个 discriminator: [1, 0, 0, 0, 1, 0, 0, 0]
#[inline]
pub fn is_nonce_account(data: &[u8]) -> bool {
    data.len() >= 8 && &data[0..8] == &[1, 0, 0, 0, 1, 0, 0, 0]
}

/// 检查账户所有者是否是 Token Program
#[inline]
pub fn is_token_program_account(owner: &Pubkey) -> bool {
    owner.to_bytes() == spl_token::ID.to_bytes()
        || owner.to_bytes() == spl_token_2022::ID.to_bytes()
}

/// 检查账户是否匹配指定的 discriminator
#[inline]
pub fn has_discriminator(data: &[u8], discriminator: &[u8]) -> bool {
    if data.len() < discriminator.len() {
        return false;
    }
    &data[0..discriminator.len()] == discriminator
}
