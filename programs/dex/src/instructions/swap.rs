use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::state::Pool;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Swap<'info> {

    // User account swapping tokens
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

    // Reference to token_a user custodying account
    #[account(
        mut,
        token::mint = token_mint_a,
        token::authority = user,
    )]
    pub user_a: Box<Account<'info, TokenAccount>>,

    // Reference to token_b user custodying account
    #[account(
        mut, 
        token::mint = token_mint_b,
        token::authority = user,
    )]
    pub user_b: Box<Account<'info, TokenAccount>>,

    // Reference to LP mint
    #[account(mut)]
    pub lp_mint: Box<Account<'info, Mint>>,

    // Read-only references to the mints of the tokens trading in this DEX
    pub token_mint_a: Box<Account<'info, Mint>>,
    pub token_mint_b: Box<Account<'info, Mint>>,

    // Reference to orchestrating token program
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Swap>, a_to_b: bool, in_amount: u64, min_out: u64) -> Result<()> {

    // Ensuring against zero deposited funds
    require!(in_amount > 0, ErrorCode::ZeroDeposit);

    let (a, b, vault_in, vault_out, user_in, user_out) = if a_to_b {
        (ctx.accounts.pool.reserve_a as u128, ctx.accounts.pool.reserve_b as u128, &ctx.accounts.token_vault_a, &ctx.accounts.token_vault_b, &ctx.accounts.user_a, &ctx.accounts.user_b)
    } else {
        (ctx.accounts.pool.reserve_b as u128 , ctx.accounts.pool.reserve_a as u128, &ctx.accounts.token_vault_b, &ctx.accounts.token_vault_a, &ctx.accounts.user_b, &ctx.accounts.user_a)
    };

    // References to vault values and effective in calculation
    let in_eff = in_amount as u128 * (10_000 - ctx.accounts.pool.fee_bps) as u128 / 10_000;
    
    // Calculating payout
    let out = (b * in_eff) / (a + in_eff);
    require!(out > 0, ErrorCode::LPDust);
    require!(out as u64 >= min_out, ErrorCode::SlippageExceeded);

    // Transferring user funds into pool
    let cpi_in = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: user_in.to_account_info(),
            to: vault_in.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }
    );
    token::transfer(cpi_in, in_amount)?;

    // Creating PDA seed
    let mint_a_key = ctx.accounts.pool.token_mint_a;
    let mint_b_key = ctx.accounts.pool.token_mint_b;
    let bump = ctx.accounts.pool.bump;
    let seeds: &[&[&[u8]]] = &[&[b"pool", mint_a_key.as_ref(), mint_b_key.as_ref(), &[bump]]];

    // Transferring pool funds to user
    let cpi_out = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: vault_out.to_account_info(),
            to: user_out.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        },
        seeds
    );
    token::transfer(cpi_out, out as u64)?;

    // Reserve updates
    let pool = &mut ctx.accounts.pool;
    if a_to_b {
        pool.reserve_a = pool.reserve_a.checked_add(in_amount).ok_or(ErrorCode::Overflow)?;
        pool.reserve_b = pool.reserve_b.checked_sub(out as u64).ok_or(ErrorCode::Overflow)?;
    } else {
        pool.reserve_b = pool.reserve_b.checked_add(in_amount).ok_or(ErrorCode::Overflow)?;
        pool.reserve_a = pool.reserve_a.checked_sub(out as u64).ok_or(ErrorCode::Overflow)?;
    }

    Ok(())
}