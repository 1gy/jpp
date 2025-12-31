import Editor from '@monaco-editor/react'

interface JsonInputProps {
  value: string
  onChange: (value: string) => void
}

export function JsonInput({ value, onChange }: JsonInputProps) {
  return (
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
  )
}
