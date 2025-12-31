import { useState } from 'react'
import { useJpp } from './hooks/useJpp'
import { Header } from './components/Header'
import { QueryInput } from './components/QueryInput'
import { JsonInput } from './components/JsonInput'
import { ResultView } from './components/ResultView'
import * as styles from './styles/app.css'

const DEFAULT_JSON = `{
  "store": {
    "book": [
      { "category": "reference", "author": "Nigel Rees", "title": "Sayings of the Century", "price": 8.95 },
      { "category": "fiction", "author": "Evelyn Waugh", "title": "Sword of Honour", "price": 12.99 }
    ]
  }
}`

function App() {
  const [query, setQuery] = useState('$.store.book[*].author')
  const [json, setJson] = useState(DEFAULT_JSON)
  const result = useJpp(query, json)

  return (
    <div className={styles.container}>
      <Header />
      <main className={styles.main}>
        <div className={styles.leftPane}>
          <div className={styles.paneHeader}>Input</div>
          <div className={styles.paneContent}>
            <QueryInput value={query} onChange={setQuery} />
            <JsonInput value={json} onChange={setJson} />
          </div>
        </div>
        <div className={styles.rightPane}>
          <div className={styles.paneHeader}>Result</div>
          <div className={styles.paneContent}>
            <ResultView
              status={result.status}
              data={result.status === 'success' ? result.data : undefined}
              message={result.status === 'error' ? result.message : undefined}
            />
          </div>
        </div>
      </main>
    </div>
  )
}

export default App
