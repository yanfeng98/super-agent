/**
 * The startup experience: one full-window overlay that runs the archive
 * sequence and/or a clip, in the order the Host asked for.
 *
 * It renders what the store says is playing and reports what happened; every
 * decision lives in `decidePlay` and the plugin body. It portals to `body` so
 * ancestor clipping and stacking contexts cannot shrink it.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/IntroOverlay
 */
import { useCallback, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { ReactNode } from 'react'
import type { IntroOverlayProps } from './contract.ts'
import { ArchiveStage } from './archive/ArchiveStage.tsx'
import { archiveLabels } from './archive/labels.ts'
import css from './IntroOverlay.module.css'

/** Give up when no frame arrives in this long. */
const STALL_TIMEOUT_MS = 25_000

/** Detach a media element and release its decoder. */
function releaseVideo(video: HTMLVideoElement): void {
  try {
    video.pause()
    video.removeAttribute('src')
    video.load()
  } catch (error) {
    // A detached element can throw from any of the calls above and there is
    // nothing left to release either way; the error is explicitly discarded.
    void error
  }
}

/**
 * Render the intro while the store has something playing.
 * @param props - composed slot props plus the injected playback face.
 * @returns the overlay, or null when nothing is playing.
 */
export function IntroOverlay({
  usePlayback, useSettings, markPlaying, advance, stop, t,
}: IntroOverlayProps): ReactNode {
  const playback = usePlayback(value => value)
  const settings = useSettings(value => value)
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const [needsTap, setNeedsTap] = useState(false)
  const [failed, setFailed] = useState(false)
  const [stalled, setStalled] = useState(false)
  /** Timers owned by this overlay; a stale one must never stop a later clip. */
  const timers = useRef<number[]>([])
  /** The pending "leave after a stall" timer, cancelled if a frame finally arrives. */
  const leaveTimer = useRef<number | null>(null)
  const url = playback.url
  const archive = playback.stage === 'archive'

  const schedule = (ms: number, run: () => void): void => {
    timers.current.push(window.setTimeout(() => { run() }, ms))
  }

  useEffect(() => {
    if (url === null) return undefined
    const video = videoRef.current
    /* v8 ignore next -- the portal's video is attached before effects run; the null guard only covers a render that produced none */
    if (video === null) return undefined
    setFailed(false)
    setNeedsTap(false)
    setStalled(false)
    video.src = url
    video.load()
    let timer = window.setTimeout(() => {
      // No frame at all: say so before leaving, instead of vanishing silently.
      setStalled(true)
      leaveTimer.current = window.setTimeout(() => { advance() }, 3_000)
    }, STALL_TIMEOUT_MS)
    const onPlaying = (): void => {
      window.clearTimeout(timer)
      timer = 0
      // A late first frame wins: the stall notice and its exit are cancelled.
      if (leaveTimer.current !== null) {
        window.clearTimeout(leaveTimer.current)
        leaveTimer.current = null
      }
      setStalled(false)
      markPlaying()
    }
    video.addEventListener('playing', onPlaying)
    void video.play().catch(() => { setNeedsTap(true) })
    return () => {
      window.clearTimeout(timer)
      video.removeEventListener('playing', onPlaying)
      releaseVideo(video)
    }
  }, [url, playback.nonce, advance, markPlaying])

  // A desktop shell reads this attribute to hide its caption buttons with the
  // chrome the sequence covers; a browser ignores it entirely.
  useEffect(() => {
    if (playback.stage === 'none') return undefined
    const root = document.documentElement
    root.setAttribute('data-dsh-splash', '')
    return () => { root.removeAttribute('data-dsh-splash') }
  }, [playback.stage])

  useEffect(() => () => {
    for (const timer of timers.current) window.clearTimeout(timer)
    timers.current = []
    if (leaveTimer.current !== null) {
      window.clearTimeout(leaveTimer.current)
      leaveTimer.current = null
    }
  }, [])

  const onKey = useCallback((event: KeyboardEvent): void => {
    if (event.key === 'Escape') stop()
  }, [stop])

  useEffect(() => {
    if (url === null) return undefined
    window.addEventListener('keydown', onKey)
    return () => { window.removeEventListener('keydown', onKey) }
  }, [url, onKey])

  if (archive && typeof document !== 'undefined') {
    return createPortal(
      <ArchiveStage
        key={playback.nonce}
        settings={settings}
        labels={archiveLabels(t)}
        onFinished={advance}
      />,
      document.body,
    )
  }

  if (url === null || typeof document === 'undefined') return null

  const activate = (): void => {
    const video = videoRef.current
    /* v8 ignore next -- a click only reaches this handler while the video is mounted */
    if (video === null) return
    if (video.muted) video.muted = false
    void video.play().catch(() => { setNeedsTap(true) })
    if (document.fullscreenElement === null) {
      void video.requestFullscreen().catch(() => undefined)
    }
  }

  return createPortal((
    <div className={css.root} onClick={activate} role="presentation">
      <video
        ref={videoRef}
        className={settings.fit === 'cover' ? `${css.video} ${css.cover}` : css.video}
        muted
        autoPlay
        playsInline
        preload="auto"
        onEnded={advance}
        onError={() => {
          setFailed(true)
          schedule(4_000, advance)
        }}
      />
      {playback.preview ? <div className={css.badge}>{t('overlay.preview')}</div> : null}
      {failed ? <div className={css.status}>{t('overlay.error')}</div> : null}
      {!failed && stalled ? <div className={css.status}>{t('overlay.stalled')}</div> : null}
      {!failed && !stalled && playback.phase !== 'playing'
        ? <div className={css.status}>{needsTap ? t('overlay.tap') : t('overlay.loading')}</div>
        : null}
      {playback.phase === 'playing' ? <div className={css.hint}>{t('overlay.hint')}</div> : null}
      <button
        type="button"
        className={css.skip}
        title={t('overlay.skipHint')}
        onClick={(event) => { event.stopPropagation(); stop() }}
      >
        {t('overlay.skip')}
      </button>
    </div>
  ), document.body)
}
