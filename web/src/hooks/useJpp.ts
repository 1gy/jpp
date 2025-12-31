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

export function useJpp(jsonpath: string, json: string): JppResult {
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<JppData>({ status: 'idle' })
  const workerRef = useRef<Worker | null>(null)
  const requestIdRef = useRef(0)

  // Initialize worker
  useEffect(() => {
    workerRef.current = new JppWorker()

    workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const { id, ...response } = e.data
      // Only accept the latest request
      if (id === requestIdRef.current) {
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
    if (!jsonpath.trim() || !json.trim()) {
      setResult({ status: 'idle' })
      setLoading(false)
      return
    }

    setLoading(true)
    requestIdRef.current += 1
    workerRef.current?.postMessage({
      id: requestIdRef.current,
      jsonpath,
      json,
    })
  }, [jsonpath, json])

  return { loading, result }
}
