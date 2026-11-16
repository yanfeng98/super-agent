/**
 * Startup intro - Host face.
 *
 * Owns exactly the Host resources the browser cannot reach by itself: the four
 * shipped clips and the user's own video files (over an HTTP route that carries
 * ranges and validators), the answer to "which clip plays now", and the durable
 * per-conversation override.
 *
 * The Host does not read the media eagerly: clips are discovered and stat'ed on
 * the first question, and bytes are streamed only when a browser asks for them.
 *
 * Every route is owner-only. The bytes are the user's own files, so an
 * authenticated non-owner must not read them, and no route may become a
 * filesystem reader for an arbitrary path: clip ids are validated against the
 * discovered catalog and never used as a path.
 * @module @deepseek-ai/dsh-client-ui-boot-animation
 */
import { mkdirSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { IncomingMessage, ServerResponse } from 'node:http'
import type { Context, Volatile } from '@deepseek-ai/cordis'
import type {} from '@deepseek-ai/dsh-host-webserver'
import type {} from '@deepseek-ai/dsh-settings'
import type {} from '@deepseek-ai/dsh-storage-domain'
import z from '@deepseek-ai/schemastery'
import { isAudioName } from './audio-names.ts'
import { SEQUENCE_MODES } from './screen.ts'
import { AudioCatalog, audioDir } from './host/audio.ts'
import { ClipCatalog } from './host/catalog.ts'
import { serveClip } from './host/media.ts'
import { bootAnimationDomain, SessionOverrides, normalizeSessionId } from './host/overrides.ts'
import { stagesOf } from './host/plan.ts'
import { IntroResolver } from './host/resolve.ts'
import type { FitMode, PlayTiming, SequenceMode } from './types.ts'

/** Stable Cordis plugin name. */
export const name = 'dsh-client-ui-boot-animation'

/** The only Host service this plugin needs; storage is optional. */
export const inject = ['webServer']

/** Every route this plugin owns lives under this prefix. */
const ROUTE = '/dsh-boot-animation'

/** Largest `POST session` body accepted, in bytes. */
const MAX_BODY_BYTES = 4096

/** The shipped clips, in the order the picker lists them. */
const BUILTINS = [
  { key: 'brand', fileName: 'deepseek-brand-intro.mp4' },
  { key: 'awakening', fileName: 'deepseek-awakening-intro.mp4' },
  { key: 'cyberpunk', fileName: 'deepseek-cyberpunk-intro.mp4' },
  { key: 'startup', fileName: 'deepseek-startup-intro.mp4' },
] as const

/** The archive sequence's settings, as mutable runtime values. */
export interface Config {
  /** Color scheme of the sequence: 'light' is the warm paper plinth, 'dark' the dark one. */
  readonly screenTheme: Volatile<'light' | 'dark'>
  /** Multiplier for every fixed duration in the sequence; 0.35 is slow, 3 is fast. */
  readonly screenSpeed: Volatile<number>
  /** Whether the first two lines wait for a click, Enter, or Space. */
  readonly screenRequireInteraction: Volatile<boolean>
  /** Whether the skip control is shown and Escape dismisses the sequence. */
  readonly screenAllowSkip: Volatile<boolean>
  /** Whether interface cues play at all. */
  readonly screenSound: Volatile<boolean>
  /** Volume of the interface cues, 0 to 1. */
  readonly screenSoundVolume: Volatile<number>
  /** Whether the five lines are spoken. */
  readonly screenVoice: Volatile<boolean>
  /** Where the voice comes from: 'asset' prefers the recorded takes, 'tts' always synthesizes. */
  readonly screenVoiceSource: Volatile<'asset' | 'tts'>
  /** Language spoken: 'en', 'zh', or 'both'. */
  readonly screenVoiceLang: Volatile<'en' | 'zh' | 'both'>
  /** Exact installed voice name, or an empty string to pick one automatically. */
  readonly screenVoiceName: Volatile<string>
  /** Speaking rate for synthesized lines. */
  readonly screenVoiceRate: Volatile<number>
  /** Pitch for synthesized lines; below 1 is deeper. */
  readonly screenVoicePitch: Volatile<number>
  /** Volume of the spoken lines, 0 to 1. */
  readonly screenVoiceVolume: Volatile<number>
  /** Whether a short cue plays before a spoken line. */
  readonly screenVoiceCue: Volatile<boolean>
  /** Silence in milliseconds between the cue and the spoken line. */
  readonly screenVoiceLead: Volatile<number>
  /** Source of the interface cues: 'audio' plays the shipped files, 'synth' synthesizes them. */
  readonly screenSfx: Volatile<'audio' | 'synth'>
  /** Name in the closing welcome line and on the identity card. */
  readonly screenIdentity: Volatile<string>
  /** Identity number on the identity card and the confirmation line. */
  readonly screenIdentityId: Volatile<string>
  /** Access level shown on the identity card. */
  readonly screenAccessLevel: Volatile<string>
  /** First line of the brand block in the top-left corner. */
  readonly screenBrand1: Volatile<string>
  /** Second line of the brand block. */
  readonly screenBrand2: Volatile<string>
  /** Letter-spaced mark in the third brand line. */
  readonly screenBrand3a: Volatile<string>
  /** Suffix after the brand mark. */
  readonly screenBrand3b: Volatile<string>
  /** First line of the black prologue wordmark. */
  readonly screenOrgZh: Volatile<string>
  /** Second line of the black prologue wordmark. */
  readonly screenOrgEn: Volatile<string>
  /** Subtitle under the prologue wordmark. */
  readonly screenOrgSub: Volatile<string>
  /** Text in the black box of the closing act. */
  readonly screenWelcomeBox: Volatile<string>
  /** Text in the bottom-right footer. */
  readonly screenFooter: Volatile<string>
  /** Accent color for the status bar, progress bar, and reticle. */
  readonly screenAccent: Volatile<string>
  /** First color of the chromatic split during the reveal. */
  readonly screenGlitchA: Volatile<string>
  /** Second color of the chromatic split during the reveal. */
  readonly screenGlitchB: Volatile<string>
  /** Whether the plinth and prologue become translucent frosted glass. */
  readonly screenGlass: Volatile<boolean>
  /** Opacity of the glass layers, 0.15 to 0.95. */
  readonly screenGlassOpacity: Volatile<number>
  /** Frosted blur radius in pixels. */
  readonly screenGlassBlur: Volatile<number>
  /** Saturation percentage behind the glass. */
  readonly screenGlassSaturate: Volatile<number>
  /** Blur radius in pixels applied behind the overlay. */
  readonly screenGlassBackdropBlur: Volatile<number>

  /** Whether the intro plays at all. */
  enabled: Volatile<boolean>
  /** Which moment auto-plays it. */
  timing: Volatile<PlayTiming>
  /** How the clip fits the window. */
  fit: Volatile<FitMode>
  /** Play a different clip each time instead of the selected one. */
  random: Volatile<boolean>
  /** The clip id played when random playback is off. */
  selectedClip: Volatile<string>
  /** Which stages run, and in which order. */
  sequence: Volatile<SequenceMode>
}

/** Defaults: on, once per launch, the archive sequence followed by the brand clip. */
export const Config = z.object({
  enabled: z.boolean().default(true).volatile(),
  timing: z.union(['app-start', 'entry'] as const).default('app-start').volatile(),
  fit: z.union(['cover', 'contain'] as const).default('cover').volatile(),
  random: z.boolean().default(false).volatile(),
  selectedClip: z.string().min(1).default('builtin:brand').volatile(),
  sequence: z.union(SEQUENCE_MODES).default('archive-then-video').volatile(),
  screenTheme: z.union(['light', 'dark'] as const).default('light').volatile(),
  screenSpeed: z.number().min(0.35).max(3).default(1).volatile(),
  screenRequireInteraction: z.boolean().default(true).volatile(),
  screenAllowSkip: z.boolean().default(true).volatile(),
  screenSound: z.boolean().default(true).volatile(),
  screenSoundVolume: z.number().min(0).max(1).default(0.5).volatile(),
  screenVoice: z.boolean().default(true).volatile(),
  screenVoiceSource: z.union(['asset', 'tts'] as const).default('asset').volatile(),
  screenVoiceLang: z.union(['en', 'zh', 'both'] as const).default('en').volatile(),
  screenVoiceName: z.string().default('').volatile(),
  screenVoiceRate: z.number().min(0.5).max(2).default(0.88).volatile(),
  screenVoicePitch: z.number().min(0).max(2).default(0.85).volatile(),
  screenVoiceVolume: z.number().min(0).max(1).default(0.9).volatile(),
  screenVoiceCue: z.boolean().default(true).volatile(),
  screenVoiceLead: z.number().min(0).max(1200).default(240).volatile(),
  screenSfx: z.union(['audio', 'synth'] as const).default('audio').volatile(),
  screenIdentity: z.string().default('JOYCE MOORE').volatile(),
  screenIdentityId: z.string().default('0087').volatile(),
  screenAccessLevel: z.string().default('3').volatile(),
  screenBrand1: z.string().default('DEEPSEEK').volatile(),
  screenBrand2: z.string().default('SYNTHESIZE INTELLIGENCE').volatile(),
  screenBrand3a: z.string().default('HARNESS').volatile(),
  screenBrand3b: z.string().default('OS').volatile(),
  screenOrgZh: z.string().default('深度求索').volatile(),
  screenOrgEn: z.string().default('DEEPSEEK').volatile(),
  screenOrgSub: z.string().default('DEEPSEEK HARNESS').volatile(),
  screenWelcomeBox: z.string().default('DEEPSEEK HARNESS').volatile(),
  screenFooter: z.string().default('POWERED BY DEEPSEEK').volatile(),
  screenAccent: z.string().default('#ff7500').volatile(),
  screenGlitchA: z.string().default('#ff3b30').volatile(),
  screenGlitchB: z.string().default('#2971b8').volatile(),
  screenGlass: z.boolean().default(true).volatile(),
  screenGlassOpacity: z.number().min(0.15).max(0.95).default(0.75).volatile(),
  screenGlassBlur: z.number().min(0).max(60).default(22).volatile(),
  screenGlassSaturate: z.number().min(100).max(200).default(160).volatile(),
  screenGlassBackdropBlur: z.number().min(0).max(80).default(28).volatile(),
})

/** Absolute path of one shipped clip, resolved beside the built entry. */
function builtinPath(fileName: string): string {
  return fileURLToPath(new URL(`../assets/${fileName}`, import.meta.url))
}

/** The directory the user drops their own clips into. */
function userVideoDir(): string {
  const home = process.env.DSH_HOME?.trim()
  const root = home === undefined || home === '' ? join(homedir(), '.dsh') : home
  return join(root, 'boot-animation', 'videos')
}

/** JSON reply, never cached: these answers change with settings and files. */
function sendJson(res: ServerResponse, status: number, payload: unknown): void {
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
  })
  res.end(JSON.stringify(payload))
}

