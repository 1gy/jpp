import init, { query } from '../../wasm/jpp_wasm'

type WorkerRequest = {
  id: number
  jsonpath: string
  json: string
}

type WorkerResponse = {
  id: number
} & (
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }
)

let initialized = false

async function ensureInit() {
  if (!initialized) {
    await init()
    initialized = true
  }
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const { id, jsonpath, json } = e.data

  try {
    await ensureInit()
    const result = query(jsonpath, json)
    self.postMessage({ id, status: 'success', data: result } satisfies WorkerResponse)
  } catch (err) {
    self.postMessage({ id, status: 'error', message: String(err) } satisfies WorkerResponse)
  }
}
