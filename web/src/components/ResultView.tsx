import * as styles from '../styles/app.css'

interface ResultViewProps {
  status: 'loading' | 'success' | 'error' | 'idle'
  data?: string
  message?: string
}

export function ResultView({ status, data, message }: ResultViewProps) {
  return (
    <div className={styles.resultArea}>
      {status === 'loading' && (
        <span className={styles.loadingText}>Loading WASM...</span>
      )}
      {status === 'idle' && (
        <span className={styles.loadingText}>Enter a JSONPath query and JSON to see results</span>
      )}
      {status === 'error' && (
        <span className={styles.errorText}>{message}</span>
      )}
      {status === 'success' && data}
    </div>
  )
}
