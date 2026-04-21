# Contributing to altertable-lakehouse-rust

Thanks for helping improve the Altertable Lakehouse Rust SDK.

## Workflow

1. Fork the repository
2. Create a feature branch from `main`
3. Add tests for every behavior change
4. Run the full local validation suite
5. Open a pull request against `main`

## Local checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Integration tests run against the Altertable mock server. In CI, the mock runs as a service container on `localhost:15000`. Locally, the test suite starts the mock with Testcontainers unless `CI=true`.
