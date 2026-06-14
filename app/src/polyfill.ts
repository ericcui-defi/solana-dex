// MUST be imported before any module that touches Buffer at init time
// (e.g. @solana/spl-token reads Buffer in its module body).
// ESM evaluates imports depth-first in declaration order, so making this
// the first import in main.tsx guarantees the assignment runs before
// subsequent imports start initializing.
import { Buffer } from 'buffer'
;(globalThis as any).Buffer = Buffer
