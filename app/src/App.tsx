import { useEffect, useState } from 'react'
import { CreatePool } from './components/CreatePool'
import { TradePool } from './components/TradePool'
import { PhantomProvider, getPhantom, RPC_URL } from './solana'

type Tab = 'create' | 'trade'

export default function App() {
  const [phantom, setPhantom] = useState<PhantomProvider | null>(null)
  const [tab, setTab] = useState<Tab>('create')

  useEffect(() => {
    const p = getPhantom()
    if (p?.isConnected) setPhantom(p)
  }, [])

  async function connect() {
    const p = getPhantom()
    if (!p) {
      alert('Phantom not detected — install the browser extension first.')
      return
    }
    await p.connect()
    setPhantom(p)
  }

  return (
    <div className="root">
      <header>
        <h1>DEX Client</h1>
        <div className="meta">
          <span className="rpc">{RPC_URL}</span>
          {phantom ? (
            <span className="pubkey">{phantom.publicKey.toBase58().slice(0, 8)}…</span>
          ) : (
            <button onClick={connect}>Connect Phantom</button>
          )}
        </div>
      </header>

      <nav>
        <button className={tab === 'create' ? 'active' : ''} onClick={() => setTab('create')}>
          Create Pool
        </button>
        <button className={tab === 'trade' ? 'active' : ''} onClick={() => setTab('trade')}>
          Trade
        </button>
      </nav>

      <main>
        {phantom ? (
          tab === 'create' ? <CreatePool phantom={phantom} /> : <TradePool phantom={phantom} />
        ) : (
          <div className="panel"><p>Connect a Phantom wallet to start.</p></div>
        )}
      </main>
    </div>
  )
}
