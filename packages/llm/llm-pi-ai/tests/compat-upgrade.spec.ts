import { describe, expect, it } from 'vitest'
import { Config, resolveProfiles } from '../src/config.ts'
import { THINKING_TOKEN_BUDGET_FIELDS } from '../src/catalog.ts'

function configured(compat: Record<string, unknown>, api = 'openai-completions'): Config {
  return Config({ providers: { gateway: {
    api, baseURL: 'https://gateway.test/v1', models: [{ id: 'model' }], compat,
  } } })
}

function resolved(compat: Record<string, unknown>, api = 'openai-completions') {
  return resolveProfiles(structuredClone(configured(compat, api).providers.get()) as import('../src/config.ts').Options['providers']).get('gateway')?.piProvider?.getModels()[0]?.compat
}

describe('pi-ai gateway compatibility declarations', () => {
  it.each(THINKING_TOKEN_BUDGET_FIELDS)('preserves reasoning-budget field %s', (field) => {
    expect(resolved({ thinkingTokenBudgetField: field })).toMatchObject({ thinkingTokenBudgetField: field })
  })

  it.each([-2, 0, 2])('preserves scheduler priority %s', (priority) => {
    expect(resolved({ vllmPriority: priority })).toMatchObject({ vllmPriority: priority })
  })

  it('preserves a disabled Responses output-cap parameter', () => {
    expect(resolved({ supportsMaxOutputTokens: false }, 'openai-responses'))
      .toMatchObject({ supportsMaxOutputTokens: false })
  })

  it.each(['chatTemplateKwargs', 'chatTemplateArgs'])('accepts thinking.budget in %s', (field) => {
    const value = { budget: { $var: 'thinking.budget' } }
    expect(resolved({ [field]: value })).toMatchObject({ [field]: value })
  })

  it('accepts an explicit off-omission flag on a template value', () => {
    const value = { budget: { $var: 'thinking.budget', omitWhenOff: true } }
    expect(resolved({ chatTemplateKwargs: value })).toMatchObject({ chatTemplateKwargs: value })
  })

  it.each(['requiresNonNullAssistantContent', 'requiresToolsWithToolHistory'])('preserves %s', (field) => {
    expect(resolved({ [field]: true })).toMatchObject({ [field]: true })
  })

  it.each(['auto', 'enabled', 'disabled'])('preserves prompt cache key mode %s', (mode) => {
    expect(resolved({ promptCacheKeyMode: mode })).toMatchObject({ promptCacheKeyMode: mode })
    expect(resolved({ promptCacheKeyMode: mode }, 'openai-responses')).toMatchObject({ promptCacheKeyMode: mode })
  })

  it('preserves an Anthropic message-thread switch', () => {
    expect(resolved({ supportsMessageThreads: false }, 'anthropic-messages'))
      .toMatchObject({ supportsMessageThreads: false })
  })

  it.each([
    { thinkingTokenBudgetField: 'unknown' },
    { vllmPriority: 'high' },
    { vllmPriority: 0.5 },
    { supportsMaxOutputTokens: 'false' },
    { promptCacheKeyMode: 'sometimes' },
    { chatTemplateKwargs: { budget: { $var: 'thinking.unknown' } } },
    { chatTemplateArgs: { budget: { $var: 'thinking.unknown' } } },
  ])('rejects invalid schema values: %j', (compat) => {
    expect(() => configured(compat)).toThrow()
  })

  it.each([
    { thinkingTokenBudgetField: 'thinking_budget' },
    { vllmPriority: 0 },
    { supportsMaxOutputTokens: false },
    { requiresNonNullAssistantContent: true },
  ])('rejects fields on a protocol that cannot consume them: %j', (compat) => {
    expect(() => resolved(compat, 'anthropic-messages')).toThrow(/compat/)
  })

  it.each([
    'supportsMidConvoEffort', 'allowedFallbackModels', 'supportsMidConvoSystemMessages',
    'supportsMidConvoToolAdditions', 'supportsMidConvoToolChanges', 'sessionAffinityFormat',
  ])('withholds catalog-owned %s', (field) => {
    expect(() => resolved({ [field]: true }, 'anthropic-messages'))
      .toThrow(/which is not configurable here/)
  })

  it('rejects an Anthropic-only switch on an OpenAI-compatible route', () => {
    expect(() => resolved({ supportsMessageThreads: true })).toThrow(/compat/)
  })

  it('refuses a declared route on the withheld Mistral protocol', () => {
    // `mistral` is withheld and `mistral-conversations` is not one of the four
    // protocols a hand-declared route may name, so the compat fields its gate
    // withholds have no route left to configure.
    expect(() => configured({ supportsMidConvoSystemMessages: true }, 'mistral-conversations'))
      .toThrow(/mistral-conversations/)
  })

  it('keeps generic additions absent unless configured', () => {
    expect(resolved({})).toBeUndefined()
  })
})
