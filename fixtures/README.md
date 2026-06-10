# Fixtures

Canonical test fixtures live in the `schemas-ref/` git submodule, which points
to `UbU-project/ubu-schemas`.

The `fixtures/placeholders` directory contains explicitly named local fallback
fixtures for isolated scaffold experiments where the submodule is intentionally
absent. Placeholder fixtures are not canonical and must not replace submodule
fixture tests in CI.
