import { useState, useEffect, useCallback } from 'react'
import init, { query } from '../../wasm/jpp_wasm'

type JppResult =
  | { status: 'loading' }
  | { status: 'success'; data: string }
  | { status: 'error'; message: string }

export function useJpp() {
  const [ready, setReady] = useState(false)

  useEffect(() => {
    init().then(() => setReady(true))
  }, [])

  const execute = useCallback(
    (jsonpath: string, json: string): JppResult => {
      if (!ready) {
        return { status: 'loading' }
      }
      try {
        const result = query(jsonpath, json)
        return { status: 'success', data: result }
      } catch (e) {
        return { status: 'error', message: String(e) }
      }
    },
    [ready]
  )

  return { ready, execute }
}
