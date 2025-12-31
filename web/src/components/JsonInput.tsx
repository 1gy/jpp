import * as styles from '../styles/app.css'

interface JsonInputProps {
  value: string
  onChange: (value: string) => void
}

export function JsonInput({ value, onChange }: JsonInputProps) {
  return (
    <textarea
      className={styles.textarea}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder='{"store": {"book": [...]}}'
      spellCheck={false}
    />
  )
}
