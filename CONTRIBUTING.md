# Contributing

Run these checks before opening a pull request:

```sh
cargo fmt --check
cargo clippy --all-targets
cargo test
```

Keep changes focused on Phase 1 shared domain contracts. Do not add database,
planner, network service, UI, Tauri, GitHub API, or GPU runtime behavior to this
crate.
