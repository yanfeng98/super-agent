/**
 * The server-rendering path: without a document there is no body to portal
 * into, so a playing clip renders nothing instead of throwing.
 */
import { renderToString } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import { IntroOverlay } from '../src/client/IntroOverlay.tsx'
import type { IntroOverlayProps, PlaybackState } from '../src/client/contract.ts'

const PLAYING: PlaybackState = {
  phase: 'loading',
  stage: 'video',
  clipId: 'builtin:brand',
  url: '/dsh-boot-animation/media/builtin%3Abrand?v=1',
  reason: 'app-start',
  nonce: 1,
  preview: false,
}

/** Props over one playback state, as the slot runtime composes them. */
function props(playback: PlaybackState): IntroOverlayProps {
  return {
    usePlayback: <T,>(selector: (value: PlaybackState) => T): T => selector(playback),
    useSettings: <T,>(selector: (value: { fit: 'cover' | 'contain' }) => T): T => selector({ fit: 'cover' }),
    stop: () => {},
    t: (key: string) => key,
  } as IntroOverlayProps
}

describe('IntroOverlay without a document', () => {
  it('renders nothing when there is no body to portal into', () => {
    expect(typeof document).toBe('undefined')
    expect(renderToString(<IntroOverlay {...props(PLAYING)} />)).toBe('')
    expect(renderToString(<IntroOverlay {...props({ ...PLAYING, url: null })} />)).toBe('')
  })
})
