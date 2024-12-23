use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Close auction can only be called by the auction authority")]
    ProxyClose,
    #[msg("Auction has not yet begun")]
    AuctionEarly,
    #[msg("Auction has concluded")]
    AuctionLate,
    #[msg("Start date must occur before end date")]
    InvalidDateRange,
    #[msg("Start date must occur in the future")]
    InvalidStartDate,
    #[msg("Auction owner must match auction authority")]
    MismatchedOwners,
    #[msg("Incorrect escrow token account")]
    InvalidEscrow,
}