/** Plain-text refusal, never cached. */
function sendText(res: ServerResponse, status: number, text: string): void {
  res.writeHead(status, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' })
  res.end(text)
}

/** The raw request target's query, without the leading `?`. */
function queryOf(req: IncomingMessage): URLSearchParams {
  const url = req.url ?? ''
  const at = url.indexOf('?')
  return new URLSearchParams(at === -1 ? '' : url.slice(at + 1))
}

/** The pathname of the request target, without a query. */
function pathOf(req: IncomingMessage): string {
  const url = req.url ?? ''
  const at = url.indexOf('?')
  return at === -1 ? url : url.slice(0, at)
}

/**
 * Read a small JSON object body.
 * @param req - the request to read.
 * @returns the parsed object, or null when the body is absent, oversized, or not an object.
 */
function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown> | null> {
  return new Promise((resolve) => {
    const chunks: Buffer[] = []
    let size = 0
    let settled = false
    const settle = (value: Record<string, unknown> | null): void => {
      if (settled) return
      settled = true
      resolve(value)
    }
    req.on('data', (chunk: Buffer) => {
      size += chunk.length
      if (size > MAX_BODY_BYTES) {
        settle(null)
        return
      }
      chunks.push(chunk)
    })
    req.on('error', () => { settle(null) })
    req.on('end', () => {
      if (settled) return
      try {
        const parsed: unknown = JSON.parse(Buffer.concat(chunks).toString('utf8'))
        settle(typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)
          ? parsed as Record<string, unknown>
          : null)
      } catch {
        settle(null)
      }
    })
  })
}

