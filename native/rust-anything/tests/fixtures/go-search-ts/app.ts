import { score } from './search.ts'

const WORDS = ['alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot', 'golf', 'hotel', 'india', 'juliet']

function main() {
  const items: string[] = []
  for (let i = 0; i < 50_000; i++)
    items.push(`${WORDS[i % 10]} ${WORDS[(i * 7) % 10]} ${i}`)
  let hits = 0
  for (const query of ['al', 'chd', 'hot 1', 'xyz', 'ec ju', 'go 99']) {
    for (let round = 0; round < 40; round++) {
      for (const text of items) {
        if (score(query, text) > 0)
          hits++
      }
    }
  }
  console.log(hits, 'hits')
}

main()
