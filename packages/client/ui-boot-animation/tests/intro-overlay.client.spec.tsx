// @vitest-environment jsdom
/**
 * The full-screen intro layer: what it shows for each playback state, how it
 * survives a stall or a broken clip, and how every way out (ending, skipping,
 * Escape, teardown) releases the element.
 */
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { IntroOverlay } from '../src/client/IntroOverlay.tsx'
import { DEFAULT_SETTINGS } from '../src/client/contract.ts'
import type { IntroOverlayProps, IntroSettings, PlaybackState } from '../src/client/contract.ts'

const IDLE: PlaybackState = {
  phase: 'idle', stage: 'none', clipId: null, url: null, reason: 'none', nonce: 0, preview: false,
}

const PLAYING: PlaybackState = {
  phase: 'loading',
  stage: 'video',
  clipId: 'builtin:brand',
  url: '/dsh-boot-animation/media/builtin%3Abrand?v=1',
  reason: 'app-start',
  nonce: 1,
  preview: false,
}

let playMock: ReturnType<typeof vi.fn>
let pauseMock: ReturnType<typeof vi.fn>
let loadMock: ReturnType<typeof vi.fn>
let fullscreenMock: ReturnType<typeof vi.fn>

/** The props the component reads, with a supplied stop spy. */
function makeProps(
  playback: PlaybackState,
  stop: ReturnType<typeof vi.fn> = vi.fn(),
  fit: 'cover' | 'contain' = 'cover',
) {
  const markPlaying = vi.fn()
  const advance = vi.fn()
  const props = {
    usePlayback: <T,>(selector: (value: PlaybackState) => T): T => selector(playback),
    useSettings: <T,>(selector: (value: IntroSettings) => T): T => selector({ ...DEFAULT_SETTINGS, fit }),
    markPlaying,
    advance,
    stop,
    t: (key: string) => key,
  } as IntroOverlayProps
  return { props, stop, markPlaying, advance }
}

/** The one video the portal mounted. */
function video(): HTMLVideoElement {
  const element = document.querySelector('video')
  if (element === null) throw new Error('expected the overlay to mount a video')
  return element
}

beforeEach(() => {
  playMock = vi.fn(() => Promise.resolve())
  pauseMock = vi.fn()
  loadMock = vi.fn()
  fullscreenMock = vi.fn(() => Promise.resolve())
  Object.defineProperty(HTMLMediaElement.prototype, 'play', { configurable: true, writable: true, value: playMock })
  Object.defineProperty(HTMLMediaElement.prototype, 'pause', { configurable: true, writable: true, value: pauseMock })
  Object.defineProperty(HTMLMediaElement.prototype, 'load', { configurable: true, writable: true, value: loadMock })
  Object.defineProperty(HTMLVideoElement.prototype, 'requestFullscreen', { configurable: true, writable: true, value: fullscreenMock })
  Object.defineProperty(document, 'fullscreenElement', { configurable: true, writable: true, value: null })
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  Reflect.deleteProperty(document, 'fullscreenElement')
})

