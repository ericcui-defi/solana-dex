use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::state::Pool;

#[derive(Accounts)]
pub struct Initialize<'info> {

    // paying account
    #[account(mut)]
    pub payer: Signer<'info>,

    // References to token mints
    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,


    // Pool account
    #[account(
        init,
        payer = payer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [b"pool", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Account<'info, Pool>,


    // Token vault accounts
    #[account(
        init,
        payer = payer,
        token::mint = token_mint_a,
        token::authority = pool,
        seeds = [b"vault_a", pool.key().as_ref()],
        bump,
    )]
    pub token_vault_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = token_mint_b,
        token::authority = pool,
        seeds = [b"vault_b", pool.key().as_ref()],
        bump,
    )]
    pub token_vault_b: Account<'info, TokenAccount>,

    // Initializing LP mint account
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = pool,
        seeds = [b"lp", pool.key().as_ref()],
        bump
    )]
    pub lp_mint: Account<'info, Mint>,

    // System program reference
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,

}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    
    let pool = &mut ctx.accounts.pool;
    pool.token_mint_a = ctx.accounts.token_mint_a.key();
    pool.token_mint_b = ctx.accounts.token_mint_b.key();
    pool.token_vault_a = ctx.accounts.token_vault_a.key();
    pool.token_vault_b = ctx.accounts.token_vault_b.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.reserve_a = 0;
    pool.reserve_b = 0;
    pool.bump = ctx.bumps.pool;
    Ok(())
}
