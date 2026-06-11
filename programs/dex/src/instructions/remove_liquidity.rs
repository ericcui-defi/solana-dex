use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, Burn};
use crate::state::Pool;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {

    // User account removing liquidity to the pool
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
    pub token_vault_a: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_vault_b: Box<Account<'info, TokenAccount>>,

    // Reference to token_a custodying account
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = user,
    )]
    pub user_a: Box<Account<'info, TokenAccount>>,

    // Reference to token_b custodying account
    #[account(
        mut,
        token::mint = token_mint_b,
        token::authority = user,
    )]
    pub user_b: Box<Account<'info, TokenAccount>>,

    // Reference to user lp token account
    #[account(
        mut,
        token::mint = lp_mint,
        token::authority = user,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    // Reference to LP mint
    #[account(mut)]
    pub lp_mint: Box<Account<'info, Mint>>,

    // Read-only references to the token mints trading in this DEX
    pub token_mint_a: Box<Account<'info, Mint>>,
    pub token_mint_b: Box<Account<'info, Mint>>,

    // Reference to orchestrating token program
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<RemoveLiquidity>, lp_amount: u64) -> Result<()> {

    // Verifying nonzero removal
    require!(lp_amount > 0, ErrorCode::ZeroWithdrawal);

    // Verify user actually has the amount of liquidity they are trying to withdraw
    

    // References to a and b dex vault token amounts
    let a_reserve = ctx.accounts.pool.reserve_a;
    let b_reserve = ctx.accounts.pool.reserve_b;

    // Calculating token a and token b payout amount
    let token_a_payout = (lp_amount as u128  * a_reserve as u128) / ctx.accounts.lp_mint.supply as u128;
    let token_b_payout = (lp_amount as u128 * b_reserve as u128) / ctx.accounts.lp_mint.supply as u128;

    // References necessary for PDA
    let mint_a_key = ctx.accounts.token_mint_a.key();
    let mint_b_key = ctx.accounts.token_mint_b.key();
    let bump = ctx.accounts.pool.bump;

    // Creating PDA seed
    let seeds: &[&[&[u8]]] = &[&[b"pool", mint_a_key.as_ref(), mint_b_key.as_ref(), &[bump]]];

    // Transferring user_a tokens into vault_a
    let cpi_a = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from : ctx.accounts.token_vault_a.to_account_info(),
            to: ctx.accounts.user_a.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        },
        seeds
    );
    token::transfer(cpi_a, token_a_payout as u64)?;

    // Transferring user_b tokens into vault_b
    let cpi_b = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.token_vault_b.to_account_info(),
            to: ctx.accounts.user_b.to_account_info(), 
            authority: ctx.accounts.pool.to_account_info(),
        },
        seeds
    );
    token::transfer(cpi_b, token_b_payout as u64)?;

    // Update dex vault tracked reserve values
    let pool = &mut ctx.accounts.pool;
    pool.reserve_a = pool.reserve_a.checked_sub(token_a_payout as u64).ok_or(ErrorCode::Overflow)?;
    pool.reserve_b = pool.reserve_b.checked_sub(token_b_payout as u64).ok_or(ErrorCode::Overflow)?;

    // Building lp mint/transfer CPI context and firing
    let cpi_burn = CpiContext::new(
        ctx.accounts.token_program.key(),
        Burn {
            mint: ctx.accounts.lp_mint.to_account_info(),
            from: ctx.accounts.user_lp.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }
    );
    token::burn(cpi_burn, lp_amount)?;

    Ok(())
}