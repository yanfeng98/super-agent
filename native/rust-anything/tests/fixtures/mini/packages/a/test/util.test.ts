import { expect, it } from 'vitest'
import { rangeOf } from '../src/util'

it('finds ranges', () => {
  expect(rangeOf('hello world', 'world').start).toBe(6)
})
