# Contract

`ubu_core` mirrors UbU Phase 1 schema contracts as Rust domain types.

## Stable Decisions

- Public repository under `UbU-project/ubu-core`.
- Crate name: `ubu_core`.
- Version: `0.1.0`.
- License: MIT.
- No cross-repo Rust dependency on other UbU crates.
- No runtime behavior for storage, planning, GitHub APIs, HTTP, UI, Tauri, or GPU execution.
- Canonical GPU advisory wire types live in `src/worker/gpu_advisory.rs`.

## Compatibility

Compatibility is checked by round-tripping canonical fixtures from
`schemas-ref/fixtures` when the submodule is available.
