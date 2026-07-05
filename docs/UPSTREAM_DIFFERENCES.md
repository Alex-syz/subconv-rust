# Upstream Differences

## Scope

Reference upstream: [SubConv/SubConv](https://github.com/SubConv/SubConv).

This comparison is based on the public upstream repository, this repository's code, and the test suite in this checkout. The upstream project may evolve independently.

## Comparison

| Area | Upstream | This repository | Status |
| --- | --- | --- | --- |
| Backend | Python/FastAPI implementation | Rust/Axum implementation | Changed |
| Web UI | Upstream UI | Independent Vue rewrite | Changed |
| Built-in templates | `zju` and `general` | `meta-rules`, `zju`, and `general` | Added |
| Default template | `zju` | `meta-rules` | Changed |
| Deployment/runtime | Upstream deployment model | Rust binary plus container image workflow | Changed |
| Cache implementation | Upstream cache behavior | Local in-process and disk-backed cache | Changed |
| API compatibility | Public API surface exists upstream | Same main endpoints, with default handling and URL generation adapted to this repo | Compatible |
| Vercel support | Provided upstream | Not provided here | Not provided |

## Inherited behavior

- Subscription parsing.
- Mihomo-compatible YAML output.
- `proxy-providers`-based node handling.
- Rule-provider concepts.
- Template-driven conversion flow.

## Project-specific work

- Rust backend.
- Rewritten frontend.
- `meta-rules` as the default template.
- Local cache implementation.
- Container and image publishing workflow.
- Documentation and release automation in this repository.

## Notes

The goal of this repository is practical compatibility for the supported endpoints, not a claim of byte-for-byte parity with upstream.
