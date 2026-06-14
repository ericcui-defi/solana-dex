import { useState } from 'react'
import { PublicKey, Transaction } from '@solana/web3.js'
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
  quoteSwap,
  vaultPda,
} from '../solana'

interface Props {
  phantom: PhantomProvider
}

type PoolInfo = {
  address: PublicKey
  mintA: PublicKey
  mintB: PublicKey
  feeBps: number
  reserveA: bigint
  vaultA: bigint
  reserveB: bigint
  vaultB: bigint
  lpSupply: bigint
  decimalsA: number
  decimalsB: number
}

type UserBalance = {
  ata: PublicKey
  exists: boolean
  amount: bigint
}

type UserBalances = { a: UserBalance; b: UserBalance }

type Direction = 'aToB' | 'bToA'

function formatAmount(amount: bigint, decimals: number): string {
  const divisor = 10n ** BigInt(decimals)
  const whole = amount / divisor
  const frac = amount % divisor
  if (frac === 0n) return whole.toString()
  const fracStr = frac.toString().padStart(decimals, '0').replace(/0+$/, '')
  return `${whole}.${fracStr}`
}

export function TradePool({ phantom }: Props) {
  const [poolAddress, setPoolAddress] = useState('')
  const [pool, setPool] = useState<PoolInfo | null>(null)
  const [balances, setBalances] = useState<UserBalances | null>(null)
  const [poolError, setPoolError] = useState('')

  const [direction, setDirection] = useState<Direction>('aToB')
  const [inAmount, setInAmount] = useState('')
  const [slippagePct, setSlippagePct] = useState('1')
  const [quote, setQuote] = useState('')
  const [status, setStatus] = useState('')
  const [busy, setBusy] = useState(false)

  async function inspectPool() {
    setPool(null)
    setBalances(null)
    setPoolError('')
    setQuote('')
    setStatus('')
    try {
      const poolAddr = new PublicKey(poolAddress)
      const program = makeProgram(phantom)
      const state: any = await (program.account as any).pool.fetch(poolAddr)

      const vaultAddrA = vaultPda(poolAddr, 'a')
      const vaultAddrB = vaultPda(poolAddr, 'b')
      const lpMint = lpMintPda(poolAddr)

      const [vaultBalA, vaultBalB, supply] = await Promise.all([
        connection.getTokenAccountBalance(vaultAddrA),
        connection.getTokenAccountBalance(vaultAddrB),
        connection.getTokenSupply(lpMint),
      ])

      const next: PoolInfo = {
        address: poolAddr,
        mintA: state.tokenMintA,
        mintB: state.tokenMintB,
        feeBps: state.feeBps,
        reserveA: BigInt(state.reserveA.toString()),
        vaultA: BigInt(vaultBalA.value.amount),
        reserveB: BigInt(state.reserveB.toString()),
        vaultB: BigInt(vaultBalB.value.amount),
        lpSupply: BigInt(supply.value.amount),
        decimalsA: vaultBalA.value.decimals,
        decimalsB: vaultBalB.value.decimals,
      }
      setPool(next)

      // Compute user ATAs from the pool's mints. Querying balance throws if the
      // SPL token account doesn't exist — distinguishes "exists with 0" from
      // "no account at all".
      const userAtaA = ata(next.mintA, phantom.publicKey)
      const userAtaB = ata(next.mintB, phantom.publicKey)
      const [balA, balB] = await Promise.all([
        connection.getTokenAccountBalance(userAtaA).catch(() => null),
        connection.getTokenAccountBalance(userAtaB).catch(() => null),
      ])
      setBalances({
        a: balA
          ? { ata: userAtaA, exists: true, amount: BigInt(balA.value.amount) }
          : { ata: userAtaA, exists: false, amount: 0n },
        b: balB
          ? { ata: userAtaB, exists: true, amount: BigInt(balB.value.amount) }
          : { ata: userAtaB, exists: false, amount: 0n },
      })
    } catch (e: any) {
      setPoolError(e.message ?? String(e))
    }
  }

  function getQuote() {
    setQuote('')
    if (!pool) return
    try {
      const aToB = direction === 'aToB'
      const reserveIn = aToB ? pool.reserveA : pool.reserveB
      const reserveOut = aToB ? pool.reserveB : pool.reserveA
      const dIn = aToB ? pool.decimalsA : pool.decimalsB
      const dOut = aToB ? pool.decimalsB : pool.decimalsA
      const inAmt = BigInt(inAmount)
      const out = quoteSwap(inAmt, reserveIn, reserveOut, pool.feeBps)
      setQuote(
        `expected out: ${out} raw (${formatAmount(out, dOut)} tokens)\n` +
          `sending: ${inAmt} raw (${formatAmount(inAmt, dIn)} tokens)\n` +
          `fee: ${pool.feeBps} bps`,
      )
    } catch (e: any) {
      setQuote(`✗ ${e.message ?? String(e)}`)
    }
  }

  async function submit() {
    setStatus('')
    setBusy(true)
    try {
      if (!pool || !balances) throw new Error('inspect a pool first')
      const aToB = direction === 'aToB'
      const sourceBal = aToB ? balances.a : balances.b
      const destBal = aToB ? balances.b : balances.a
      const destMint = aToB ? pool.mintB : pool.mintA

      if (!sourceBal.exists) {
        throw new Error('no token account for the source mint — acquire some of that token first')
      }
      const inAmt = BigInt(inAmount)
      if (inAmt === 0n) throw new Error('amount must be > 0')
      if (inAmt > sourceBal.amount) {
        throw new Error(`insufficient balance (have ${sourceBal.amount}, need ${inAmt})`)
      }

      const reserveIn = aToB ? pool.reserveA : pool.reserveB
      const reserveOut = aToB ? pool.reserveB : pool.reserveA
      const expected = quoteSwap(inAmt, reserveIn, reserveOut, pool.feeBps)
      const slippageBps = BigInt(Math.round(parseFloat(slippagePct) * 100))
      const minOut = (expected * (10_000n - slippageBps)) / 10_000n

      const program = makeProgram(phantom)
      const vaultAddrA = vaultPda(pool.address, 'a')
      const vaultAddrB = vaultPda(pool.address, 'b')
      const lpMint = lpMintPda(pool.address)

      const tx = new Transaction()
      // Bundle ATA creation for the destination mint if missing — saves the
      // user a separate "create account" tx and lets the swap settle in one shot.
      if (!destBal.exists) {
        tx.add(
          createAssociatedTokenAccountInstruction(
            phantom.publicKey,
            destBal.ata,
            phantom.publicKey,
            destMint,
            TOKEN_PROGRAM_ID,
            ASSOCIATED_TOKEN_PROGRAM_ID,
          ),
        )
      }
      tx.add(
        await program.methods
          .swap(aToB, new BN(inAmt.toString()), new BN(minOut.toString()))
          .accounts({
            user: phantom.publicKey,
            pool: pool.address,
            tokenVaultA: vaultAddrA,
            tokenVaultB: vaultAddrB,
            userA: balances.a.ata,
            userB: balances.b.ata,
            lpMint,
            tokenMintA: pool.mintA,
            tokenMintB: pool.mintB,
            tokenProgram: TOKEN_PROGRAM_ID,
          } as any)
          .instruction(),
      )
      tx.feePayer = phantom.publicKey
      tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash

      const sim = await connection.simulateTransaction(tx)
      if (sim.value.err) {
        const logs = sim.value.logs?.join('\n') ?? '(no logs)'
        setStatus(`✗ simulation failed: ${JSON.stringify(sim.value.err)}\n\n${logs}`)
        return
      }

      const signed = await phantom.signTransaction(tx)
      const sig = await connection.sendRawTransaction(signed.serialize())
      await connection.confirmTransaction(sig, 'confirmed')
      setStatus(`✓ swap landed. tx: ${sig}`)

      // Refresh pool + balances so the UI reflects the new state.
      await inspectPool()
    } catch (e: any) {
      setStatus(`✗ ${e.message ?? String(e)}`)
    } finally {
      setBusy(false)
    }
  }

  const sourceBal = pool && balances ? (direction === 'aToB' ? balances.a : balances.b) : null
  const destBal = pool && balances ? (direction === 'aToB' ? balances.b : balances.a) : null
  const sourceDecimals = pool ? (direction === 'aToB' ? pool.decimalsA : pool.decimalsB) : 0
  const sourceMint = pool ? (direction === 'aToB' ? pool.mintA : pool.mintB) : null
  const destMint = pool ? (direction === 'aToB' ? pool.mintB : pool.mintA) : null

  return (
    <div className="panel">
      <h2>Trade</h2>
      <p className="hint">Paste a pool address to load its state, your balances, and a swap UI.</p>

      <label>Pool address
        <input value={poolAddress} onChange={(e) => setPoolAddress(e.target.value)} placeholder="pool pubkey" />
      </label>
      <div className="row">
        <button type="button" onClick={inspectPool}>Inspect pool</button>
      </div>
      {poolError && <pre className="status">✗ {poolError}</pre>}

      {pool && (
        <div className="pool-inspector">
          <table className="kv">
            <tbody>
              <tr><th>mint A</th><td className="mono">{pool.mintA.toBase58()}</td></tr>
              <tr><th>mint B</th><td className="mono">{pool.mintB.toBase58()}</td></tr>
              <tr><th>fee</th><td>{pool.feeBps} bps</td></tr>
              <tr><th>LP supply</th><td className="mono">{pool.lpSupply.toString()}</td></tr>
            </tbody>
          </table>

          <table className="reserves">
            <thead>
              <tr>
                <th></th>
                <th>reserve (cached)</th>
                <th>vault (actual)</th>
                <th>Δ donation</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <th>token A</th>
                <td className="mono">{pool.reserveA.toString()}</td>
                <td className="mono">{pool.vaultA.toString()}</td>
                <td className={`mono ${pool.vaultA !== pool.reserveA ? 'diff' : ''}`}>
                  {(pool.vaultA - pool.reserveA).toString()}
                </td>
              </tr>
              <tr>
                <th>token B</th>
                <td className="mono">{pool.reserveB.toString()}</td>
                <td className="mono">{pool.vaultB.toString()}</td>
                <td className={`mono ${pool.vaultB !== pool.reserveB ? 'diff' : ''}`}>
                  {(pool.vaultB - pool.reserveB).toString()}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      )}

      {pool && balances && (
        <div className="section">
          <h3>Your balances</h3>
          <table className="kv">
            <tbody>
              <tr>
                <th>token A</th>
                <td>
                  {balances.a.exists ? (
                    <>
                      <span className="mono">{balances.a.amount.toString()} raw</span>
                      <span className="muted"> ({formatAmount(balances.a.amount, pool.decimalsA)} tokens)</span>
                    </>
                  ) : (
                    <span className="muted">no token account configured</span>
                  )}
                </td>
              </tr>
              <tr>
                <th>token B</th>
                <td>
                  {balances.b.exists ? (
                    <>
                      <span className="mono">{balances.b.amount.toString()} raw</span>
                      <span className="muted"> ({formatAmount(balances.b.amount, pool.decimalsB)} tokens)</span>
                    </>
                  ) : (
                    <span className="muted">no token account configured</span>
                  )}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      )}

      {pool && balances && sourceBal && destBal && sourceMint && destMint && (
        <div className="section">
          <h3>Swap</h3>
          <div className="toggle">
            <button
              type="button"
              className={direction === 'aToB' ? 'active' : ''}
              onClick={() => { setDirection('aToB'); setQuote('') }}
            >
              A → B
            </button>
            <button
              type="button"
              className={direction === 'bToA' ? 'active' : ''}
              onClick={() => { setDirection('bToA'); setQuote('') }}
            >
              B → A
            </button>
          </div>

          <p className="hint">
            Sending <span className="mono">{sourceMint.toBase58().slice(0, 8)}…</span>
            {' → receiving '}
            <span className="mono">{destMint.toBase58().slice(0, 8)}…</span>
            {!destBal.exists && (
              <span className="muted"> (destination ATA missing — will be auto-created with the swap)</span>
            )}
          </p>

          {!sourceBal.exists ? (
            <pre className="status">
              ✗ You don't hold a token account for the source mint. Acquire some of that token first
              (e.g. via `spl-token transfer` or by minting if you control the mint).
            </pre>
          ) : sourceBal.amount === 0n ? (
            <pre className="status">
              ✗ Your source token account exists but has 0 balance. Top it up before swapping.
            </pre>
          ) : (
            <>
              <label>Amount in (raw units)
                <input value={inAmount} onChange={(e) => setInAmount(e.target.value)} placeholder="e.g. 1000000" />
                <span className="muted">
                  max: {sourceBal.amount.toString()} raw ({formatAmount(sourceBal.amount, sourceDecimals)} tokens)
                </span>
              </label>
              <label>Slippage tolerance (%)
                <input value={slippagePct} onChange={(e) => setSlippagePct(e.target.value)} placeholder="1" />
              </label>
              <div className="row">
                <button type="button" onClick={getQuote}>Quote</button>
                <button disabled={busy} onClick={submit}>{busy ? 'Submitting…' : 'Swap'}</button>
              </div>
              {quote && <pre className="status">{quote}</pre>}
              {status && <pre className="status">{status}</pre>}
            </>
          )}
        </div>
      )}
    </div>
  )
}
