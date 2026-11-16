// @vitest-environment jsdom
/**
 * The settings page: each preference writes its own callback, the clip library
 * distinguishes built-ins from user files and keeps preview separate from
 * selection, and the conversation section degrades through its three states.
 */
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { SettingsSection } from '../src/client/SettingsSection.tsx'
import { DEFAULT_SETTINGS } from '../src/client/contract.ts'
import type {
  CatalogState, IntroSettings, StartupSessionBinding, SettingsSectionProps,
} from '../src/client/contract.ts'
import type { ClipInfo } from '../src/types.ts'

const CLIPS: readonly ClipInfo[] = [
  { id: 'builtin:brand', name: null, kind: 'builtin', bytes: 10, version: 'v1' },
  { id: 'file:mine', name: 'My clip', kind: 'file', bytes: 20, version: 'v1' },
  { id: 'file:anon', name: null, kind: 'file', bytes: 30, version: 'v1' },
  { id: 'builtin:mystery', name: null, kind: 'builtin', bytes: 40, version: 'v1' },
]

const SETTINGS: IntroSettings = {
  ...DEFAULT_SETTINGS,
}

const CATALOG: CatalogState = {
  clips: CLIPS,
  session: { replay: false, clipId: null },
  loading: false,
  error: null,
  overridesAvailable: true,
}

const SESSION: StartupSessionBinding = { sessionId: 's1', blank: true }

interface Overrides {
  readonly catalog?: Partial<CatalogState>
  readonly settings?: Partial<IntroSettings>
  readonly session?: Partial<StartupSessionBinding>
  readonly sessionStorageAvailable?: boolean
}

/** Props over one scripted catalog/settings/session triple, plus their spies. */
function makeProps(overrides: Overrides = {}) {
  const catalog: CatalogState = { ...CATALOG, ...overrides.catalog }
  const settings: IntroSettings = { ...SETTINGS, ...overrides.settings }
  const session: StartupSessionBinding = { ...SESSION, ...overrides.session }
  const spies = {
    refresh: vi.fn(),
    preview: vi.fn(),
    setEnabled: vi.fn(),
    setTiming: vi.fn(),
    setFit: vi.fn(),
    setRandom: vi.fn(),
    setSelectedClip: vi.fn(),
    previewArchive: vi.fn(),
    setSequence: vi.fn(),
    setScreen: vi.fn(),
    setSessionReplay: vi.fn(),
    setSessionClip: vi.fn(),
  }
  const props = {
    useCatalog: <T,>(selector: (value: CatalogState) => T): T => selector(catalog),
    useSettings: <T,>(selector: (value: IntroSettings) => T): T => selector(settings),
    useSession: <T,>(selector: (value: StartupSessionBinding) => T): T => selector(session),
    ...spies,
    sessionStorageAvailable: overrides.sessionStorageAvailable ?? true,
    t: (key: string) => key,
  } as SettingsSectionProps
  return { props, ...spies }
}

function enabledSwitch(): HTMLInputElement {
  return screen.getByRole('checkbox', { name: /enabled\.label/ })
}

function randomSwitch(): HTMLInputElement {
  return screen.getByRole('checkbox', { name: 'random.label' })
}

function replaySwitch(): HTMLInputElement {
  return screen.getByRole('checkbox', { name: 'session.replay' })
}

function radio(name: string): HTMLInputElement {
  const element = screen.getByRole('radio', { name })
  if (!(element instanceof HTMLInputElement)) throw new Error(`expected a radio input for ${name}`)
  return element
}

function clipSelect(): HTMLSelectElement {
  const element = screen.getByRole('combobox', { name: 'session.clip' })
  if (!(element instanceof HTMLSelectElement)) throw new Error('expected the conversation clip select')
  return element
}

afterEach(() => { cleanup() })

