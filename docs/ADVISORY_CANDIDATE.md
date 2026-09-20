# AdvisoryCandidate: P1B-4 choices and boundaries

- `AdvisoryCandidateId` belongs to candidate state, outside the admitted-object
  registry, following the ticket's explicit identity placement. Parsing is strict:
  `advcand_` and 32 lowercase UUIDv7 hex digits, including the RFC variant. The
  existing DeviceId parser still accepts any non-empty registered name.
- Retention uses `retain` and `purge_payload`: preserve under applicable policy,
  or allow payload removal while keeping durable correction metadata. Disclosure
  uses `compartment_only` and `redacted_only`: two review restrictions, neither an
  export grant. All five requested vocabularies are non-exhaustive Rust enums.
- The transition function is the complete lifecycle authority in this type-only
  ticket: exactly 17 directed edges, no self-edges, and all 49 state pairs tested.
  `Deferred` cannot become `Admitted` without first resurfacing. Every transition
  into `Resurfaced` requires a trigger; all other transitions forbid a trigger.
  Loading a historical record does not replay its history or force it to Proposed;
  `INITIAL` defines the starting state for new proposals.
- The candidate retains every top-level field in the ticket sketch. Structured
  fields remain nested: payload and review label use explicit `kind`/`value` tags;
  the unit redacted label is `{"kind":"redacted"}`. Optional fields are omitted
  when absent. Maps and sets serialize in sorted order, and duplicate Compartment
  IDs are rejected during deserialization.
- `ProposingActor` groups model/tool name, version, and an optional prompt/template
  digest; it is execution provenance, not the deciding actor's authority.
- `CandidateLinks` uses opaque string references because decision event types are
  out of scope. It includes deferral, prior deferral, resurfacing, supersession,
  admission, archive, correction, and rejection references, plus the trigger,
  evidence refs, and resurfacing reason. As explicitly requested by task C, only
  a non-empty prior-deferral reference and trigger are mandatory for Resurfaced;
  evidence and reason are representable, without invented additional validation.
- `validate()` also runs on deserialization. Confidence rejects non-finite values
  as well as values outside [0, 1]. Schema version means non-empty, with no invented
  version grammar. Compartment entries are parseable `UbuId`s as specified, without
  imposing a Compartment-only prefix. Version is bounded by Rust's `u64`.
- Suppression's `target_and_scope_shape` copies the sketch's joint target-or-scope
  typed refs. `compartment_ids` plus the tagged `review_label` represent Compartment
  and redaction class. `proposing_actor` groups extractor/model version and optional
  prompt/template digest; `schema_version` is copied from the candidate. These
  choices retain all UBU-D0274 suppression fields without a rejected payload.
- `SuppressionDecision` groups the builder's deciding Identity, authority, time,
  reason/correction, retention policy, and evidence hashes/source fingerprints.
  Fingerprints are explicitly supplied, never inferred from evidence references.
  The builder requires Rejected and a non-empty existing suppression key, and
  validates the Identity prefix. It does not mint a suppression key, mutate the
  candidate, redact content, or implement matching/retention. Reason and schema
  version are also required non-empty. The suppression prohibition is documented
  on the type and in its schema; reader enforcement belongs to later tickets.
- The shared DeviceId, IdempotencyKey, and ExecutionContext are reused. The legacy
  CandidateObject, admission registry, schema submodule pin, dependencies, store,
  and orchestrator are unchanged. P1B-5 still owns the candidate storage correction.
- Schema validation covers both valid and invalid fixture trees in ubu-schemas.
  Core copies all eight fixtures byte-for-byte into its fallback placeholder tree;
  its existing resolver and pinned schema submodule remain unchanged. Five valid
  fixtures additionally match pretty-serialized Rust output byte-for-byte.
