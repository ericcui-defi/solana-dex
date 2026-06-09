use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::Pool;

#[derive(Accounts)]
pub struct AddLiquidity<'info> {

    // User account adding liquidity to the pool
    #[account(mut)]
    pub user: Signer<'info>,

    // Liquidity pool (verifying it is in fact the correct, canonical pool)
    #[account(
        mut,
        has_one = token_vault_a,
        has_one = token_vault_b,
        has_one = lp_mint,
        has_one = token_mint_a,
        has_one = token_mint_b,
    )]
    pub pool: Account<'info, Pool>,

    // References to the custodying TokenAccounts of the DEX to update reserve_* values
    #[account(mut)]
    pub token_vault_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_vault_b: Account<'info, TokenAccount>,

    // Reference to token_a custodying account
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = user,
    )]
    pub user_a: Account<'info, TokenAccount>,

    // Reference to token_b custodying account
    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = user,
    )]
    pub user_b: Account<'info, TokenAccount>,

    // Reference to user lp token account
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    // Reference to LP mint
    #[account(mut)]
    pub lp_mint: Account<'info, Mint>,

    // Read-only references to the token mints trading in this DEX
    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,

    // References to relevant programs (system for transferring into/out of accounts, token program for SPL token stuff)
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<AddLiquidity>, a_amount: u64, b_amount: u64) -> Result<()> {


    Ok(())
}