describe('SettingsSection preferences', () => {
  it('shows the current choices and writes each one through its callback', () => {
    const h = makeProps({ settings: { timing: 'entry', fit: 'contain', random: true } })
    const view = render(<SettingsSection {...h.props} />)

    expect(enabledSwitch().checked).toBe(true)
    expect(radio('timing.entry').checked).toBe(true)
    expect(radio('fit.contain').checked).toBe(true)
    expect(randomSwitch().checked).toBe(true)

    fireEvent.click(enabledSwitch())
    fireEvent.click(screen.getByRole('radio', { name: 'timing.appStart' }))
    fireEvent.click(screen.getByRole('radio', { name: 'fit.cover' }))
    fireEvent.click(randomSwitch())

    expect(h.setEnabled).toHaveBeenCalledExactlyOnceWith(false)
    expect(h.setTiming).toHaveBeenCalledExactlyOnceWith('app-start')
    expect(h.setFit).toHaveBeenCalledExactlyOnceWith('cover')
    expect(h.setRandom).toHaveBeenCalledExactlyOnceWith(false)

    // The opposite choices come from a fresh render, as a real settings reload would.
    view.unmount()
    const flipped = makeProps()
    render(<SettingsSection {...flipped.props} />)
    fireEvent.click(screen.getByRole('radio', { name: 'timing.entry' }))
    fireEvent.click(screen.getByRole('radio', { name: 'fit.contain' }))
    expect(flipped.setTiming).toHaveBeenCalledExactlyOnceWith('entry')
    expect(flipped.setFit).toHaveBeenCalledExactlyOnceWith('contain')
  })
})

describe('SettingsSection clip library', () => {
  it('names built-in clips through copy and user clips through their own name or id', () => {
    const h = makeProps()
    render(<SettingsSection {...h.props} />)

    const library = screen.getByRole('list')
    expect(within(library).getByText('clips.builtin.brand')).toBeTruthy()
    expect(within(library).getByText('My clip')).toBeTruthy()
    expect(within(library).getByText('file:anon')).toBeTruthy()
    expect(within(library).getByText('builtin:mystery')).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'clips.label' })).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'clips.refresh' }))
    expect(h.refresh).toHaveBeenCalledOnce()
  })

  it('narrows to the empty prompt when the library has no clips', () => {
    render(<SettingsSection {...makeProps({ catalog: { clips: [] } }).props} />)
    expect(screen.getByText('clips.empty')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'action.preview' })).toBeNull()
  })

  it('previews without selecting, and only selects through an enabled action', () => {
    const h = makeProps()
    render(<SettingsSection {...h.props} />)

    const previews = screen.getAllByRole('button', { name: 'action.preview' })
    for (const button of previews) fireEvent.click(button)
    expect(h.preview.mock.calls).toEqual([['builtin:brand'], ['file:mine'], ['file:anon'], ['builtin:mystery']])

    const selects = screen.getAllByRole('button', { name: 'action.select' })
    expect(selects.map(button => (button as HTMLButtonElement).disabled)).toEqual([true, false, false, false])
    expect(screen.getByText('action.selected')).toBeTruthy()

    fireEvent.click(selects[1]!)
    expect(h.setSelectedClip).toHaveBeenCalledExactlyOnceWith('file:mine')
    fireEvent.click(selects[0]!)
    expect(h.setSelectedClip).toHaveBeenCalledOnce()
  })
})

