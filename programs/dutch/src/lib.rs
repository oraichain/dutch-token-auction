use anchor_lang::{
    prelude::*,
    solana_program::{
        program::invoke,
        system_instruction,
    },
    AccountsClose
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{
        Mint, 
        Token, 
        TokenAccount, 
        transfer, 
        Transfer, 
        close_account as close_token_account, 
        CloseAccount as CloseTokenAccount
    }
};

pub mod error;
pub mod instructions;
pub use instructions::*;
pub mod constants;
pub use constants::*;
pub mod state;
pub use state::*;
use crate::error::CustomErrorCode;

declare_id!("77p3Ka2WQ7a9zDS8CAE9r8ELLN5UtWidTMnd4PAnzmoM");

#[program]
pub mod dutch {
    use std::borrow::BorrowMut;

    use super::*;

    // initialize an auction
    pub fn initialize_auction(
        ctx: Context<InitializeAuction>, 
        starting_time: i64, 
        auction_period: i64, 
        start_price: u32,
        amount: u64,
        bump: u8,
    ) -> Result<()> {
        // check that start time precedes end time
        let ending_time = starting_time + auction_period;
        if starting_time >= ending_time {
            return Err(CustomErrorCode::InvalidDateRange.into());
        }

        // check that the current time is now or in the future
        let current_time = Clock::get()?.unix_timestamp;
        if current_time > starting_time {
            return Err(CustomErrorCode::InvalidStartDate.into());
        };

        let escrow_account = &ctx.accounts.escrow_token_account;
        if escrow_account.amount <= 0 {
            return Err(CustomErrorCode::AuctionInvalid.into());
        }

        if amount == 0 {
            return Err(CustomErrorCode::InvalidEscrowAmount.into());
        }

        // set the auction account data
        let auction_config = &mut ctx.accounts.auction_config;
        let auction_account: &mut Account<AuctionAccount> = &mut ctx.accounts.auction_account; 

        let current_auction_start = auction_config.next_auction_start;

        // cannot create round id current_auction_start > timestamp
        if current_auction_start > current_time {
            return err!(CustomErrorCode::PreviousRoundNotEnd);
        }
        if auction_account.current_auction_slot_count > 0 {
            return err!(CustomErrorCode::PreviousRoundNotEnd);
        }

        // prevent auction snipes and bots
        auction_config.next_auction_start = current_time + auction_config.interval_seconds as i64;
            
        auction_account.authority = ctx.accounts.authority.key().clone();
        auction_account.escrow_account = ctx.accounts.escrow_token_account.key();
        auction_account.starting_price = start_price;
        auction_account.starting_time = starting_time;
        auction_account.current_auction_slot_count = auction_config.max_auction_slots;
        
        // auction period can be much longer than the next auction round to reduce the price
        auction_account.auction_period = auction_period;
        auction_account.amount = amount.min(escrow_account.amount);
        auction_account.bump = bump;

        Ok(())
    }

    // allow the auction initializer to close the auction
    pub fn close_auction(
        ctx: Context<CloseAuction>,
    ) -> Result<()> {
        // transfer the remaining token(s) from escrow account to the owner
        let transfer_ctx = ctx.accounts.clone();
        let authority_key = ctx.accounts.authority.key();
        transfer(
            transfer_ctx.into_transfer_ctx()
            .with_signer(&[&[authority_key.as_ref(), &[ctx.accounts.auction_account.bump]]]),
            ctx.accounts.auction_account.amount, 
        )?;

        // close the token account
        let close_token_ctx = ctx.accounts.clone();
        close_token_account(
            close_token_ctx.into_close_account_ctx()
            .with_signer(&[&[authority_key.as_ref(), &[ctx.accounts.auction_account.bump]]]),
        )?;

        // close the auction account
        ctx.accounts.auction_account.close(ctx.accounts.authority.to_account_info())?;

        Ok(())
    }

    // the meat and potatoes, bidding on an auction
    pub fn bid(
        ctx: Context<Bid>,
    ) -> Result<()> {

        // TODO: add bidder details so when users want to sell during curve -> can refund them

        // compute the current price based on the time
        let current_time = Clock::get()?.unix_timestamp;
        let transfer_ctx = ctx.accounts.borrow_mut();
        let ending_time = transfer_ctx.auction_account.starting_time + transfer_ctx.auction_account.auction_period;

        // check that the auction is in session
        if current_time < transfer_ctx.auction_account.starting_time {
            return Err(CustomErrorCode::AuctionEarly.into());
        }
        if current_time > ending_time {
            return Err(CustomErrorCode::AuctionLate.into());
        }
        if transfer_ctx.auction_account.current_auction_slot_count == 0 {
            return Err(CustomErrorCode::AuctionLate.into());
        }

        let starting_price = transfer_ctx.auction_account.starting_price as f64;
        let elapsed_time = (current_time - transfer_ctx.auction_account.starting_time) as f64;
        let duration = (ending_time - transfer_ctx.auction_account.starting_time) as f64;
        let price = starting_price - ((elapsed_time * starting_price) / duration);

        // transfer lamports to the global vault
        invoke(
            &system_instruction::transfer(
                &transfer_ctx.authority.key(),
                &transfer_ctx.global_vault.key(),
                price as u64,
            ),
            &[
                transfer_ctx.authority.to_account_info(),
                transfer_ctx.global_vault.to_account_info(),
                transfer_ctx.system_program.to_account_info(),
            ],
        )?;

        // transfer the token to the bidder
        transfer(
            transfer_ctx.into_transfer_ctx()
            .with_signer(&[&[constants::AUCTION, transfer_ctx.auction_config.key().as_ref(), &[transfer_ctx.auction_account.bump]]]), 
            transfer_ctx.auction_account.amount
        )?;

        // reset start and end time after each bid
        transfer_ctx.auction_account.starting_time = current_time;
        // reduce an auction slot
        transfer_ctx.auction_account.current_auction_slot_count -= 1;

        // don't close any account, let the owner close at will

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(starting_time: i64, ending_time: i64, start_price: u32, amount: u32, bump: u8)]
pub struct InitializeAuction<'info> {
    #[account(mut, constraint = auction_config.moderator == moderator.key() @CustomErrorCode::IncorrectAuthority)]
    moderator: Signer<'info>,

