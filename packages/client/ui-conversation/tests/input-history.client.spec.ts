// @vitest-environment jsdom
/**
 * Folder-scoped composer input history: browser storage, keyboard recall at
 * the editor boundary, and recording of accepted submissions.
 */
import { randomUUID } from 'node:crypto'
import { Context } from '@deepseek-ai/cordis'
import { brandString } from '@deepseek-ai/dsh-brand'
import { createSnapshotStore } from '@deepseek-ai/dsh-client-store'
import { makeTranslate, TestSessions } from '@deepseek-ai/dsh-client-test-runtime'
import type { SessionId } from '@deepseek-ai/dsh-session/types'
import { $getRoot, $setSelection } from 'lexical'
import { beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest'
import type { CommandClaim, DraftAttachmentId, InputTriggerController, SubmitOutcome } from '../src/client/contract/input.ts'
import type { SessionInputDeps } from '../src/client/input/facade.ts'
import { SessionInputShell } from '../src/client/input/facade.ts'
import {
  appendInputHistory, atHistoryBoundary, INPUT_HISTORY_LIMIT, inputHistoryStorageKey, readInputHistory,
} from '../src/client/input/history.ts'
import type { ComposerHistorySource } from '../src/client/input/history.ts'
import { InputHub } from '../src/client/input/hub.ts'
import { zh } from '../src/client/locales.ts'

beforeEach(() => { localStorage.clear() })

/** A unique folder so tests never share a storage key. */
function folderOf(name: string): string {
  return `/w/${name}-${randomUUID()}`
}

describe('folder input history storage', () => {
  it('decodes only nonblank string entries', () => {
    const folder = folderOf('decode')
    expect(readInputHistory(folder)).toEqual([])
    localStorage.setItem(inputHistoryStorageKey(folder), '{')
    expect(readInputHistory(folder)).toEqual([])
    localStorage.setItem(inputHistoryStorageKey(folder), '42')
    expect(readInputHistory(folder)).toEqual([])
    localStorage.setItem(inputHistoryStorageKey(folder), JSON.stringify(['one', 7, '', '  ', 'two']))
    expect(readInputHistory(folder)).toEqual(['one', 'two'])
  })

  it('keeps only the newest entries and drops the oldest', () => {
    const folder = folderOf('limit')
    for (let index = 0; index <= INPUT_HISTORY_LIMIT; index += 1) appendInputHistory(folder, `prompt ${index}`)
    const entries = readInputHistory(folder)
    expect(entries).toHaveLength(INPUT_HISTORY_LIMIT)
    expect(entries[0]).toBe('prompt 1')
    expect(entries.at(-1)).toBe(`prompt ${INPUT_HISTORY_LIMIT}`)
  })

  it('isolates folders and skips blanks and the newest duplicate', () => {
    const first = folderOf('first')
    const second = folderOf('second')
    appendInputHistory(first, 'alpha')
    appendInputHistory(first, 'alpha')
    appendInputHistory(first, '')
    appendInputHistory(first, '   ')
    appendInputHistory(second, 'beta')
    expect(readInputHistory(first)).toEqual(['alpha'])
    expect(readInputHistory(second)).toEqual(['beta'])
    appendInputHistory(first, 'gamma')
    appendInputHistory(first, 'alpha')
    expect(readInputHistory(first)).toEqual(['alpha', 'gamma', 'alpha'])
  })

  it('leaves the stored entries untouched when writing fails', () => {
    const folder = folderOf('quota')
    appendInputHistory(folder, 'kept')
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementationOnce(() => {
      throw new Error('quota exceeded')
    })
    onTestFinished(() => { setItem.mockRestore() })
    expect(() => { appendInputHistory(folder, 'lost') }).not.toThrow()
    expect(readInputHistory(folder)).toEqual(['kept'])
  })

  it('treats an absent browser store as empty history', () => {
    vi.stubGlobal('localStorage', undefined)
    onTestFinished(() => { vi.unstubAllGlobals() })
    expect(readInputHistory(folderOf('absent'))).toEqual([])
    expect(() => { appendInputHistory(folderOf('absent'), 'none') }).not.toThrow()
  })
})

describe('atHistoryBoundary', () => {
  it('navigates both directions for an empty or single-line draft', () => {
    for (const text of ['', 'one line']) {
      expect(atHistoryBoundary(text, 0, 'older')).toBe(true)
      expect(atHistoryBoundary(text, text.length, 'older')).toBe(true)
      expect(atHistoryBoundary(text, 0, 'newer')).toBe(true)
      expect(atHistoryBoundary(text, text.length, 'newer')).toBe(true)
    }
  })

  it('leaves older from the first line and newer from the last line', () => {
    const text = 'first\nmiddle\nlast'
    expect(atHistoryBoundary(text, 0, 'older')).toBe(true)
    expect(atHistoryBoundary(text, 5, 'older')).toBe(true)
    expect(atHistoryBoundary(text, 6, 'older')).toBe(false)
    expect(atHistoryBoundary(text, 12, 'newer')).toBe(false)
    expect(atHistoryBoundary(text, 13, 'newer')).toBe(true)
    expect(atHistoryBoundary(text, text.length, 'newer')).toBe(true)
  })
})

interface HistoryBenchOptions {
  readonly folder?: string | undefined
  readonly seed?: readonly string[]
  readonly sink?: SessionInputDeps['defaultSink']
  readonly adjudicate?: InputTriggerController['adjudicate']
  readonly fail?: 'folder' | 'record'
}

interface HistoryBench {
  readonly shell: SessionInputShell
  readonly recorded: { readonly folder: string; readonly text: string }[]
  readonly triggerTrack: ReturnType<typeof vi.fn>
}

/** A real input shell over an in-memory folder history. */
function historyBench(options: HistoryBenchOptions = {}): HistoryBench {
  const ctx = new Context()
  onTestFinished(() => { void ctx.fiber.dispose() })
  const folder = 'folder' in options ? options.folder : '/w/a'
  const store = new Map<string, string[]>()
  if (folder !== undefined) store.set(folder, [...(options.seed ?? [])])
  const recorded: { folder: string; text: string }[] = []
  const history: ComposerHistorySource = {
    folder: options.fail === 'folder' ? () => { throw new Error('history scope gone') } : () => folder,
    entries: candidate => store.get(candidate) ?? [],
    record: options.fail === 'record'
      ? () => { throw new Error('history store gone') }
      : (candidate, text) => {
        recorded.push({ folder: candidate, text })
        store.set(candidate, [...(store.get(candidate) ?? []), text])
      },
  }
  const triggerTrack = vi.fn()
  const triggers: InputTriggerController = {
    lexicon: createSnapshotStore<ReadonlyMap<'/' | '@', readonly string[]>>(new Map()),
    launcher: createSnapshotStore<string | null>(null),
    track: triggerTrack,
    arbitrate: () => 'pass',
    onSpace: () => false,
    serializeReference: (_source, ref) => Promise.resolve(ref),
    adjudicate: options.adjudicate ?? (() => Promise.resolve(undefined)),
    resolveClaim: () => Promise.resolve(undefined),
    openReference: () => false,
    toggleSource: () => {},
  }
  const shell = new SessionInputShell({
    actx: ctx,
    inputTriggers: () => triggers,
    history,
    defaultSink: options.sink ?? (() => Promise.resolve<SubmitOutcome>({ kind: 'success' })),
    commandAttachments: {
      serialize: () => Promise.resolve([]),
      release: () => {},
      unsupportedNotice: token => token,
    },
  })
  onTestFinished(() => { shell.dispose() })
  shell.refreshLexiconSubscription()
  return { shell, recorded, triggerTrack }
}

describe('composer history recall', () => {
  it('walks older and newer entries from an empty draft and clears past the newest', () => {
    const { shell } = historyBench({ seed: ['one', 'two'] })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('two')
    expect(shell.caretSpan()).toEqual({ start: 'two'.length, end: 'two'.length })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('one')
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('one')
    expect(shell.historyStep('newer')).toBe(true)
    expect(shell.snapshot.draft).toBe('two')
    expect(shell.historyStep('newer')).toBe(true)
    expect(shell.snapshot.draft).toBe('')
    expect(shell.historyStep('newer')).toBe(false)
  })

  it('does not start from a non-empty draft, attachments, a missing folder, or empty history', () => {
    const draftBench = historyBench({ seed: ['one'] })
    draftBench.shell.setDraft('typing')
    expect(draftBench.shell.historyStep('older')).toBe(false)
    expect(draftBench.shell.snapshot.draft).toBe('typing')

    const attachmentBench = historyBench({ seed: ['one'] })
    attachmentBench.shell.addAttachments(['file-1' as DraftAttachmentId])
    expect(attachmentBench.shell.historyStep('older')).toBe(false)
    expect(attachmentBench.shell.snapshot.draft).toBe('')

    const noFolder = historyBench({ folder: undefined, seed: ['one'] })
    expect(noFolder.shell.historyStep('older')).toBe(false)

    const noEntries = historyBench({ seed: [] })
    expect(noEntries.shell.historyStep('older')).toBe(false)
    expect(noEntries.shell.historyStep('newer')).toBe(false)
  })

  it('refuses history while the submit plane is frozen', () => {
    const pending = new Promise<SubmitOutcome>(() => {})
    const { shell } = historyBench({ seed: ['one'], sink: () => pending })
    shell.setDraft('sent')
    shell.submit()
    expect(shell.snapshot.phase).toBe('sending')
    expect(shell.historyStep('older')).toBe(false)
  })

  it('keeps arrows inside a recalled multi-line entry until its first line', () => {
    const { shell } = historyBench({ seed: ['older', 'line one\nline two'] })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('line one\nline two')
    expect(shell.historyStep('older')).toBe(false)
    expect(shell.snapshot.draft).toBe('line one\nline two')
    shell.editor.update(() => { $getRoot().selectStart() }, { discrete: true })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('older')
  })

  it('navigates a collapsed caret but leaves ranged selections and absent selections at the end', () => {
    const { shell } = historyBench({ seed: ['one', 'two'] })
    expect(shell.historyStep('older')).toBe(true)
    shell.editor.update(() => { $getRoot().select(0, 2) }, { discrete: true })
    expect(shell.historyStep('older')).toBe(false)
    expect(shell.snapshot.draft).toBe('two')
    shell.editor.update(() => { $setSelection(null) }, { discrete: true })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('one')
  })

  it('ends browsing when the draft is edited', () => {
    const { shell } = historyBench({ seed: ['one', 'two'] })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.historyStep('older')).toBe(true)
    shell.paste('!')
    expect(shell.snapshot.draft).toBe('one!')
    expect(shell.historyStep('newer')).toBe(false)
    expect(shell.historyStep('older')).toBe(false)
    expect(shell.snapshot.draft).toBe('one!')
  })

  it('does not reopen the trigger menu while a recalled command stays unchanged', () => {
    const { shell, triggerTrack } = historyBench({ seed: ['/compact'] })
    triggerTrack.mockClear()
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('/compact')
    // The recall's committed update can arrive after the keydown returns.
    shell.editor.update(() => { $getRoot().selectStart() }, { discrete: true })
    expect(triggerTrack).not.toHaveBeenCalled()
    shell.setDraft('/compact edited')
    expect(triggerTrack).toHaveBeenCalled()
  })

  it.each([
    'trailing newline\n',
    '\nleading newline',
    'two\n\nblank lines',
    'crlf\r\nline',
  ])('recalls and browsed past a recalled entry %j without losing history', (entry) => {
    const { shell } = historyBench({ seed: [entry] })
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe(entry)
    expect(shell.historyStep('older')).toBe(false)
    expect(shell.historyStep('newer')).toBe(true)
    expect(shell.snapshot.draft).toBe('')
    expect(shell.historyStep('newer')).toBe(false)
  })
})

