use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub token_mint_a: Pubkey, 
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub lp_mint: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
    pub bump: u8,
}