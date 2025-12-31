import Editor from '@monaco-editor/react'
import * as styles from '../styles/app.css'

interface JsonInputProps {
  value: string
  onChange: (value: string) => void
}

export function JsonInput({ value, onChange }: JsonInputProps) {
  return (
    <div className={styles.editorWrapper}>
      <div className={styles.editorInner}>
        <Editor
          language="json"
          theme="light"
          value={value}
          onChange={(v) => onChange(v ?? '')}
          options={{
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
