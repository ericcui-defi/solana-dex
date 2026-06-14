import { AnchorProvider, BN, Program, Wallet } from '@coral-xyz/anchor'
import { Connection, PublicKey, Transaction } from '@solana/web3.js'
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from '@solana/spl-token'
import idl from './idl.json'

export const PROGRAM_ID = new PublicKey(idl.address)
export const RPC_URL =
  (import.meta.env.VITE_RPC_URL as string) || 'https://api.devnet.solana.com'

export const connection = new Connection(RPC_URL, 'confirmed')

// Minimal wallet wrapper around window.solana (Phantom).
// Anchor's Wallet interface requires publicKey + signTransaction + signAllTransactions.
export interface PhantomProvider {
  publicKey: PublicKey
  isConnected: boolean
  connect: () => Promise<{ publicKey: PublicKey }>
  disconnect: () => Promise<void>
  signTransaction: (tx: Transaction) => Promise<Transaction>
  signAllTransactions: (txs: Transaction[]) => Promise<Transaction[]>
}

export function getPhantom(): PhantomProvider | null {
  const w = window as any
  return w.solana?.isPhantom ? (w.solana as PhantomProvider) : null
}

export function makeProgram(phantom: PhantomProvider): Program {
  const wallet: Wallet = {
    publicKey: phantom.publicKey,
    signTransaction: phantom.signTransaction.bind(phantom) as any,
    signAllTransactions: phantom.signAllTransactions.bind(phantom) as any,
    payer: null as any, // unused in browser; Anchor only needs the signers above
  }
  const provider = new AnchorProvider(connection, wallet, {
    commitment: 'confirmed',
  })
  return new Program(idl as any, provider)
}

// --- PDA derivation ---
// Mirrors the seeds used in the on-chain program (see programs/dex/src/instructions).

export function poolPda(mintA: PublicKey, mintB: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('pool'), mintA.toBuffer(), mintB.toBuffer()],
    PROGRAM_ID,
  )[0]
}

export function vaultPda(pool: PublicKey, side: 'a' | 'b'): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(side === 'a' ? 'vault_a' : 'vault_b'), pool.toBuffer()],
    PROGRAM_ID,
  )[0]
}

export function lpMintPda(pool: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('lp'), pool.toBuffer()],
    PROGRAM_ID,
  )[0]
}

// --- Mint sorting ---
// The on-chain program requires mint_a.key() < mint_b.key().
// Pubkey comparison is by raw bytes; toBuffer().compare() does exactly that.
export function sortMints(
  x: PublicKey,
  y: PublicKey,
): { mintA: PublicKey; mintB: PublicKey } {
  return x.toBuffer().compare(y.toBuffer()) < 0
    ? { mintA: x, mintB: y }
    : { mintA: y, mintB: x }
}

// --- Associated token account address (no creation here, just derivation) ---
export function ata(mint: PublicKey, owner: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    TOKEN_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  )
}

// --- Swap quote (mirrors on-chain math; integer-floor everywhere) ---
// in_eff = in_amount * (10000 - fee_bps) / 10000
// out    = reserve_out * in_eff / (reserve_in + in_eff)
export function quoteSwap(
  inAmount: bigint,
  reserveIn: bigint,
  reserveOut: bigint,
  feeBps: number,
): bigint {
  if (inAmount <= 0n || reserveIn <= 0n || reserveOut <= 0n) return 0n
  const inEff = (inAmount * BigInt(10_000 - feeBps)) / 10_000n
  return (reserveOut * inEff) / (reserveIn + inEff)
}

export { BN }
