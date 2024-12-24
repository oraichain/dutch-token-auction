use crate::{
    constants,
    error::CustomErrorCode,
    state::{AuctionConfig, AUCTION_CONFIG_SIZE, AUCTION_CONFIG_VERSION},
};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::Mint;
use solana_program::sysvar::SysvarId;

#[derive(Accounts)]
pub struct CreateAuctionConfig<'info> {
    /// CHECK: Allow any account to be the settle authority
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [
            constants::CONFIG,
            authority.key().as_ref(),
            currency_mint.key().as_ref(),
        ],
        bump,
        space = AUCTION_CONFIG_SIZE,
        payer = authority
    )]
    pub auction_config: Box<Account<'info, AuctionConfig>>,

    /// CHECK: Should be a valid pyth feed
    #[account()]
    pub moderator: UncheckedAccount<'info>,

    pub currency_mint: Account<'info, Mint>,

    /// CHECK: Allow any account to be the fee account
    #[account()]
    pub fee_account: UncheckedAccount<'info>,

    /// CHECK: Allow any account to be the global vault for storing SOL
    #[account()]
    pub global_vault: UncheckedAccount<'info>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    #[account(address = Rent::id())]
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> CreateAuctionConfig<'info> {
    pub fn process(
        &mut self,
        interval_seconds: u32,
        next_auction_start: i64,
        fee_bps: u32,
        fee_burn_bps: u32,
        config_bump: u8,
    ) -> Result<()> {
        if fee_bps > 10_000 {
            return err!(CustomErrorCode::InvalidFee);
        }

        if fee_burn_bps > 10_000 {
            return err!(CustomErrorCode::InvalidFeeBurn);
        }

        let auction_config = &mut self.auction_config;
        auction_config.bump = [config_bump];
        auction_config.version = AUCTION_CONFIG_VERSION;
        auction_config.authority = self.authority.key();
        auction_config.moderator = self.moderator.key();
        auction_config.currency_mint = self.currency_mint.key();
        auction_config.interval_seconds = interval_seconds;
        auction_config.next_auction_start = next_auction_start;
        auction_config.next_round_id = 1;
        auction_config.fee_bps = fee_bps;
        auction_config.fee_burn_bps = fee_burn_bps;
        auction_config.fee_account = self.fee_account.key();
        auction_config.global_vault = self.global_vault.key();

        msg!(
            "auction config account{:?}",
            self.auction_config.to_account_info()
        );

        Ok(())
    }
}
