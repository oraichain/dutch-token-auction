use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use crate::constants;

pub const AUCTION_CONFIG_VERSION: u8 = 1;

pub const AUCTION_CONFIG_SIZE: usize = 8 + 1 + 1 + 32 + 32 + 32 + 32 + 4 + 8 + 8 + 32 + 4 + 4 + 4;

#[account]
pub struct AuctionConfig {
    /// Bump seed used to generate the program address / authority
    pub bump: [u8; 1],
    pub version: u8,
    /// Owner of the configuration
    pub authority: Pubkey,
    // /// authority that is allowed to close accounts (different from owner)
    pub moderator: Pubkey,
    /// global vault for storing SOL during auction
    pub global_vault: Pubkey,
    /// SPL token mint or native mint for SOL for the pool bets
    pub currency_mint: Pubkey,
    /// Number of seconds between start/lock/settle
    pub interval_seconds: u32,
    /// Unix timestamp of the next time an event should start for this config
    pub next_auction_start: i64,
    // next round id
    pub next_round_id: u64,
    pub fee_account: Pubkey,
    /// Fee rate in bps
    pub fee_bps: u32,
    /// Amount in bps to burn from the fees received
    pub fee_burn_bps: u32,
    /// maximum num of auction slots at a period
    pub max_auction_slots: u32,
}

impl AuctionConfig {
    /// Seeds are unique to authority/pyth feed/currency mint combinations
    pub fn auth_seeds<'a>(&'a self) -> [&'a [u8]; 4] {
        [
            constants::CONFIG.as_ref(),
            self.authority.as_ref(),
            self.currency_mint.as_ref(),
            self.bump.as_ref(),
        ]
    }
}
