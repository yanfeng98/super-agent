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
import type { ClipInfo, FitMode, PlayTiming } from '../types.ts'
import type { SettingsSectionProps } from './contract.ts'
import css from './SettingsSection.module.css'

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
    t, useCatalog, useSettings, useSession, refresh, preview, setEnabled, setTiming,
    setFit, setRandom, setSelectedClip, setSessionReplay, setSessionClip, sessionStorageAvailable,
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
