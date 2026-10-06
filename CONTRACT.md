# Contract

`ubu_core` mirrors UbU Phase 1 schema contracts as Rust domain types.

## Stable Decisions

- Public repository under `UbU-project/ubu-core`.
- Crate name: `ubu_core`.
- Version: `0.1.0`.
- License: MIT.
- No cross-repo Rust dependency on other UbU crates.
- No runtime behavior for storage, planning, GitHub APIs, HTTP, UI, Tauri, or GPU execution.
- Canonical planning frame envelopes and engine provenance live in `src/worker/planning_worker.rs`; the kernel composes typed payloads.

## Compatibility

Compatibility is checked by round-tripping canonical fixtures from
`schemas-ref/fixtures` when the submodule is available.

## UniverseState

`src/core/universe_state.rs` holds the `UniverseState`, the mutations that
change it and the preconditions evaluated against it. From P1B-59 a measured
number is a first-class fact, and these are the rules.

**Nine operations.** `set_fact`, `clear_fact`, `set_numeric`, `clear_numeric`,
`increment_numeric`, `decrement_numeric`, `add_membership`, `remove_membership`
and `append_event_marker`. `set_numeric` replaces whatever is there and
`clear_numeric` removes the key; clearing a key that is not there changes
nothing and is not an error. The two clears take no payload and no provenance
kind, because they write nothing. Increment and decrement stay: a tally is a
real thing.

**Seven predicates.** `equals`, `member_of`, `absent`, and four numeric
comparisons: `at_least`, `at_most`, `greater_than` and `less_than`. Each
comparison requires a `numeric_values` target and a finite number as
`expected`, and is `Malformed` otherwise. A number that was never recorded
satisfies none of the four: the answer is false, and asking is not an error.

**Per-fact provenance is a sibling map.** `UniverseState.fact_provenance` maps
a full target, `<collection>.<key>`, to a `FactProvenance`: a `kind` and
`recorded_at`, and nothing else. The kinds are `asserted`, `measured`,
`derived` and `proposed`. There is no confidence number and no free text.
`source_summary` and `confidence_summary` still describe the state as a whole.

The map is not named `provenance`. A `UniverseState` in the store carries the
object envelope under that key, and `ubu-store` rewrites it on each write.

**A mutation carries the provenance of what it writes.** `provenance_kind` is
optional and absent means `asserted`. `apply_universe_mutations` takes the
write time as its third argument and reads no clock, so the same inputs give
the same state. It records the kind and that time for each target it writes.
It does not move `captured_at`.

**No entry outlives its value.** `clear_fact`, `clear_numeric`, and a
`remove_membership` that empties a set, remove the provenance entry with the
value. A `remove_membership` that leaves members rewrites the set and records
its own provenance. One that removes nothing touches nothing.

**A mutation has no `note`.** `UniverseMutation` and `FactProvenance` refuse
unknown fields, as `TaskEffect` does. A stored Task whose effects carry a
`note` on a mutation no longer deserializes.

**A key does not repeat its collection.** A fact is stored under
`operator.work_style` in `facts` and addressed `facts.operator.work_style`.
`is_intrinsic_affect_target` reads the segment after the collection as the
namespace, so a key that began with its collection would hide its namespace
from the mode guard.

**Whole-state fixture coverage, from P1B-60.** The canonical schema now agrees
with this crate: `source_summary` is a required non-empty string and
`confidence_summary` an optional nullable string. The canonical `roundtrip.json`
fixture compares the entire deserialized and serialized UniverseState, including
both summaries and per-fact provenance. Object-valued summaries are refused.
No Rust domain type or runtime behavior changed. Compatibility claims apply only
to types exercised by whole-fixture round trips; other coverage gaps are listed
in the P1B-60 report, not silently fixed here.

## Precondition candidates

P1B-61 adds `CandidateKind::Precondition`, serialized as `precondition`.
It remains candidate state, separate from admitted objects. A whole canonical
fixture round trips check the new kind, its tree-valued normalized proposal,
and the existing/proposed tree pair for an explicit replacement;
unknown candidate kinds remain refused. Schema-specific proposal validation is
the producer/admission boundary’s responsibility; the candidate keeps its
existing JSON-value proposal type. No predicate semantics change.

## P1B-67: UniverseTarget

The handwritten candidate kind adds `UniverseTarget`, serialized as
`universe_target`. Its canonical name-only fixtures are round-tripped whole.
The schemas-ref pointer updates fixture compatibility input, not runtime enum
validation. The existing whole-UniverseState fixture coverage remains.


## P1B-70 frames and provenance

The advice-shaped capacity/recommendation types are retired. PlanningStreamFrame
mirrors the four design outcomes and validates shape, version and request identity.
validate_sequence enforces zero-based increasing indices and one terminal outcome,
including refusing duplicate final responses. EngineProvenance has exactly the
design fields and closed enums, with a separate validate method for string and
GPU-framework constraints. Core names no kernel request/response type and
executes no worker. The approved legacy invocation version stays 0.1. Whole
fixtures exercise every frame, both provenance fixtures, sequences and all
invalid cases; optional provenance fields are tested independently.
