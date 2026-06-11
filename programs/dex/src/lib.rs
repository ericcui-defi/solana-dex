pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BZX5d8ghr3p5Q3wHe3HdDZgcY91FEbBsA1Qtb7FippfT");

#[program]
pub mod dex {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        initialize::handler(ctx, fee_bps)
    }
    pub fn add_liquidity(ctx: Context<AddLiquidity>, a_amount: u64, b_amount: u64, min_lp_out: u64) -> Result<()> {
        add_liquidity::handler(ctx, a_amount, b_amount, min_lp_out)
    }
    pub fn swap(ctx: Context<Swap>, a_to_b: bool, in_amount: u64, min_out: u64) -> Result<()> {
        swap::handler(ctx, a_to_b, in_amount, min_out)
    }
    pub fn remove_liquidity(ctx: Context<RemoveLiquidity>, lp_amount: u64) -> Result<()> {
        remove_liquidity::handler(ctx, lp_amount)
    }
}
