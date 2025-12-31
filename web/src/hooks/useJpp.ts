import { useState, useEffect, useRef } from 'react'
import JppWorker from '../worker/jpp.worker?worker'

export type JppData =
  | { status: 'idle' }
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }

export type JppResult = {
  loading: boolean
  result: JppData
}

type WorkerResponse = {
  id: number
} & (
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }
)

const LOADING_DELAY_MS = 100

export function useJpp(jsonpath: string, json: string): JppResult {
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<JppData>({ status: 'idle' })
  const workerRef = useRef<Worker | null>(null)
  const requestIdRef = useRef(0)
  const loadingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Initialize worker
  useEffect(() => {
    workerRef.current = new JppWorker()

    workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const { id, ...response } = e.data
      // Only accept the latest request
      if (id === requestIdRef.current) {
        // Cancel loading timer if result arrives quickly
        if (loadingTimerRef.current) {
          clearTimeout(loadingTimerRef.current)
          loadingTimerRef.current = null
        }
        setResult(response)
        setLoading(false)
      }
    }

    return () => {
      workerRef.current?.terminate()
    }
  }, [])

  // Immediate execution
  useEffect(() => {
    // Cancel any pending loading timer
    if (loadingTimerRef.current) {
      clearTimeout(loadingTimerRef.current)
      loadingTimerRef.current = null
    }

    if (!jsonpath.trim() || !json.trim()) {
      setResult({ status: 'idle' })
      setLoading(false)
      return
    }

    // Delay showing loading state
    loadingTimerRef.current = setTimeout(() => {
      setLoading(true)
    }, LOADING_DELAY_MS)

    requestIdRef.current += 1
    workerRef.current?.postMessage({
      id: requestIdRef.current,
      jsonpath,
      json,
    })

    return () => {
      if (loadingTimerRef.current) {
        clearTimeout(loadingTimerRef.current)
        loadingTimerRef.current = null
      }
    }
  }, [jsonpath, json])

  return { loading, result }
}
