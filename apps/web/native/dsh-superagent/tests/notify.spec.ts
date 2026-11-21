import { describe, expect, it } from 'vitest'
import { en } from '../src/client/locales.ts'
import { eventNotificationFor, notificationFor } from '../src/client/notify.ts'

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
  it('announces a merge event from the inbox, which a Ticket\'s state does not show', () => {
    expect(eventNotificationFor({ id: 'n1', kind: 'merged', title: '修复回归', text: 'Merged T-0001 into main' }))
      .toEqual({ title: en['notification.merged'], body: '修复回归 · Merged T-0001 into main' })
    expect(eventNotificationFor({ id: 'n2', kind: 'check', title: '修复回归', text: 'The configured check failed' }))
      .toMatchObject({ title: en['notification.check'] })
    // A question or an ending is announced from the ticket's own state, not twice.
    expect(eventNotificationFor({ id: 'n3', kind: 'attention', title: 'x', text: 'y' })).toBeUndefined()
    expect(eventNotificationFor({ id: 'n4', kind: 'ended', title: 'x', text: 'y' })).toBeUndefined()
  })
  it('announces the end of a ticket, and its reopening as waiting again', () => {
    expect(notificationFor('person:review', ticket({ state: 'ended', stage: 'ended' }))).toEqual({ title: en.notifyEnded, body: '修复回归' })
    expect(notificationFor('ended', ticket({ stage: 'person', action: 'review' }))).toMatchObject({ title: en.notifyAttention })
    // Progress and idle work need no interruption.
    expect(notificationFor('working:', ticket({ stage: 'idle' }))).toBeUndefined()
    expect(notificationFor('working:queued', ticket({ stage: 'working' }))).toBeUndefined()
  })
})
