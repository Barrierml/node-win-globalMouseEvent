import test from 'ava'

import { startListener } from '../index'

test('sync function from native code', (t) => {
  t.pass()
  startListener((data) => {
    console.log(data)
  })
  console.log('startListener')
})
