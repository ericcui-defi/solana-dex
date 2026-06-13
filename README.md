# Solana DEX — Constant-Product AMM

A Uniswap-v2-style automated market maker built on Solana with [Anchor](https://www.anchor-lang.com/), written from scratch as a learning project. Not audited, not production-ready — the goal is understanding Solana's account model, PDAs, CPIs, and SPL Token mechanics end-to-end.

## How it works

Each pool holds two SPL tokens and maintains the constant-product invariant `x · y = k`. Liquidity providers deposit both tokens and receive LP tokens (a freshly-minted SPL mint per pool) representing their pro-rata share. Traders swap against the pool's reserves; a configurable fee (in basis points) stays in the pool, accruing to LPs.

Key design points:

- **Pool, vaults, and LP mint are all PDAs.** The pool is derived from `["pool", mint_a, mint_b]`; the vaults and LP mint are derived from the pool. The pool PDA is the authority over its vaults and LP mint, signing CPIs via `invoke_signed`.
- **Canonical mint ordering.** `initialize` requires `mint_a.key() < mint_b.key()`, so exactly one pool address can ever exist for a given pair — no fragmentation, no decoy pools.
- **Cached reserves.** Swap and liquidity math uses `pool.reserve_a/b` (program-tracked state), never the raw vault balances, so donating tokens to a vault can't manipulate prices.
- **First deposit mints `sqrt(a · b)` LP tokens** (geometric mean — the unique swap-invariant liquidity measure); later deposits mint `min(a · supply / reserve_a, b · supply / reserve_b)`, so off-ratio deposits donate the excess to existing LPs instead of enabling a drain.
- **MINIMUM_LIQUIDITY floor.** First deposit must mint more than 1000 LP shares, and `remove_liquidity` rejects any withdrawal that would leave LP supply in `(0, 1000)`. Keeps the supply-1 inflation attack permanently unreachable, both at pool birth and via subsequent exits.
- **Fees are implicit.** Swaps quote output on the fee-discounted input (`in_eff = in · (10000 − fee_bps) / 10000`) but deposit the full input, so `k` grows with every trade. LPs realize fees on withdrawal via pro-rata payout.
- **Slippage protection** via client-computed floors (`min_lp_out`, `min_out`) passed as instruction arguments.
- All math uses u128 intermediates and checked arithmetic.

## Instructions

| Instruction | Args | Description |
|---|---|---|
| `initialize` | `fee_bps: u16` | Creates the pool, both vaults, and the LP mint |
| `add_liquidity` | `a_amount, b_amount, min_lp_out` | Deposits both tokens, mints LP tokens |
| `swap` | `a_to_b: bool, in_amount, min_out` | Swaps one token for the other at the constant-product price |
| `remove_liquidity` | `lp_amount` | Burns LP tokens, pays out a pro-rata share of both reserves |

## Project structure

```
programs/dex/src/
├── lib.rs              # Program entry points
├── state.rs            # Pool account (mints, vaults, lp_mint, reserves, fee_bps, bump)
├── error.rs            # Custom error codes
└── instructions/
    ├── initialize.rs
    ├── add_liquidity.rs
    ├── swap.rs
    └── remove_liquidity.rs
programs/dex/tests/     # LiteSVM integration tests — one per instruction, plus test_adversarial.rs
```

## Building and testing

Requires Rust 1.85+, Solana CLI 3.x (Agave), and Anchor 1.0.x.

```sh
anchor build   # compile the program (also produces target/deploy/dex.so used by tests)
cargo test     # run LiteSVM integration tests
```

Tests run against [LiteSVM](https://github.com/LiteSVM/litesvm), a lightweight in-process Solana VM — no local validator needed. Each test deploys the compiled program, builds raw instructions from Anchor's generated `instruction`/`accounts` modules, and asserts full post-state: user balances, vault balances, cached reserves, and LP supply.

`test_adversarial.rs` covers failure paths:

- Reverse-ordered mints rejected with `UnorderedMints`; no pool state survives
- Sub-floor first deposit (`sqrt(a · b) ≤ 1000`) rejected with `PoolTooSmall`

## Known simplifications

Deliberate omissions, since this is a learning build:

- `remove_liquidity` has no slippage floors (`min_a_out` / `min_b_out`)
- Fee is per-pool but fixed at initialization; no protocol fee
- No `emit!` events for indexers
- No frontend / TypeScript client
