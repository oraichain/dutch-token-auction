use anchor_lang::prelude::*;

#[error_code]
pub enum CustomErrorCode {
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
    #[msg("Auction global vault must match auction config global vault")]
    MismatchedGlobalVault,
    #[msg("Incorrect escrow token account")]
    InvalidEscrow,
    #[msg("Invalid fee")]
    InvalidFee,
    #[msg("Invalid fee burn")]
    InvalidFeeBurn,
    #[msg("IncorrectAuthority")]
    IncorrectAuthority,
    #[msg("The auction rounds for this escrow account has completed or has not started")]
    AuctionInvalid,
    #[msg("Invalid escrow amount")]
    InvalidEscrowAmount,
    #[msg("Previous auction round has not been ended")]
    PreviousRoundNotEnd,
}
