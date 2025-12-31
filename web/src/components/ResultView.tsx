import Editor from '@monaco-editor/react'
import * as styles from '../styles/app.css'

interface ResultViewProps {
  status: 'loading' | 'success' | 'error' | 'idle'
  data?: string
  message?: string
}

export function ResultView({ status, data, message }: ResultViewProps) {
  if (status === 'loading') {
    return (
      <div className={styles.resultArea}>
        <span className={styles.loadingText}>Loading...</span>
      </div>
    )
  }

  if (status === 'idle') {
    return (
      <div className={styles.resultArea}>
        <span className={styles.loadingText}>Enter a JSONPath query and JSON to see results</span>
      </div>
    )
  }

  if (status === 'error') {
    return (
      <div className={styles.resultArea}>
        <span className={styles.errorText}>{message}</span>
      </div>
    )
  }

  return (
    <Editor
      language="json"
      theme="light"
      value={data}
      options={{
        readOnly: true,
        minimap: { enabled: false },
        fontSize: 14,
        fontFamily: "'Fira Code', 'Consolas', 'Monaco', monospace",
        lineNumbers: 'on',
        scrollBeyondLastLine: false,
        automaticLayout: true,
        tabSize: 2,
      }}
    />
  )
}
