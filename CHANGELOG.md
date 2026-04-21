# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0](https://github.com/altertable-ai/altertable-lakehouse-rust/compare/altertable-lakehouse-v0.1.0...altertable-lakehouse-v0.2.0) (2026-04-21)


### Features

* **rust:** bootstrap lakehouse sdk ([#1](https://github.com/altertable-ai/altertable-lakehouse-rust/issues/1)) ([d458dc9](https://github.com/altertable-ai/altertable-lakehouse-rust/commit/d458dc900cdadfc71200284e4e7beec2144969ca))


### Bug Fixes

* 1 ([#4](https://github.com/altertable-ai/altertable-lakehouse-rust/issues/4)) ([3942b6f](https://github.com/altertable-ai/altertable-lakehouse-rust/commit/3942b6fe8133607fb0a639dd4e85f29f779b7187))

## [0.1.0] - 2026-04-21

- Initial Rust SDK bootstrap for the Altertable Lakehouse API.
- Added typed client support for append, query, query_all, upload, get_query, cancel_query, and validate.
- Added reqwest-based transport, typed errors, CI workflows, and Testcontainers-backed mock integration tests.