/**
 * Serve the intro and record the user's per-conversation answer.
 * @param ctx - the plugin context owning the routes.
 * @param config - resolved plugin configuration.
 */
export function apply(ctx: Context, config: Config): void {
  const userDir = userVideoDir()
  try {
    mkdirSync(userDir, { recursive: true })
  } catch (error: unknown) {
    // A missing directory costs the user's own clips, never the shipped ones.
    ctx.logger.warn(`boot-animation: user clip directory is unavailable: ${String(error)}`)
  }
  const catalog = new ClipCatalog(
    BUILTINS.map(builtin => ({ key: builtin.key, path: builtinPath(builtin.fileName) })),
    [userDir],
  )
  const audio = new AudioCatalog(audioDir(import.meta.url))
  let overrides: SessionOverrides | null = null
  const resolver = new IntroResolver(
    catalog,
    () => overrides,
    () => ({ random: config.random.get(), selectedClip: config.selectedClip.get() }),
  )

  // Storage is optional: without it the intro still plays, only the
  // per-conversation override is unavailable.
  ctx.inject(['storageDomain'], (child) => {
    child.effect(() => {
      let disposed = false
      void child.storageDomain.open(bootAnimationDomain).then((domain) => {
        if (disposed) {
          void domain.close()
          return
        }
        overrides = new SessionOverrides(domain)
      }).catch((error: unknown) => {
        child.logger.warn(`boot-animation: per-conversation overrides are unavailable: ${String(error)}`)
      })
      return () => {
        disposed = true
        overrides = null
      }
    }, 'boot-animation: conversation overrides')
  })

  // The preferences are edited by this package's own settings section, so the
  // generated form must not also claim them.
  ctx.inject(['settings'], (child) => {
    child.effect(() => child.settings.configure({ auto: false }, ctx.fiber))
  })

  const ownerOnly = (peer: { principal: { kind: string } } | undefined): boolean =>
    peer !== undefined && peer.principal.kind === 'owner'

  ctx.effect(() => ctx.webServer.register({
    kind: 'prefix',
    path: `${ROUTE}/media`,
    auth: 'required',
    handler: (req, res, peer) => {
      if (!ownerOnly(peer)) { sendText(res, 403, 'forbidden'); return }
      const raw = pathOf(req)
      const prefix = `${ROUTE}/media/`
      if (!raw.startsWith(prefix)) { sendText(res, 404, 'dsh-boot-animation: no such clip'); return }
      let id: string
      try {
        id = decodeURIComponent(raw.slice(prefix.length))
      } catch {
        sendText(res, 404, 'dsh-boot-animation: no such clip')
        return
      }
      const clip = catalog.get(id)
      if (clip === null) { sendText(res, 404, 'dsh-boot-animation: no such clip'); return }
      serveClip(req, res, clip)
    },
  }), 'boot-animation: clip media')

  ctx.effect(() => ctx.webServer.register({
    kind: 'prefix',
    path: `${ROUTE}/audio`,
    auth: 'required',
    handler: (req, res, peer) => {
      if (!ownerOnly(peer)) { sendText(res, 403, 'forbidden'); return }
      const prefix = `${ROUTE}/audio/`
      const raw = pathOf(req)
      if (!raw.startsWith(prefix)) { sendText(res, 404, 'dsh-boot-animation: no such audio file'); return }
      const name = raw.slice(prefix.length).replace(/\.mp3$/u, '')
      if (!isAudioName(name)) { sendText(res, 404, 'dsh-boot-animation: no such audio file'); return }
      const file = audio.get(name)
      /* v8 ignore next 2 -- every shipped name has a file; this only guards a damaged install. */
      if (file === null) { sendText(res, 404, 'dsh-boot-animation: no such audio file'); return }
      serveClip(req, res, file)
    },
  }), 'boot-animation: sequence audio')

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: `${ROUTE}/catalog.json`,
    auth: 'required',
    handler: (req, res, peer) => {
      if (!ownerOnly(peer)) { sendText(res, 403, 'forbidden'); return }
      const body = resolver.catalogFor(normalizeSessionId(queryOf(req).get('session')))
      sendJson(res, 200, { ...body, overridesAvailable: overrides !== null })
    },
  }), 'boot-animation: clip catalog')

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: `${ROUTE}/active.json`,
    auth: 'required',
    handler: (req, res, peer) => {
      if (!ownerOnly(peer)) { sendText(res, 403, 'forbidden'); return }
      const query = queryOf(req)
      const mode = query.get('mode') === 'random' ? 'random' : 'active'
      sendJson(res, 200, {
        stages: stagesOf(config.sequence.get()),
        ...resolver.active(normalizeSessionId(query.get('session')), mode),
      })
    },
  }), 'boot-animation: active clip')

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: `${ROUTE}/session`,
    auth: 'required',
    handler: (req, res, peer) => {
      if (!ownerOnly(peer)) { sendText(res, 403, 'forbidden'); return }
      if (req.method !== 'POST') { sendText(res, 405, 'POST required'); return }
      void (async () => {
        const body = await readJsonBody(req)
        if (body === null) { sendText(res, 400, 'expected a JSON object body'); return }
        const sessionId = normalizeSessionId(body.sessionId)
        if (sessionId === null) { sendText(res, 400, 'expected a sessionId'); return }
        const store = overrides
        if (store === null) { sendText(res, 503, 'conversation overrides are unavailable'); return }
        const clipId = body.clipId === undefined ? undefined : body.clipId
        if (clipId !== undefined && clipId !== null) {
          if (typeof clipId !== 'string' || catalog.get(clipId) === null) {
            sendText(res, 404, 'no such clip')
            return
          }
        }
        if (body.replay !== undefined && typeof body.replay !== 'boolean') {
          sendText(res, 400, 'replay must be a boolean')
          return
        }
        try {
          const next = await store.write(sessionId, {
            ...(body.replay === undefined ? {} : { replay: body.replay }),
            ...(clipId === undefined ? {} : { clipId: clipId === null ? null : clipId }),
          })
          sendJson(res, 200, next)
        } catch (error: unknown) {
          ctx.logger.warn(`boot-animation: could not store the conversation override: ${String(error)}`)
          sendText(res, 500, 'could not store the conversation override')
        }
      })()
    },
  }), 'boot-animation: conversation override')
}
