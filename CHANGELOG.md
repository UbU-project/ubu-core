# Changelog

## Unreleased

- Whole-candidate fixture coverage includes replacement preconditions and preserves both trees.

- P1B-61 B: Add the precondition candidate kind, advance schemas-ref, and test whole canonical candidate round trips and unknown-kind refusal.

- P1B-60 B: Pin the corrected UniverseState summary schemas. Replace the known-drift assertion with a whole canonical fixture round trip and reject object-valued summaries. Domain types and runtime behavior are unchanged.

- P1B-59 B: A measured number is a first-class fact.
  - **Breaking:** `apply_universe_mutations` takes a third argument, the write time. `UniverseMutation` loses `note` and gains optional `provenance_kind`, and refuses unknown fields. `UniverseState` gains `fact_provenance`.
  - Two operations: `set_numeric` replaces a number and `clear_numeric` removes it. A number could previously only be moved by a difference from whatever was there, and never removed.
  - Four predicates: `at_least`, `at_most`, `greater_than` and `less_than`, on `numeric_values` targets with a finite numeric `expected`. An absent key is false, not an error.
  - Per-fact provenance: `fact_provenance` maps a full target to a `kind` (`asserted`, `measured`, `derived`, `proposed`) and `recorded_at`. A mutation with no stated kind records `asserted`. An entry is removed with its value.
  - `schemas-ref` moves to `dadb504`, and the fixture-compatibility tests cover the mutation, precondition and provenance fixtures.
  - **A mode-validation hole, stated here because the fix is in the caller.** `is_intrinsic_affect_target` reads the second dotted segment of a target as the namespace. `ubu-orchestrator`'s bootstrap stored its keys with the collection already in them, `facts.operator.work_style` inside `facts`, so the target for such a key was `facts.facts.operator.work_style` and its namespace read as `facts`. Under that convention an intrinsic-affect fact would be targeted `facts.facts.affect.x`, and `validate_mutations_for_mode` and `validate_precondition_for_mode` would not fire for it in `organization_mode` or `worker_mode`. Nothing in this crate changed to close it: the function is right for a key that does not repeat its collection, and a test now pins what it does for one that does. The orchestrator stops writing doubled keys in the same ticket.

## 0.1.0

- Initial Phase 1 Rust domain foundation scaffold.

## P1B-67

Add UniverseTarget to the handwritten advisory vocabulary and compatibility tests.
