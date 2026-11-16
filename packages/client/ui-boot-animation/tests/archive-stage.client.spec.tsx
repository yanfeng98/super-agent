// @vitest-environment jsdom
/**
 * The archive sequence: the six acts, the two confirmation gates, skip, and the
 * teardown path. Timers are faked, so each act is reached by advancing the clock
 * rather than waiting.
 */
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ArchiveStage, INITIAL_FRAME } from '../src/client/archive/ArchiveStage.tsx'
import type { ArchiveLabels } from '../src/client/archive/ArchiveStage.tsx'
import { DEFAULT_SETTINGS } from '../src/client/contract.ts'
import type { IntroSettings } from '../src/client/contract.ts'
import type { AudioFactory } from '../src/client/archive/audio.ts'

const LABELS: ArchiveLabels = {
  skip: 'skip',
  hint: 'hint',
  cardName: 'name',
  cardId: 'id',
  cardLevel: 'level',
  welcome: name => `welcome ${name}`,
  idLine: id => `id ${id}`,
  selfCheck: { zh: 'self', en: 'SELF' },
  finale: { zh: 'finale', en: 'FINALE' },
  revision: 'rev',
  logoMark: 'MARK',
  welcomeTo: 'WELCOME TO',
  welcomeMark: 'WELCOME MARK',
}

/** An audio factory whose recorded takes end as soon as they start. */
function silentFactory(): AudioFactory {
  return {
    element: () => {
      const listeners = new Set<() => void>()
      return {
        play: async () => {
          for (const listener of [...listeners]) listener()
        },
        pause: () => undefined,
        volume: 0,
        currentTime: 0,
        duration: 1,
        addEventListener: (_type: 'ended' | 'error', listener: () => void) => { listeners.add(listener) },
        removeEventListener: (_type: 'ended' | 'error', listener: () => void) => { listeners.delete(listener) },
      }
    },
    speech: () => null,
    voices: () => [],
  }
}

/** Render the stage with the defaults and a given settings override. */
function mount(
  override: Partial<IntroSettings> = {},
  onFinished = vi.fn(),
  factory: AudioFactory | null = silentFactory(),
) {
  const settings: IntroSettings = { ...DEFAULT_SETTINGS, ...override }
  const view = render(
    <ArchiveStage settings={settings} labels={LABELS} onFinished={onFinished} audioFactory={factory} />,
  )
  return { ...view, onFinished }
}

/** Advance the fake clock, running the promise chain the sequence is built from. */
async function runFor(totalMs: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(totalMs)
  })
}

/** Advance in small slices so each intermediate frame is committed. */
async function runSlowly(totalMs: number, slice = 120): Promise<void> {
  for (let elapsed = 0; elapsed < totalMs; elapsed += slice) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(slice)
    })
  }
}

beforeEach(() => { vi.useFakeTimers() })
afterEach(() => {
  cleanup()
  vi.useRealTimers()
})

