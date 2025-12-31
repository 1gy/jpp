import Editor from '@monaco-editor/react'
import * as styles from '../styles/app.css'
import type { JppData } from '../hooks/useJpp'

interface ResultViewProps {
  loading: boolean
  result: JppData
}

export function ResultView({ loading, result }: ResultViewProps) {
  const opacity = loading ? 0.5 : 1
  const transition = 'opacity 0.15s ease-in-out'

  if (result.status === 'idle') {
    return (
      <div className={styles.resultArea} style={{ opacity, transition }}>
        <span className={styles.loadingText}>Enter a JSONPath query and JSON to see results</span>
      </div>
    )
  }

  if (result.status === 'error') {
    return (
      <div className={styles.resultArea} style={{ opacity, transition }}>
        <span className={styles.errorText}>{result.message}</span>
      </div>
    )
  }

  return (
    <div style={{ flex: 1, opacity, transition }}>
      <Editor
        language="json"
        theme="light"
        value={result.data}
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
    </div>
  )
}
