# Bootstrap Notes

## Impact analysis

- Public API to provide: `append`, `query`, `query_all`, `upload`, `get_query`, `cancel_query`, `validate`.
- Tests needed across request construction, auth resolution/redaction, streamed NDJSON parsing, error mapping, and endpoint happy/failure paths.
- Docs affected: `README.md`, `CHANGELOG.md`, cargo metadata, examples.
- Cross-SDK check target: compare with another lakehouse SDK before PR finalization.

## Current blocker

- The initial dependency setup pulled `reqwest` default TLS features, which brought in `native-tls`/`openssl` and failed to link in this environment because `ld.lld` could not load `libxml2.so.2`.
- Switched `reqwest` to `default-features = false` with `rustls-tls` only. Validation needs to be re-run after implementation exists.
