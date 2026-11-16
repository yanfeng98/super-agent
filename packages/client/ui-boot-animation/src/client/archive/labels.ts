/**
 * Localized chrome for the archive stage.
 *
 * The five authorization lines are fixed art (see `script.ts`); everything around
 * them is interface copy and comes from the dictionary.
 * @module @deepseek-ai/dsh-client-ui-boot-animation/src/client/archive/labels
 */
import type { BootAnimationLocaleKey } from '../locales.ts'
import type { ArchiveLabels } from './ArchiveStage.tsx'

/** The translate seat this package's dictionary provides. */
export type Translate = (key: BootAnimationLocaleKey, params?: Record<string, unknown>) => string

/**
 * Build the stage's chrome labels.
 * @param t - the bound dictionary.
 * @returns the labels the stage renders.
 */
export function archiveLabels(t: Translate): ArchiveLabels {
  return {
    skip: t('archive.skip'),
    hint: t('archive.hint'),
    cardName: t('archive.card.name'),
    cardId: t('archive.card.id'),
    cardLevel: t('archive.card.level'),
    welcome: name => t('archive.welcome', { name }),
    idLine: id => t('archive.idLine', { id }),
    selfCheck: { zh: t('archive.selfCheck.zh'), en: t('archive.selfCheck.en') },
    finale: { zh: t('archive.finale.zh'), en: t('archive.finale.en') },
    revision: t('archive.revision'),
    logoMark: t('archive.logo'),
    welcomeTo: t('archive.welcome.to'),
    welcomeMark: t('archive.welcome.mark'),
  }
}
