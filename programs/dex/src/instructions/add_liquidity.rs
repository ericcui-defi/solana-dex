use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::state::Pool;
use crate::error::ErrorCode;

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

    // Reference to orchestrating token program
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<AddLiquidity>, a_amount: u64, b_amount: u64, min_lp_out: u64) -> Result<()> {

    // Verifying nonzero deposit
    require!(a_amount > 0, ErrorCode::ZeroDeposit);
    require!(b_amount > 0, ErrorCode::ZeroDeposit);


    // References to a and b dex vault token amounts
    let a_reserve = ctx.accounts.pool.reserve_a;
    let b_reserve = ctx.accounts.pool.reserve_b;

    // u64 pre-mint LP token supply
    let supply = ctx.accounts.lp_mint.supply;

    // Calculating LP tokens to payout
    let lp_to_mint = if supply == 0 {
        (a_amount as u128 * b_amount as u128).isqrt() as u64
    } else {
        (a_amount as u128 * supply as u128 / a_reserve as u128).min(b_amount as u128 * supply as u128 / b_reserve as u128) as u64
    };

    // Min LP out + dust check
    require!(lp_to_mint >= min_lp_out, ErrorCode::SlippageExceeded);
    require!(lp_to_mint > 0, ErrorCode::LPDust);

    // Transferring user_a tokens into vault_a
    let cpi_a = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from :ctx.accounts.user_a.to_account_info(),
            to: ctx.accounts.token_vault_a.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }
    );
    token::transfer(cpi_a, a_amount)?;

    // Transferring user_b tokens into vault_b
    let cpi_b = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.user_b.to_account_info(),
            to: ctx.accounts.token_vault_b.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }
    );
    token::transfer(cpi_b, b_amount)?;



    Ok(())
}