describe('composer history recording', () => {
  it('records an accepted message and keeps a refused one out of history', async () => {
    const accepted = historyBench()
    accepted.shell.setDraft('hello')
    accepted.shell.submit()
    await vi.waitFor(() => { expect(accepted.recorded).toEqual([{ folder: '/w/a', text: 'hello' }]) })

    const refused = historyBench({ sink: () => Promise.resolve<SubmitOutcome>({ kind: 'error' }) })
    refused.shell.setDraft('kept')
    refused.shell.submit()
    await vi.waitFor(() => { expect(refused.shell.snapshot.phase).toBe('plain') })
    expect(refused.recorded).toEqual([])
    expect(refused.shell.snapshot.draft).toBe('kept')
  })

  it('keeps recall and the accepted commit working when the history scope fails', async () => {
    const brokenRecall = historyBench({ seed: ['one'], fail: 'folder' })
    expect(brokenRecall.shell.historyStep('older')).toBe(false)
    brokenRecall.shell.setDraft('hello')
    brokenRecall.shell.submit()
    await vi.waitFor(() => { expect(brokenRecall.shell.snapshot.phase).toBe('plain') })
    expect(brokenRecall.shell.snapshot.draft).toBe('')

    const brokenRecord = historyBench({ seed: ['one'], fail: 'record' })
    expect(brokenRecord.shell.historyStep('older')).toBe(true)
    brokenRecord.shell.setDraft('hello')
    brokenRecord.shell.submit()
    await vi.waitFor(() => { expect(brokenRecord.shell.snapshot.phase).toBe('plain') })
    expect(brokenRecord.shell.snapshot.draft).toBe('')
  })

  it('records an accepted command submission', async () => {
    const claim: CommandClaim = {
      name: 'goal',
      token: '/goal ',
      submit: () => Promise.resolve<SubmitOutcome>({ kind: 'success' }),
    }
    const { shell, recorded } = historyBench({ adjudicate: () => Promise.resolve({ claim }) })
    shell.setDraft('/goal ship it')
    shell.submit()
    await vi.waitFor(() => { expect(recorded).toEqual([{ folder: '/w/a', text: '/goal ship it' }]) })
  })
})

