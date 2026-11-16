/**
 * The settings page: preferences, the clip library, and this conversation.
 *
 * Three groups, in the order a user meets them: what the intro does, which clips
 * it can play, and what this conversation does differently. Preview and selection
 * are separate actions on purpose - auditioning a clip must not change what plays
 * tomorrow.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/SettingsSection
 */
import type { ChangeEvent, ReactNode } from 'react'
import type { ClipInfo, FitMode, PlayTiming, SequenceMode } from '../types.ts'
import { SCREEN_TEXT_LIMIT } from '../screen.ts'
import { CONTROL_GROUPS, boundsOf, optionsOf } from './archive/controls.ts'
import type { ControlSpec } from './archive/controls.ts'
import type { SettingsSectionProps } from './contract.ts'
import css from './SettingsSection.module.css'

/** The sequence choices, in menu order. */
const SEQUENCES: readonly { readonly value: SequenceMode; readonly label: 'sequence.archiveThenVideo' | 'sequence.videoThenArchive' | 'sequence.archiveOnly' | 'sequence.videoOnly' }[] = [
  { value: 'archive-then-video', label: 'sequence.archiveThenVideo' },
  { value: 'video-then-archive', label: 'sequence.videoThenArchive' },
  { value: 'archive-only', label: 'sequence.archiveOnly' },
  { value: 'video-only', label: 'sequence.videoOnly' },
]

/**
 * Narrow a select value to a known sequence.
 * @param value - the chosen option value.
 * @returns the matching mode, or the default when the value is unknown.
 */
function asSequence(value: string): SequenceMode {
  return SEQUENCES.find(choice => choice.value === value)?.value ?? 'archive-then-video'
}

/** Dictionary keys naming the shipped clips, keyed by their id suffix. */
const BUILTIN_LABEL = {
  brand: 'clips.builtin.brand',
  awakening: 'clips.builtin.awakening',
  cyberpunk: 'clips.builtin.cyberpunk',
  startup: 'clips.builtin.startup',
} as const

/** The id suffix of a `builtin:` clip, or null for a user file. */
function builtinKey(clip: ClipInfo): keyof typeof BUILTIN_LABEL | null {
  if (clip.kind !== 'builtin' || !clip.id.startsWith('builtin:')) return null
  const key = clip.id.slice('builtin:'.length)
  return key in BUILTIN_LABEL ? key as keyof typeof BUILTIN_LABEL : null
}

/**
 * Render the startup-intro settings page.
 * @param props - composed slot props plus the injected settings face.
 * @returns the page.
 */