    #[account(mut, constraint = auction_config.authority == authority.key() @CustomErrorCode::IncorrectAuthority)]
    authority: UncheckedAccount<'info>,

    #[account(
        seeds = [
            constants::CONFIG,
            auction_config.authority.as_ref(),
            mint.key().as_ref(),
        ],
        bump,
        has_one = authority,
    )]
    pub auction_config: Box<Account<'info, AuctionConfig>>,

    #[account(
        init_if_needed, 
        payer = authority, 
        seeds = [constants::AUCTION, auction_config.key().as_ref()],
        bump,
        space = AuctionAccount::LEN
    )]
    auction_account: Account<'info, AuctionAccount>,
    #[account(
        init_if_needed,
        payer = moderator,
        associated_token::mint = mint,
        associated_token::authority = auction_account,
    )]
    escrow_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    mint: Account<'info, Mint>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts, Clone)]
pub struct CloseAuction<'info> {
    #[account(mut)]
    authority: Signer<'info>,

    #[account(
        seeds = [
            constants::CONFIG,
            auction_config.authority.as_ref(),
            mint.key().as_ref(),
        ],
        bump,
        constraint = auction_config.authority == authority.key() @CustomErrorCode::IncorrectAuthority
    )]
    pub auction_config: Box<Account<'info, AuctionConfig>>,

    #[account(
        mut, 
        seeds = [constants::AUCTION, auction_config.key().as_ref()],
        bump,
        constraint = authority.key() == auction_account.authority @CustomErrorCode::IncorrectAuthority
    )]
    auction_account: Account<'info, AuctionAccount>,

    #[account(mut)]
    holder_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    escrow_token_account: Box<Account<'info, TokenAccount>>,
    mint: Account<'info, Mint>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts, Clone)]
pub struct Bid<'info> {
    #[account(
        mut,
    )]
    authority: Signer<'info>,

    #[account(mut, constraint = auction_config.authority == authority.key() @CustomErrorCode::IncorrectAuthority)]
    pub config_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [
            constants::CONFIG,
            auction_config.authority.as_ref(),
            mint.key().as_ref(),
        ],
        bump,
    )]
    pub auction_config: Box<Account<'info, AuctionConfig>>,

    #[account(
        mut, 
        seeds = [constants::AUCTION, auction_config.key().as_ref()],
        bump,
    )]
    auction_account: Account<'info, AuctionAccount>,

    #[account(
        mut,
        constraint = escrow_token_account.key() == auction_account.escrow_account @CustomErrorCode::InvalidEscrow,
    )]
    escrow_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
    bidder_token_account: Account<'info, TokenAccount>,
    
    /// CHECK: Using the constraint above, we verify that the pubkey of this account matches the auction authority
    #[account(
        mut,
        constraint = auction_config.global_vault.key() == global_vault.key() @CustomErrorCode::MismatchedGlobalVault,
    )]
    global_vault: UncheckedAccount<'info>,
    mint: Account<'info, Mint>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> CloseAuction<'info> {
    fn into_transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Transfer {
            authority: self.auction_account.to_account_info(),
            from: self.escrow_token_account.to_account_info(),
            to: self.holder_token_account.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }

    fn into_close_account_ctx(&self) -> CpiContext<'_, '_, '_, 'info, CloseTokenAccount<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = CloseTokenAccount {
            account: self.escrow_token_account.to_account_info(),
            authority: self.auction_account.to_account_info(),
            destination: self.authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'info> Bid<'info> {
    fn into_transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Transfer {
            authority: self.auction_account.to_account_info(),
            from: self.escrow_token_account.to_account_info(),
            to: self.bidder_token_account.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[account]
pub struct AuctionAccount {
    bump: u8,
    authority: Pubkey,
    amount: u64,
    escrow_account: Pubkey,
    starting_price: u32,
    starting_time: i64,
    auction_period: i64,
    current_auction_slot_count: u32,
}

const DISCRIMINATOR_LENGTH: usize = 8;
const PUBLIC_KEY_LENGTH: usize = 32;
const TIMESTAMP_LENGTH: usize = 8;
const U8_LENGTH: usize = 1;
const U32_LENGTH: usize = 4;
const U64_LENGTH: usize = 8;

impl AuctionAccount {
    const LEN: usize = 
    DISCRIMINATOR_LENGTH        // discriminator
        + U8_LENGTH             // bump
        + PUBLIC_KEY_LENGTH     // authority
        + U64_LENGTH            // amount
        + PUBLIC_KEY_LENGTH     // escrow account
        + U32_LENGTH            // starting price
        + TIMESTAMP_LENGTH      // starting time
        + TIMESTAMP_LENGTH      // auction period
        + U32_LENGTH;           // current_auction_slot_count
}