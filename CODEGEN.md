# Code Generation

`ubu_core` types are hand-written Rust types. They are not generated from JSON
Schema.

Canonical JSON Schema remains in `UbU-project/ubu-schemas`. This crate uses the
`schemas-ref/` submodule only for fixture compatibility tests.

Generated code may be considered later only through an explicit contract change.