export function SettingsSection(props: SettingsSectionProps): ReactNode {
  const {
    t, useCatalog, useSettings, useSession, refresh, preview, previewArchive, setEnabled, setTiming,
    setFit, setRandom, setSelectedClip, setSequence, setScreen, setSessionReplay, setSessionClip,
    sessionStorageAvailable,
  } = props
  const catalog = useCatalog(value => value)
  const settings = useSettings(value => value)
  const session = useSession(value => value)
  const clips = catalog.clips
  const nameOf = (clip: ClipInfo): string => {
    const key = builtinKey(clip)
    return key === null ? (clip.name ?? clip.id) : t(BUILTIN_LABEL[key])
  }

  const onTiming = (event: ChangeEvent<HTMLInputElement>): void => {
    setTiming(event.target.value === 'entry' ? 'entry' satisfies PlayTiming : 'app-start')
  }
  const onFit = (event: ChangeEvent<HTMLInputElement>): void => {
    setFit(event.target.value === 'contain' ? 'contain' satisfies FitMode : 'cover')
  }

  /** Render one row of the archive stage's settings table. */
  const control = (spec: ControlSpec): ReactNode => {
    const value: string | number | boolean = settings[spec.key]
    const label = (
      <span className={css.rowText}>
        <span className={css.label}>{t(spec.label)}</span>
        {spec.hint === undefined ? null : <span className={css.hint}>{t(spec.hint)}</span>}
      </span>
    )
    const write = (next: string | number | boolean): void => { setScreen(spec.key, next) }
    if (spec.kind === 'toggle') {
      return (
        <label key={spec.key} className={css.row}>
          {label}
          <input type="checkbox" checked={Boolean(value)} onChange={(event) => { write(event.target.checked) }} />
        </label>
      )
    }
    if (spec.kind === 'select') {
      return (
        <label key={spec.key} className={css.row}>
          {label}
          <select className={css.input} value={String(value)} onChange={(event) => { write(event.target.value) }}>
            {optionsOf(spec).map(option => (
              <option key={option.value} value={option.value}>{t(option.label)}</option>
            ))}
          </select>
        </label>
      )
    }
    if (spec.kind === 'range') {
      const [min, max] = boundsOf(spec.key)
      return (
        <label key={spec.key} className={css.row}>
          {label}
          <span className={css.range}>
            <input
              type="range"
              min={min}
              max={max}
              step={(max - min) / 100}
              value={Number(value)}
              onChange={(event) => { write(Number(event.target.value)) }}
            />
            <span className={css.value}>{String(value)}</span>
          </span>
        </label>
      )
    }
    if (spec.kind === 'color') {
      return (
        <label key={spec.key} className={css.row}>
          {label}
          <input
            type="color"
            className={css.swatch}
            value={String(value)}
            onChange={(event) => { write(event.target.value) }}
          />
        </label>
      )
    }
    return (
      <label key={spec.key} className={css.row}>
        {label}
        <input
          type="text"
          className={css.input}
          maxLength={SCREEN_TEXT_LIMIT}
          value={String(value)}
          onChange={(event) => { write(event.target.value) }}
        />
      </label>
    )
  }

  return (
    <div className={css.page}>
      <section className={css.group}>
        <label className={css.row}>
          <span className={css.rowText}>
            <span className={css.label}>{t('enabled.label')}</span>
            <span className={css.hint}>{t('enabled.hint')}</span>
          </span>
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(event) => { setEnabled(event.target.checked) }}
          />
        </label>

        <fieldset className={css.field}>
          <legend className={css.label}>{t('timing.label')}</legend>
          <label className={css.choice}>
            <input type="radio" name="boot-animation-timing" value="app-start"
              checked={settings.timing === 'app-start'} onChange={onTiming} />
            <span>{t('timing.appStart')}</span>
          </label>
          <label className={css.choice}>
            <input type="radio" name="boot-animation-timing" value="entry"
              checked={settings.timing === 'entry'} onChange={onTiming} />
            <span>{t('timing.entry')}</span>
          </label>
        </fieldset>

        <fieldset className={css.field}>
          <legend className={css.label}>{t('fit.label')}</legend>
          <label className={css.choice}>
            <input type="radio" name="boot-animation-fit" value="cover"
              checked={settings.fit === 'cover'} onChange={onFit} />
            <span>{t('fit.cover')}</span>
          </label>
          <label className={css.choice}>
            <input type="radio" name="boot-animation-fit" value="contain"
              checked={settings.fit === 'contain'} onChange={onFit} />
            <span>{t('fit.contain')}</span>
          </label>
        </fieldset>

        <label className={css.row}>
          <span className={css.label}>{t('random.label')}</span>
          <input
            type="checkbox"
            checked={settings.random}
            onChange={(event) => { setRandom(event.target.checked) }}
          />
        </label>
      </section>

      <section className={css.group} id="screen-sequence">
        <div className={css.groupHeader}>
          <h3 className={css.groupTitle}>{t('screen.sequence')}</h3>
          <button type="button" className={css.secondary} onClick={previewArchive}>{t('action.previewSequence')}</button>
        </div>
        <label className={css.row}>
          <span className={css.rowText}>
            <span className={css.label}>{t('sequence.label')}</span>
            <span className={css.hint}>{t('sequence.hint')}</span>
          </span>
          <select
            className={css.input}
            value={settings.sequence}
            onChange={(event) => { setSequence(asSequence(event.target.value)) }}
          >
            {SEQUENCES.map(choice => <option key={choice.value} value={choice.value}>{t(choice.label)}</option>)}
          </select>
        </label>
        {CONTROL_GROUPS.map(group => (
          <section key={group.id} className={css.subgroup} id={group.id}>
            <h4 className={css.subgroupTitle}>{t(group.title)}</h4>
            {group.controls.map(spec => control(spec))}
          </section>
        ))}
      </section>

      <section className={css.group}>
        <div className={css.groupHeader}>
          <h3 className={css.groupTitle}>{t('clips.label')}</h3>
          <button type="button" className={css.secondary} onClick={refresh}>{t('clips.refresh')}</button>
        </div>
        {clips.length === 0
          ? <p className={css.hint}>{t('clips.empty')}</p>
          : (
            <ul className={css.clipList}>
              {clips.map((clip) => {
                const selected = settings.selectedClip === clip.id
                return (
                  <li key={clip.id} className={selected ? `${css.clip} ${css.clipCurrent}` : css.clip}>
                    <span className={css.clipName}>{nameOf(clip)}</span>
                    {selected ? <span className={css.badge}>{t('action.selected')}</span> : null}
                    <button type="button" className={css.secondary} onClick={() => { preview(clip.id) }}>
                      {t('action.preview')}
                    </button>
                    <button
                      type="button"
                      className={css.primary}
                      disabled={selected}
                      onClick={() => { setSelectedClip(clip.id) }}
                    >
                      {t('action.select')}
                    </button>
                  </li>
                )
              })}
            </ul>
          )}
      </section>

      <section className={css.group}>
        <h3 className={css.groupTitle}>{t('session.title')}</h3>
        {!sessionStorageAvailable
          ? <p className={css.hint}>{t('session.unavailable')}</p>
          : session.sessionId === null
            ? <p className={css.hint}>{t('session.noSession')}</p>
            : (
              <>
                <label className={css.row}>
                  <span className={css.label}>{t('session.replay')}</span>
                  <input
                    type="checkbox"
                    checked={catalog.session.replay}
                    onChange={(event) => { setSessionReplay(event.target.checked) }}
                  />
                </label>
                <label className={css.row}>
                  <span className={css.label}>{t('session.clip')}</span>
                  <select
                    value={catalog.session.clipId ?? ''}
                    onChange={(event) => {
                      const value = event.target.value
                      setSessionClip(value === '' ? null : value)
                    }}
                  >
                    <option value="">{t('session.followGlobal')}</option>
                    {clips.map(clip => <option key={clip.id} value={clip.id}>{nameOf(clip)}</option>)}
                  </select>
                </label>
              </>
            )}
      </section>
    </div>
  )
}
