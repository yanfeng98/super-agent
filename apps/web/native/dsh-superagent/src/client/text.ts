/** Localization, reason formatting, message marker cleanup, and relative timestamp helpers. */
import type { Translate } from '@deepseek-ai/dsh-client-ui-slots'
import type { DshSuperagentKey } from './locales.ts'
import type { Localized } from '../sources/kind.ts'
import type { SetupProblem } from '../setup.ts'
import type { Health } from '../sources/host.ts'
import { reasonText, type Reason } from '../core/reasons.ts'

export type T = Translate<DshSuperagentKey>
export const errorText = (failure: unknown): string => failure instanceof Error ? failure.message : String(failure)
export const localized = (t: T, text: Localized | string | undefined): string =>
  typeof text === 'string' ? text : text ? text[t('lang') === 'en' ? 'en' : 'zh'] : ''
export const actorName = (t: T, actor: { name?: Localized } | undefined): string => actor?.name ? localized(t, actor.name)
  : t('automatedNotice')
export function valueLabel(t: T, group: string, key: string | undefined): string {
  if (!key) return ''
  const name = `${group}.${key}` as DshSuperagentKey
  const text = t(name)
  return text === name ? key : text
}
/** Format one or more structured or raw reasons in the active UI language, truncating multi-line host errors to their first line. */
export function reasonLabel(t: T, reason: string | Reason | readonly Reason[] | undefined): string {
  if (typeof reason === 'string') return reason ? label(t, { code: reason }) : ''
  return [reason ?? []].flat().map(item => label(t, item)).filter(Boolean).join('；')
}
function label(t: T, reason: Reason): string {
  const key = `reason.${reason.code}` as DshSuperagentKey
  const text = reason.code === 'said' ? reason.values?.text ?? '' : t(key, reason.values)
  if (reason.code !== 'said' && text !== key) return text
  const written = reason.code === 'said' ? text : reasonText({ ...reason, triggerId: undefined })
  if (/^\w*Error\b/.test(written)) { const first = written.split('\n')[0]!; return first.length > 140 ? `${first.slice(0, 140)}…` : first }
  return written
}
export const setupText = (t: T, problem: Pick<SetupProblem, 'code' | 'values'>): string => t(`setup.${problem.code}`, problem.values)
export const healthText = (t: T, health: Pick<Health, 'detail' | 'code' | 'values'>): string =>
  health.code ? t(`healthCode.${health.code}` as const, health.values) : health.detail
/**
 * A ticket ending's time, outcome and reason in the panel's language.
 * @param t - Translator of the active panel language.
 * @param ended - The recorded ending, or undefined for an open Ticket.
 * @returns The ending sentence, or an empty string without an ending.
 */
export function endedLabel(t: T, ended: { at: string; outcome: string; reason?: string } | undefined): string {
  if (!ended) return ''
  const outcome = valueLabel(t, 'outcome', ended.outcome), reason = reasonLabel(t, ended.reason), time = when(ended.at, t('lang'))
  return reason ? t('endedAt', { time, outcome, reason }) : t('endedAtShort', { time, outcome })
}
/**
 * One notification's detail sentence in the panel's language: an ending's outcome and reason, or where a merge landed.
 * Records without those fields show the sentence they were written with.
 * @param t - Translator of the active panel language.
 * @param item - The stored notification.
 * @returns The detail sentence to show under its title.
 */
export function notificationLabel(t: T, item: { text: string; ending?: { outcome: string; reason?: string }
  merge?: { target: string; head?: string } }): string {
  if (item.ending) {
    const outcome = valueLabel(t, 'outcome', item.ending.outcome)
    return item.ending.reason ? `${outcome}：${item.ending.reason}` : outcome
  }
  if (item.merge) return item.merge.head
    ? t('notificationMergedCommit', { target: item.merge.target, commit: item.merge.head.slice(0, 8) })
    : t('notificationMerged', { target: item.merge.target })
  return item.text
}
/** Replaces Feishu content placeholders (images, cards, media, forwards, unresolved mentions) with localized labels. */
export function readableMarkers(t: T, text: string): string {
  return text.replace(/!\[[^\]]*\]\([^)]*\)|\[Image: [^\]]*\]|\[image\]/g, t('imageMarker'))
    .replace(/\[Chat card: [^\]]*\]/g, t('cardMarker'))
    .replace(/\[Media: [^\]]*\]/g, t('mediaMarker')).replace(/\[Merged forward\]/g, t('forwardMarker'))
    .replace(/@_user_\d+/g, t('unknownMention'))
}
const timeLocale = (lang: string) => lang === 'en' ? 'en' : 'zh-CN'
export function when(value: string | undefined, lang: string): string {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  const seconds = Math.round((date.getTime() - Date.now()) / 1000)
  if (Math.abs(seconds) < 86_400) {
    const relative = new Intl.RelativeTimeFormat(timeLocale(lang), { numeric: 'auto' })
    if (Math.abs(seconds) < 60) return relative.format(0, 'second')
    if (Math.abs(seconds) < 3600) return relative.format(Math.round(seconds / 60), 'minute')
    return relative.format(Math.round(seconds / 3600), 'hour')
  }
  const sameYear = date.getFullYear() === new Date().getFullYear()
  const year = sameYear ? {} : { year: 'numeric' } as const
  return date.toLocaleString(timeLocale(lang), { ...year, month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}
export function fullTime(value: string | undefined, lang: string): string {
  const date = new Date(value ?? '')
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleString(timeLocale(lang))
}

/** A ticket's stable display number, such as `T-0001`; empty when it has none. */
export const ticketNumber = (number: number | undefined): string => number ? `T-${String(number).padStart(4, '0')}` : ''
