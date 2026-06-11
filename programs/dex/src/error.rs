use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,

    #[msg("Zero deposit balance")]
    ZeroDeposit,

    #[msg("Slippage exceeded")]
    SlippageExceeded,

    #[msg("LP Dust")]
    LPDust,

    #[msg("Overflow")]
    Overflow,

    #[msg("HighBPS")]
    HighBPS,

    #[msg("ZeroWithdrawal")]
    ZeroWithdrawal,
}
