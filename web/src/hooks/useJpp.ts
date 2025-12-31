import { useSyncExternalStore } from 'react'
import JppWorker from '../worker/jpp.worker?worker'

export type JppResult =
  | { status: 'idle' }
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }

type WorkerResponse = {
  id: number
} & (
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }
)

const worker = new JppWorker()
let currentId = 0
let currentResult: JppResult = { status: 'idle' }
let listeners: Set<() => void> = new Set()

worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
  const { id, ...response } = e.data
  if (id === currentId) {
    currentResult = response
    listeners.forEach((l) => l())
  }
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

function getSnapshot() {
  return currentResult
}

function query(jsonpath: string, json: string) {
  if (!jsonpath.trim() || !json.trim()) {
    currentResult = { status: 'idle' }
    listeners.forEach((l) => l())
    return
  }
  currentId += 1
  worker.postMessage({ id: currentId, jsonpath, json })
}

export function useJpp(jsonpath: string, json: string): JppResult {
  const result = useSyncExternalStore(subscribe, getSnapshot)

  // Query on every render with new inputs
  const key = jsonpath + '\0' + json
  if ((query as any).lastKey !== key) {
    (query as any).lastKey = key
    query(jsonpath, json)
  }

  return result
}
