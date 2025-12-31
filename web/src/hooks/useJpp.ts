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
let lastQuery = ''
const listeners = new Set<() => void>()

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
  const key = jsonpath + '\0' + json
  if (key === lastQuery) return
  lastQuery = key

  if (!jsonpath.trim() || !json.trim()) {
    currentResult = { status: 'idle' }
    listeners.forEach((l) => l())
    return
  }
  currentId += 1
  worker.postMessage({ id: currentId, jsonpath, json })
}

export function useJpp(jsonpath: string, json: string): JppResult {
  query(jsonpath, json)
  return useSyncExternalStore(subscribe, getSnapshot)
}
