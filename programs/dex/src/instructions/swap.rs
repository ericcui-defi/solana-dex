use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo};
use solana_keypair::keypair_from_seed;
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

pub fn handler(ctx: Context<Swap>, a_to_b: bool, in_amount: u64) -> Result<()> {

    let (a, b, vault_in, vault_out, user_in, user_out) = if a_to_b {
        (ctx.accounts.pool.reserve_a as u128, ctx.accounts.pool.reserve_b as u128, &ctx.accounts.token_vault_a, &ctx.accounts.token_vault_b, &ctx.accounts.user_a, &ctx.accounts.user_b)
    } else {
        (ctx.accounts.pool.reserve_b as u128 , ctx.accounts.pool.reserve_a as u128, &ctx.accounts.token_vault_b, &ctx.accounts.token_vault_a, &ctx.accounts.user_b, &ctx.accounts.user_a)
    };

    // References to vault values and effective in calculation
    let in_eff = in_amount as u128 * (10_000 - ctx.accounts.pool.fee_bps) as u128 / 10_000;
    
    // Calculating payout
    let out = (b * in_eff) / (a + in_eff);

    // Transferring user funds into pool
    let cpi_in = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: user_in.to_account_info(),
            to: vault_in.to_account_info(),
            authority: user_in.to_account_info(),
        }
    );
    token::transfer(cpi_in, in_eff as u64)?;

    // Transferrinf pool funds to user
    let cpi_out = CpiContext::new(
        ctx.accounts.token_program.key(),
        Transfer {
            from: vault_out.to_account_info(),
            to: user_out.to_account_info(),
            authority: ctx.accounts.pool.to_account_info(),
        }
    );
    token::transfer(cpi_out, out as u64)?;
    Ok(())
}