import * as styles from '../styles/app.css'

export function Header() {
  return (
    <header className={styles.header}>
      <h1 className={styles.title}>jpp</h1>
      <span>JSONPath Processor</span>
    </header>
  )
}