describe('IntroOverlay', () => {
  it('mounts the archive stage instead of a video when that stage is playing', () => {
    vi.useFakeTimers()
    const { props } = makeProps({ ...PLAYING, stage: 'archive', clipId: null, url: null })
    render(<IntroOverlay {...props} />)
    expect(document.querySelector('video')).toBeNull()
    expect(document.querySelector('.dsh-boot-archive-root')).toBeTruthy()
  })

  it('ends the whole experience when the archive stage is skipped', () => {
    vi.useFakeTimers()
    const { props, stop, advance } = makeProps({ ...PLAYING, stage: 'archive', clipId: null, url: null })
    render(<IntroOverlay {...props} />)
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(stop).toHaveBeenCalledOnce()
    expect(advance).not.toHaveBeenCalled()
  })

  it('marks the document while it owns the window, and clears the mark on unmount', () => {
    const { props } = makeProps(PLAYING)
    const view = render(<IntroOverlay {...props} />)
    expect(document.documentElement.hasAttribute('data-dsh-splash')).toBe(true)
    view.unmount()
    expect(document.documentElement.hasAttribute('data-dsh-splash')).toBe(false)
  })

  it('renders nothing until a clip is playing', () => {
    const { props } = makeProps(IDLE)
    render(<IntroOverlay {...props} />)
    expect(document.querySelector('video')).toBeNull()
    expect(screen.queryByRole('button', { name: 'overlay.skip' })).toBeNull()
  })

  it('points the element at the clip, loads it, and shows the loading status', () => {
    const { props } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    expect(video().getAttribute('src')).toBe(PLAYING.url)
    expect(loadMock).toHaveBeenCalled()
    expect(playMock).toHaveBeenCalled()
    expect(screen.getByText('overlay.loading')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'overlay.skip' })).toBeTruthy()
  })

  it('honors the contain fit without changing what the user sees', () => {
    const { props } = makeProps(PLAYING, vi.fn(), 'contain')
    render(<IntroOverlay {...props} />)
    expect(video().getAttribute('src')).toBe(PLAYING.url)
  })

  it('reports the stall, then leaves once no frame arrives', () => {
    vi.useFakeTimers()
    const { props, advance } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    act(() => { vi.advanceTimersByTime(24_999) })
    expect(advance).not.toHaveBeenCalled()
    act(() => { vi.advanceTimersByTime(1) })
    expect(document.body.textContent).toContain('overlay.stalled')
    expect(advance).not.toHaveBeenCalled()
    act(() => { vi.advanceTimersByTime(3_000) })
    expect(advance).toHaveBeenCalledOnce()
  })

  it('lets a late first frame cancel the stall exit', () => {
    vi.useFakeTimers()
    const { props, stop, markPlaying } = makeProps({ ...PLAYING, phase: 'loading' })
    render(<IntroOverlay {...props} />)
    act(() => { vi.advanceTimersByTime(25_000) })
    expect(document.body.textContent).toContain('overlay.stalled')
    fireEvent(video(), new Event('playing'))
    expect(document.body.textContent).not.toContain('overlay.stalled')
    act(() => { vi.advanceTimersByTime(5_000) })
    expect(stop).not.toHaveBeenCalled()
    expect(markPlaying).toHaveBeenCalled()
  })

  it('clears a pending stall exit when it unmounts first', () => {
    vi.useFakeTimers()
    const { props, stop } = makeProps(PLAYING)
    const { unmount } = render(<IntroOverlay {...props} />)
    act(() => { vi.advanceTimersByTime(25_000) })
    unmount()
    act(() => { vi.advanceTimersByTime(5_000) })
    expect(stop).not.toHaveBeenCalled()
  })

  it('tells the plugin body the first frame arrived', () => {
    const { props, markPlaying } = makeProps({ ...PLAYING, phase: 'loading' })
    render(<IntroOverlay {...props} />)
    fireEvent(video(), new Event('playing'))
    expect(markPlaying).toHaveBeenCalledOnce()
  })

  it('does not stop a later clip when it unmounts before the error leaves', () => {
    vi.useFakeTimers()
    const { props, stop } = makeProps({ ...PLAYING, phase: 'loading' })
    const { unmount } = render(<IntroOverlay {...props} />)
    fireEvent(video(), new Event('error'))
    unmount()
    act(() => { vi.advanceTimersByTime(10_000) })
    expect(stop).not.toHaveBeenCalled()
  })

  it('cancels the stall watchdog once a frame is playing', () => {
    vi.useFakeTimers()
    const { props, stop } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    fireEvent(video(), new Event('playing'))
    act(() => { vi.advanceTimersByTime(25_000) })
    expect(stop).not.toHaveBeenCalled()
  })

  it('leaves for the next stage when the clip ends', () => {
    const { props, advance } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    fireEvent(video(), new Event('ended'))
    expect(advance).toHaveBeenCalledOnce()
  })

  it('reports a broken clip and leaves after four seconds', () => {
    vi.useFakeTimers()
    const { props, advance } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    fireEvent(video(), new Event('error'))
    expect(screen.getByText('overlay.error')).toBeTruthy()
    expect(screen.queryByText('overlay.loading')).toBeNull()
    act(() => { vi.advanceTimersByTime(3_999) })
    expect(advance).not.toHaveBeenCalled()
    act(() => { vi.advanceTimersByTime(1) })
    expect(advance).toHaveBeenCalledOnce()
  })

  it('stops on Escape only while the intro is mounted', () => {
    const { props, stop } = makeProps(PLAYING)
    const view = render(<IntroOverlay {...props} />)
    fireEvent.keyDown(window, { key: 'a' })
    expect(stop).not.toHaveBeenCalled()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(stop).toHaveBeenCalledOnce()
    view.unmount()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(stop).toHaveBeenCalledOnce()
  })

  it('stops when the user skips, without activating the layer', () => {
    const { props, stop } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    fireEvent.click(screen.getByRole('button', { name: 'overlay.skip' }))
    expect(stop).toHaveBeenCalledOnce()
    expect(fullscreenMock).not.toHaveBeenCalled()
    expect(playMock).toHaveBeenCalledOnce()
  })

  it('turns a click on the picture into sound and fullscreen, once', () => {
    const { props } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    const picture = document.querySelector('[role="presentation"]')
    if (picture === null) throw new Error('expected the overlay layer')
    const element = video()
    expect(element.muted).toBe(true)

    fireEvent.click(picture)
    expect(element.muted).toBe(false)
    expect(fullscreenMock).toHaveBeenCalledOnce()
    expect(playMock).toHaveBeenCalledTimes(2)

    // Already unmuted and already fullscreen: neither is requested again.
    Object.defineProperty(document, 'fullscreenElement', { configurable: true, get: () => element })
    fireEvent.click(picture)
    expect(element.muted).toBe(false)
    expect(fullscreenMock).toHaveBeenCalledOnce()
    expect(playMock).toHaveBeenCalledTimes(3)
  })

  it('contains a refused fullscreen request', async () => {
    fullscreenMock.mockRejectedValueOnce(new Error('fullscreen denied'))
    const { props } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    const picture = document.querySelector('[role="presentation"]')
    if (picture === null) throw new Error('expected the overlay layer')
    fireEvent.click(picture)
    await act(async () => { await Promise.resolve() })
    expect(fullscreenMock).toHaveBeenCalledOnce()
  })

  it('offers a tap when the browser refuses the initial autoplay', async () => {
    playMock.mockRejectedValueOnce(new Error('autoplay blocked'))
    const { props } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    expect(await screen.findByText('overlay.tap')).toBeTruthy()
    expect(screen.queryByText('overlay.loading')).toBeNull()
  })

  it('offers a tap when resuming after the click is refused', async () => {
    const { props } = makeProps(PLAYING)
    render(<IntroOverlay {...props} />)
    playMock.mockRejectedValueOnce(new Error('resume blocked'))
    const picture = document.querySelector('[role="presentation"]')
    if (picture === null) throw new Error('expected the overlay layer')
    fireEvent.click(picture)
    expect(await screen.findByText('overlay.tap')).toBeTruthy()
  })

  it('shows the preview badge and the sound hint while playing', () => {
    const { props } = makeProps({ ...PLAYING, phase: 'playing', preview: true })
    render(<IntroOverlay {...props} />)
    expect(screen.getByText('overlay.preview')).toBeTruthy()
    expect(screen.getByText('overlay.hint')).toBeTruthy()
    expect(screen.queryByText('overlay.loading')).toBeNull()
  })

  it('releases the element and its stall timer on unmount', () => {
    vi.useFakeTimers()
    const { props, stop } = makeProps(PLAYING)
    const view = render(<IntroOverlay {...props} />)
    const element = video()
    view.unmount()
    expect(pauseMock).toHaveBeenCalled()
    expect(loadMock).toHaveBeenCalledTimes(2)
    expect(element.hasAttribute('src')).toBe(false)
    act(() => { vi.advanceTimersByTime(25_000) })
    expect(stop).not.toHaveBeenCalled()
  })

  it('releases a detached element even when it throws on pause', () => {
    pauseMock.mockImplementation(() => { throw new Error('detached') })
    const { props } = makeProps(PLAYING)
    const view = render(<IntroOverlay {...props} />)
    expect(() => { view.unmount() }).not.toThrow()
  })
})
