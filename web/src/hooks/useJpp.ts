import { useState, useEffect, useRef, useCallback } from 'react'
import JppWorker from '../worker/jpp.worker?worker'

export type JppResult =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }

type WorkerResponse = {
  id: number
} & (
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }
)

const DEBOUNCE_MS = 300

export function useJpp(jsonpath: string, json: string): JppResult {
  const [result, setResult] = useState<JppResult>({ status: 'idle' })
  const workerRef = useRef<Worker | null>(null)
  const requestIdRef = useRef(0)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Initialize worker
  useEffect(() => {
    workerRef.current = new JppWorker()

    workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const { id, ...response } = e.data
      // Only accept the latest request
      if (id === requestIdRef.current) {
        setResult(response)
      }
    }

    return () => {
      workerRef.current?.terminate()
    }
  }, [])

  // Debounced execution
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current)
    }

    if (!jsonpath.trim() || !json.trim()) {
      setResult({ status: 'idle' })
      return
    }

    setResult({ status: 'loading' })

    debounceRef.current = setTimeout(() => {
      requestIdRef.current += 1
      workerRef.current?.postMessage({
        id: requestIdRef.current,
        jsonpath,
        json,
      })
    }, DEBOUNCE_MS)

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current)
      }
    }
  }, [jsonpath, json])

  return result
}
