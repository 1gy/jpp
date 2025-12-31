import Editor from '@monaco-editor/react'
import * as styles from '../styles/app.css'
import type { JppResult } from '../hooks/useJpp'

interface ResultViewProps {
  result: JppResult
}

export function ResultView({ result }: ResultViewProps) {
  if (result.status === 'idle') {
    return (
      <div className={styles.resultArea}>
        <span className={styles.loadingText}>Enter a JSONPath query and JSON to see results</span>
      </div>
    )
  }

  if (result.status === 'error') {
    return (
      <div className={styles.resultArea}>
        <span className={styles.errorText}>{result.message}</span>
      </div>
    )
  }

  return (
    <div className={styles.editorWrapper}>
      <div className={styles.editorInner}>
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
    </div>
  )
}
