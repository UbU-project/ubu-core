# ubu-core

`ubu_core` is the shared Rust domain foundation for UbU Phase 1.

It contains hand-written Rust types compatible with canonical schemas from
`UbU-project/ubu-schemas`, common prefixed ID handling, timestamp parsing,
provenance helpers, compartment labels, policy summaries, the closed
`AuthoritySource` enum, and the canonical GPU advisory wire types.

This repository intentionally does not contain a database, GitHub API client,
planner, HTTP server, UI, Tauri application, or GPU runtime behavior.

## Schema Fixtures

Canonical fixtures are read from the `schemas-ref/` git submodule:

```sh
git submodule update --init --recursive
```

The submodule is pinned manually:

```sh
cd schemas-ref
git checkout <rev>
cd ..
git add schemas-ref
```

`build.rs` exposes the fixture directory to tests through
`UBU_SCHEMAS_FIXTURES`. It never fetches from the network. If the submodule is
absent during an isolated scaffold experiment, tests fall back to explicitly
named placeholder fixtures under `fixtures/placeholders` and Cargo emits a
warning with a TODO.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets
cargo test
```