describe('ArchiveStage', () => {
  it('starts on the black prologue with nothing else visible', async () => {
    const { container } = mount()
    expect(container.querySelector('.dsh-boot-archive-root')).toBeTruthy()
    expect(container.querySelector('.dsh-boot-archive-black-inner')?.className)
      .not.toContain('dsh-boot-archive-in')
    expect(container.querySelector('.dsh-boot-archive-brand')?.className).not.toContain('dsh-boot-archive-in')
  })

  it('draws the wordmark, the sweep, the wash, and the plinth as time passes', async () => {
    const { container } = mount()
    await runFor(850)
    expect(container.querySelector('.dsh-boot-archive-black-inner')?.className).toContain('dsh-boot-archive-in')
    await runFor(300)
    expect(container.querySelector('.dsh-boot-archive-black-en')?.className).toContain('dsh-boot-archive-tighten')
    await runFor(900)
    expect(container.querySelector('.dsh-boot-archive-sweep')?.className).toContain('dsh-boot-archive-go')
    await runFor(800)
    expect(container.querySelector<HTMLElement>('.dsh-boot-archive-wash')?.style.opacity).toBe('1')
    await runFor(300)
    expect(container.querySelector<HTMLElement>('.dsh-boot-archive-plate')?.style.opacity).toBe('1')
    await runFor(800)
    expect(container.querySelector('.dsh-boot-archive-brand')?.className).toContain('dsh-boot-archive-in')
  })

  it('types the first authorization line and then waits for the user', async () => {
    const { container, onFinished } = mount()
    await runFor(6_000)
    expect(container.textContent).toContain('需要访问许可')
    expect(container.querySelector('.dsh-boot-archive-action')?.className).toContain('dsh-boot-archive-in')
    expect(onFinished).not.toHaveBeenCalled()
  })

  it('finishes when the last act hands off', async () => {
    const { onFinished } = mount({ screenRequireInteraction: false }, vi.fn())
    await runFor(60_000)
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('finishes on Escape and stops both timers and audio', async () => {
    const { onFinished } = mount()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('ignores Escape when skipping is turned off', async () => {
    const { onFinished } = mount({ screenAllowSkip: false })
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onFinished).not.toHaveBeenCalled()
  })

  it('leaves nothing scheduled after unmount', async () => {
    const onFinished = vi.fn()
    const { unmount } = mount({}, onFinished)
    unmount()
    await runFor(120_000)
    expect(onFinished).not.toHaveBeenCalled()
  })

  it('shows the identity card on the confirmation line and continues on click', async () => {
    const { container } = mount()
    await runFor(6_000)
    expect(container.querySelector('.dsh-boot-archive-card')?.hasAttribute('hidden')).toBe(true)
    fireEvent.click(screen.getByRole('button'))
    await runFor(3_000)
    expect(container.textContent).toContain('身份资料确认')
    expect(container.querySelector('.dsh-boot-archive-card')?.hasAttribute('hidden')).toBe(false)
    expect(container.textContent).toContain('0087')
  })

  it('continues on Enter and on Space', async () => {
    const { container } = mount()
    await runFor(6_000)
    fireEvent.keyDown(window, { key: 'Enter' })
    await runFor(2_500)
    expect(container.textContent).toContain('身份资料确认')
    fireEvent.keyDown(window, { key: ' ' })
    await runFor(2_500)
    // Both confirmations are behind us: the counter is past the second line.
    expect(container.querySelector('.dsh-boot-archive-stepcount')?.textContent).toMatch(/\[ 0[3-6] \/ 06 \]/)
  })

  it('runs the hexadecimal readout and the welcome act', async () => {
    const { container, onFinished } = mount({ screenRequireInteraction: false })
    // The readout is one of the five lines; catch it while it runs.
    await runSlowly(8_400, 100)
    expect(container.querySelectorAll('.dsh-boot-archive-hex span').length).toBe(12)
    await runFor(8_000)
    expect(container.querySelector('.dsh-boot-archive-welcome')?.className).toContain('dsh-boot-archive-in')
    expect(container.textContent).toContain('WELCOME TO')
    await runFor(6_000)
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('applies the theme and glass switches to the root', () => {
    const dark = mount({ screenTheme: 'dark', screenGlass: true })
    expect(dark.container.querySelector('.dsh-boot-archive-root')?.className).toContain('dsh-boot-archive-dark')
    expect(dark.container.querySelector('.dsh-boot-archive-root')?.className).toContain('dsh-boot-archive-glass')
    cleanup()
    const plain = mount({ screenTheme: 'light', screenGlass: false })
    expect(plain.container.querySelector('.dsh-boot-archive-root')?.className).not.toContain('dsh-boot-archive-dark')
    expect(plain.container.querySelector('.dsh-boot-archive-root')?.className).not.toContain('dsh-boot-archive-glass')
  })

  it('scales the timeline with the speed setting', async () => {
    const { container } = mount({ screenSpeed: 3 })
    await runFor(300)
    expect(container.querySelector('.dsh-boot-archive-black-inner')?.className).toContain('dsh-boot-archive-in')
  })

  it('hides the skip control when skipping is off and shows it otherwise', async () => {
    const withSkip = mount()
    await runFor(4_000)
    expect(withSkip.container.textContent).toContain('skip')
    cleanup()
    const without = mount({ screenAllowSkip: false })
    await runFor(4_000)
    expect(without.container.textContent).not.toContain('skip')
  })

  it('keeps the interface voice out of the frame when sound is off', async () => {
    const silent = mount({ screenSound: false })
    await runFor(4_000)
    expect(silent.container.querySelector('.dsh-boot-archive-voice')?.className).not.toContain('dsh-boot-archive-in')
  })

  it('advances the clock beside the revision line', async () => {
    const { container } = mount()
    const clock = (): string => container.querySelector('.dsh-boot-archive-meta-2')?.textContent ?? ''
    await runFor(1_200)
    expect(clock()).toMatch(/^\d{2}:\d{2}:\d{2}$/)
    const before = clock()
    await runFor(1_000)
    expect(clock()).not.toBe('')
    expect(before).toMatch(/^\d{2}:\d{2}:\d{2}$/)
  })

  it('runs with no audio at all', async () => {
    const { container } = mount({}, vi.fn(), null)
    await runFor(1_000)
    expect(container.querySelector('.dsh-boot-archive-root')).toBeTruthy()
  })

  it('leaves on the skip control', async () => {
    const { container, onFinished } = mount()
    await runFor(5_000)
    const skip = container.querySelector('.dsh-boot-archive-skip')
    expect(skip).toBeTruthy()
    fireEvent.click(skip as Element)
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('ignores a second skip after the sequence finished', async () => {
    const { onFinished } = mount({ screenRequireInteraction: false })
    await runFor(60_000)
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('shows the highlight and hands off at the very end', async () => {
    const { container, onFinished } = mount({ screenRequireInteraction: false })
    await runSlowly(30_000, 100)
    expect(onFinished).toHaveBeenCalledOnce()
    expect(container.querySelector('.dsh-boot-archive-root')?.className).toContain('dsh-boot-archive-handoff')
  })

  it('ignores Enter and Space while nothing waits for a confirmation', async () => {
    const { container } = mount()
    await runFor(900)
    fireEvent.keyDown(window, { key: 'Enter' })
    fireEvent.keyDown(window, { key: ' ' })
    expect(container.querySelector('.dsh-boot-archive-root')).toBeTruthy()
  })

  it('shows the highlight for a moment before handing off', async () => {
    const { container, onFinished } = mount({ screenRequireInteraction: false, screenSpeed: 0.35 })
    let sawHighlight = false
    for (let elapsed = 0; elapsed < 80_000 && !sawHighlight; elapsed += 100) {
      await act(async () => { await vi.advanceTimersByTimeAsync(100) })
      sawHighlight = container.querySelector<HTMLElement>('.dsh-boot-archive-flash')?.style.opacity === '0.72'
    }
    expect(sawHighlight).toBe(true)
    await runSlowly(30_000, 200)
    expect(onFinished).toHaveBeenCalledOnce()
  })

  it('starts from a frame with no act applied', async () => {
    expect(INITIAL_FRAME.blackIn).toBe(false)
    expect(INITIAL_FRAME.welcomeOn).toBe(false)
  })
})
