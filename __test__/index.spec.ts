/* eslint-disable no-console */
import test from 'ava'

import { startListener } from '../index'

test('sync function from native code', (t) => {
  t.pass()
  startListener((data) => {
    console.log(JSON.parse(data))
  })
  setTimeout(() => {
    // @ts-expect-error
    process.exit(0)
  }, 10 * 1000)
})
