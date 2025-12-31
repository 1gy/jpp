import * as styles from '../styles/app.css'

interface QueryInputProps {
  value: string
  onChange: (value: string) => void
}

export function QueryInput({ value, onChange }: QueryInputProps) {
  return (
    <input
      type="text"
      className={styles.queryInput}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder="$.store.book[*].author"
      spellCheck={false}
    />
  )
}