describe('SettingsSection conversation', () => {
  it('reports when host storage is unavailable', () => {
    render(<SettingsSection {...makeProps({ sessionStorageAvailable: false }).props} />)
    expect(screen.getByText('session.unavailable')).toBeTruthy()
    expect(screen.queryByRole('checkbox', { name: 'session.replay' })).toBeNull()
  })

  it('asks the user to open a conversation when none is current', () => {
    render(<SettingsSection {...makeProps({ session: { sessionId: null } }).props} />)
    expect(screen.getByText('session.noSession')).toBeTruthy()
    expect(screen.queryByRole('checkbox', { name: 'session.replay' })).toBeNull()
  })

  it('follows the global clip until the user picks a conversation clip', () => {
    const h = makeProps()
    render(<SettingsSection {...h.props} />)

    expect(replaySwitch().checked).toBe(false)
    const select = clipSelect()
    expect(select.value).toBe('')
    expect(screen.getByRole('option', { name: 'session.followGlobal' })).toBeTruthy()
    expect(screen.getByRole('option', { name: 'My clip' })).toBeTruthy()

    fireEvent.change(select, { target: { value: 'file:anon' } })
    expect(h.setSessionClip).toHaveBeenNthCalledWith(1, 'file:anon')
    fireEvent.change(select, { target: { value: '' } })
    expect(h.setSessionClip).toHaveBeenNthCalledWith(2, null)
  })

  it('shows the stored replay flag and clip for this conversation', () => {
    const h = makeProps({ catalog: { session: { replay: true, clipId: 'file:mine' } } })
    render(<SettingsSection {...h.props} />)

    expect(replaySwitch().checked).toBe(true)
    expect(clipSelect().value).toBe('file:mine')
    fireEvent.click(replaySwitch())
    expect(h.setSessionReplay).toHaveBeenCalledExactlyOnceWith(false)
  })
})

describe('sequence and archive settings', () => {
  it('offers the four sequence choices and writes the picked one', () => {
    const { props, setSequence } = makeProps()
    render(<SettingsSection {...props} />)
    const select = screen.getByRole('combobox', { name: /sequence\.label/ })
    expect(within(select).getAllByRole('option')).toHaveLength(4)
    fireEvent.change(select, { target: { value: 'archive-only' } })
    expect(setSequence).toHaveBeenCalledWith('archive-only')
  })

  it('falls back to the default when an unknown sequence value arrives', () => {
    const { props, setSequence } = makeProps()
    render(<SettingsSection {...props} />)
    fireEvent.change(screen.getByRole('combobox', { name: /sequence\.label/ }), { target: { value: 'nonsense' } })
    expect(setSequence).toHaveBeenCalledWith('archive-then-video')
  })

  it('previews the archive sequence from the sequence header', () => {
    const { props, previewArchive } = makeProps()
    render(<SettingsSection {...props} />)
    fireEvent.click(screen.getByRole('button', { name: 'action.previewSequence' }))
    expect(previewArchive).toHaveBeenCalledOnce()
  })

  it('writes a toggle, a select, a range, a colour, and a text field', () => {
    const { props, setScreen } = makeProps()
    render(<SettingsSection {...props} />)

    fireEvent.click(screen.getByRole('checkbox', { name: /screen\.allowSkip\.label/ }))
    expect(setScreen).toHaveBeenCalledWith('screenAllowSkip', false)

    fireEvent.change(screen.getByRole('combobox', { name: /screen\.theme\.label/ }), { target: { value: 'dark' } })
    expect(setScreen).toHaveBeenCalledWith('screenTheme', 'dark')

    fireEvent.change(screen.getByRole('slider', { name: /screen\.speed\.label/ }), { target: { value: '2.5' } })
    expect(setScreen).toHaveBeenCalledWith('screenSpeed', 2.5)

    fireEvent.change(screen.getByLabelText(/screen\.identity\.label/), { target: { value: 'ADA' } })
    expect(setScreen).toHaveBeenCalledWith('screenIdentity', 'ADA')

    fireEvent.change(screen.getByLabelText(/screen\.accent\.label/), { target: { value: '#123456' } })
    expect(setScreen).toHaveBeenCalledWith('screenAccent', '#123456')
  })

  it('bounds a range by the field it writes', () => {
    const { props } = makeProps()
    render(<SettingsSection {...props} />)
    const speed = screen.getByRole('slider', { name: /screen\.speed\.label/ })
    expect(speed.getAttribute('min')).toBe('0.35')
    expect(speed.getAttribute('max')).toBe('3')
  })
})