describe('hub history scope', () => {
  it('scopes recall by the Session working directory', async () => {
    const id = brandString<SessionId>(`history-hub-${randomUUID()}`)
    const folder = folderOf('hub')
    const ctx = new Context()
    const sessions = new TestSessions(async (operation) => { await operation() }, ctx)
    onTestFinished(async () => {
      await sessions.disposeScopes()
      await ctx.fiber.dispose()
    })
    ctx.provide('sessions', sessions)
    await sessions.add({ id, summary: { cwd: folder } })
    const reference = sessions.retain(id)
    onTestFinished(() => { reference.release() })
    await reference.ready
    appendInputHistory(folder, 'from the folder')
    ctx.provide('conversation', {
      blocks: { storeFor: () => ({ getSnapshot: () => undefined }) },
      sendSession: async (
        _session: object, _text: string, _ids: readonly unknown[], _mode: string, _signal: AbortSignal, accept?: () => void,
      ) => {
        accept?.()
        return { kind: 'success' }
      },
      serializeDraftAttachments: async () => ({ attachments: [] }),
      releaseDraftAttachment: () => {},
    })
    const hub = new InputHub(ctx, makeTranslate(zh, {}))
    const shell = hub.shellFor(reference.binding)
    expect(shell.historyStep('older')).toBe(true)
    expect(shell.snapshot.draft).toBe('from the folder')
    shell.setDraft('record me')
    shell.submit()
    await vi.waitFor(() => { expect(shell.snapshot.phase).toBe('plain') })
    expect(readInputHistory(folder)).toEqual(['from the folder', 'record me'])
  })
})
