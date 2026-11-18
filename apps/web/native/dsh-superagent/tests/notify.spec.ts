import { describe, expect, it } from 'vitest'
import { en } from '../src/client/locales.ts'
import { notificationFor } from '../src/client/notify.ts'

type Watched = Parameters<typeof notificationFor>[1]
const ticket = (fields: Partial<Watched>): Watched => ({ id: 't1', title: '修复回归', state: 'open', stage: 'working', ...fields } as Watched)

describe('desktop notifications', () => {
  it('announces a ticket that starts waiting on a person, with the stage it waits in', () => {
    expect(notificationFor('working:', ticket({ stage: 'person', action: 'review' }))).toEqual({ title: en.notifyAttention,
      body: `修复回归 · ${en['stage.review']}` })
    expect(notificationFor('working:', ticket({ stage: 'person', action: 'answer' }))).toMatchObject({ body: expect.stringContaining(en['stage.answer']) })
    // A ticket that keeps waiting the same way is announced once.
    expect(notificationFor('person:review', ticket({ stage: 'person', action: 'review' }))).toBeUndefined()
    expect(notificationFor(undefined, ticket({ stage: 'person', action: 'review' }))).toBeUndefined()
  })
  it('announces the end of a ticket, and its reopening as waiting again', () => {
    expect(notificationFor('person:review', ticket({ state: 'ended', stage: 'ended' }))).toEqual({ title: en.notifyEnded, body: '修复回归' })
    expect(notificationFor('ended', ticket({ stage: 'person', action: 'review' }))).toMatchObject({ title: en.notifyAttention })
    // Progress and idle work need no interruption.
    expect(notificationFor('working:', ticket({ stage: 'idle' }))).toBeUndefined()
    expect(notificationFor('working:queued', ticket({ stage: 'working' }))).toBeUndefined()
  })
})
