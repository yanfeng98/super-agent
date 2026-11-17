/**
 * Browser-safe failure vocabulary of the configuration surfaces this package
 * serves. The redacted views themselves live with their seam in
 * `@deepseek-ai/dsh-settings/types`, whose Cordis event declarations already
 * register that file for the Client compilation face.
 *
 * @module @deepseek-ai/dsh-api-settings-controller/types
 */

declare module '@deepseek-ai/dsh-typert-protocol' {
  interface RemoteErrorDetailsMap {
    /**
     * Every seam refusal that is not a stale write: an unregistered or malformed
     * namespace, a read-only provider, schema validation, storage.
     */
    'settings/rejected': { readonly ns: string }
    /**
     * The stored revision moved after the caller read it. Its own outcome rather
     * than an invalid request: the caller must re-read and re-apply.
     */
    'settings/conflict': { readonly ns: string; readonly expected: number; readonly actual: number }
    /**
     * The provider refused a valid credential write, for example because the
     * stored document no longer parses. The details name only the reference,
     * never the value.
     */
    'credential/rejected': { readonly ref: string }
  }
}

/**
 * Declared failures of the `authorization` namespace. Every refusal names the
 * credential key the caller addressed and never a value it typed.
 */
declare module '@deepseek-ai/dsh-typert-protocol' {
  interface RemoteErrorDetailsMap {
    /** No registered flow writes this credential, or this composition mounts none. */
    'authorization/unknown-flow': { readonly key: string }
    /** The flow is registered but does not offer the named method. */
    'authorization/unknown-method': { readonly key: string; readonly method: string }
    /** An attempt for this key is already running; a page watches or cancels it through `state`. */
    'authorization/in-flight': { readonly key: string }
    /** The key has no attempt since this controller mounted. */
    'authorization/no-attempt': { readonly key: string }
    /** The attempt is not waiting on that question: it was answered, replaced, or withdrawn. */
    'authorization/no-prompt': { readonly key: string; readonly promptId: number }
  }
}

/** Confirmation that the settings document was handed to the native editor. */
export interface SettingsDocumentOpenValue {
  readonly opened: true
}

/** One way a registered flow obtains its credential. */
export interface AuthorizationMethodView {
  /** Flow-owned identifier, echoed back to `begin`. */
  readonly id: string
  /** User-facing label for a picker. */
  readonly label: string
}

/** One registered flow, as a configuration page lists it. */
export interface AuthorizationFlowView {
  /** Joined `<scope>/<id>` credential key the flow writes. */
  readonly key: string
  /** Owning plugin's name, the key's first segment. */
  readonly scope: string
  /** That plugin's own addressing unit, the key's second segment. */
  readonly id: string
  /** User-facing name of what is being authorized. */
  readonly label: string
  /** The methods this flow offers, most preferred first. */
  readonly methods: readonly AuthorizationMethodView[]
  /** Whether an attempt for this key is running right now. */
  readonly inFlight: boolean
}

/** A running flow's report to the page watching it. Never carries a secret. */
export interface AuthorizationNoticeView {
  /** What is happening, or what the human must do next. */
  readonly message: string
  /** A page the human must open to continue. */
  readonly url?: string
  /** A short code the human must enter on that page. */
  readonly code?: string
}

/** One choice offered by a `select` question. */
export interface AuthorizationPromptOptionView {
  /** Value returned when this option is chosen. */
  readonly id: string
  /** User-facing label. */
  readonly label: string
  /** Optional extra context. */
  readonly description?: string
}

/**
 * A question the flow needs answered. `secret` differs from `text` only in
 * presentation — a page masks it and keeps it out of logs — and `select`
 * answers with the chosen option's id.
 */
export type AuthorizationPromptView = {
  /** Id echoed back with the answer; a replaced question gets a new one. */
  readonly promptId: number
  /** What to ask. */
  readonly message: string
} & (
  | { readonly kind: 'text'; readonly placeholder?: string }
  | { readonly kind: 'secret'; readonly placeholder?: string }
  | { readonly kind: 'select'; readonly options: readonly AuthorizationPromptOptionView[] }
)

/** How one attempt ended, or that it has not ended yet. */
export type AuthorizationPhase = 'running' | 'authorized' | 'cancelled' | 'failed'

/** One attempt's state at the moment it was read. */
export interface AuthorizationAttemptView {
  /** Joined `<scope>/<id>` credential key the attempt authorizes. */
  readonly key: string
  /** Where the attempt stands. */
  readonly phase: AuthorizationPhase
  /** Everything the flow has reported, oldest first, bounded by the controller. */
  readonly notices: readonly AuthorizationNoticeView[]
  /** The question the attempt is waiting on, or null when it is not waiting. */
  readonly prompt: AuthorizationPromptView | null
  /** What went wrong, on `failed` alone. */
  readonly error?: string
}
