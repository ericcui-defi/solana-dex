import { useState } from 'react'
import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js'
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
} from '@solana/spl-token'
import {
  BN,
  PhantomProvider,
  ata,
  connection,
  lpMintPda,
  makeProgram,
  poolPda,
  sortMints,
  vaultPda,
} from '../solana'

interface Props {
  phantom: PhantomProvider
}

export function CreatePool({ phantom }: Props) {
  const [mintA, setMintA] = useState('')
  const [mintB, setMintB] = useState('')
  const [feeBps, setFeeBps] = useState('30')
  const [depositA, setDepositA] = useState('')
  const [depositB, setDepositB] = useState('')
  const [status, setStatus] = useState<string>('')
  const [busy, setBusy] = useState(false)

  async function submit() {
    setStatus('')
    setBusy(true)
    try {
      // Parse + sort. The program rejects unordered mints, so we sort here
      // and remap the deposit amounts to match.
      const xMint = new PublicKey(mintA)
      const yMint = new PublicKey(mintB)
      const xAmt = BigInt(depositA)
      const yAmt = BigInt(depositB)
      const { mintA: a, mintB: b } = sortMints(xMint, yMint)
      const aAmount = a.equals(xMint) ? xAmt : yAmt
      const bAmount = a.equals(xMint) ? yAmt : xAmt

      const pool = poolPda(a, b)
      const vaultA = vaultPda(pool, 'a')
      const vaultB = vaultPda(pool, 'b')
      const lpMint = lpMintPda(pool)
      const userA = ata(a, phantom.publicKey)
      const userB = ata(b, phantom.publicKey)
      const userLp = ata(lpMint, phantom.publicKey)

      const program = makeProgram(phantom)

      // Build one transaction that:
      //  1. initialize     — creates pool, vaults, LP mint
      //  2. create userLp ATA — Anchor's #[account(token::mint = lp_mint)] expects an existing token account
      //  3. add_liquidity — seeds the pool (an empty pool can't be traded against)
      const tx = new Transaction()

      tx.add(
        await program.methods
          .initialize(parseInt(feeBps))
          .accounts({
            payer: phantom.publicKey,
            tokenMintA: a,
            tokenMintB: b,
            pool,
            tokenVaultA: vaultA,
            tokenVaultB: vaultB,
            lpMint,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .instruction(),
      )

      // Create user's LP token ATA (idempotent-style: only added if it doesn't exist).
      const userLpInfo = await connection.getAccountInfo(userLp)
      if (!userLpInfo) {
        tx.add(
          createAssociatedTokenAccountInstruction(
            phantom.publicKey,
            userLp,
            phantom.publicKey,
            lpMint,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID,
          ),
        )
      }

      tx.add(
        await program.methods
          .addLiquidity(new BN(aAmount.toString()), new BN(bAmount.toString()), new BN(1))
          .accounts({
            user: phantom.publicKey,
            pool,
            tokenVaultA: vaultA,
            tokenVaultB: vaultB,
            userA,
            userB,
            userLp,
            lpMint,
            tokenMintA: a,
            tokenMintB: b,
            tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .instruction(),
      )

      tx.feePayer = phantom.publicKey
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash

      // Simulate ourselves — Phantom's "Advanced" view hides program logs.
      // RPC returns the full log stream on failure so we can pinpoint the revert.
      const sim = await connection.simulateTransaction(tx)
      console.log('simulation result:', sim.value)
      if (sim.value.err) {
        const logs = sim.value.logs?.join('\n') ?? '(no logs)'
        setStatus(`✗ simulation failed: ${JSON.stringify(sim.value.err)}\n\n${logs}`)
        return
      }

      const signed = await phantom.signTransaction(tx)
      const sig = await connection.sendRawTransaction(signed.serialize())
      await connection.confirmTransaction(sig, 'confirmed')

      setStatus(`✓ pool ${pool.toBase58()} created. tx: ${sig}`)
    } catch (e: any) {
      setStatus(`✗ ${e.message ?? String(e)}`)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="panel">
      <h2>Create Pool</h2>
      <p className="hint">
        Initializes the pool and seeds it with initial liquidity in a single transaction.
        Mints are automatically sorted; deposit amounts follow.
      </p>
      <label>Token A mint
        <input value={mintA} onChange={(e) => setMintA(e.target.value)} placeholder="mint pubkey" />
      </label>
      <label>Token B mint
        <input value={mintB} onChange={(e) => setMintB(e.target.value)} placeholder="mint pubkey" />
      </label>
      <label>Fee (bps)
        <input value={feeBps} onChange={(e) => setFeeBps(e.target.value)} placeholder="30 = 0.3%" />
      </label>
      <label>Initial deposit A (raw units)
        <input value={depositA} onChange={(e) => setDepositA(e.target.value)} placeholder="e.g. 1000000" />
      </label>
      <label>Initial deposit B (raw units)
        <input value={depositB} onChange={(e) => setDepositB(e.target.value)} placeholder="e.g. 1000000" />
      </label>
      <button disabled={busy} onClick={submit}>{busy ? 'Submitting…' : 'Create pool'}</button>
      {status && <pre className="status">{status}</pre>}
    </div>
  )
}
