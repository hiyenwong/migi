# Migi — CLAUDE.md

## Project: Migi (右)
A symbiotic AI agent in Rust, inspired by Parasyte's Migi. Parasitic, not takeover.

## Architecture (ONE-WAY dependency, NEVER reverse)
```
config → error → observer → learner → intervener → trust → main
```

## Quality Gates (MUST pass before every commit)
```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Conventions
- **Error types**: Use `MigiError` + `MigiResult`, propagate with `?`
- **Logging**: Use `tracing` crate with structured fields
- **Testing**: Unit tests in same file, `#[cfg(test)]` module
- **File size**: Max 300 lines per file, split into submodules if exceeded
- **async**: Use `#[async_trait]` for traits, `tokio` runtime
- **Serialization**: All public structs must derive `Serialize + Deserialize`

## OpenSpec (ALL features must match spec scenarios)
| Spec | Path |
|------|------|
| Symbiosis Core | `openspec/core/symbiosis.spec.md` |
| Observation | `openspec/core/observation.spec.md` |
| Learning | `openspec/core/learning.spec.md` |
| Intervention | `openspec/core/intervention.spec.md` |
| Trust | `openspec/core/trust.spec.md` |

## Implementation Order
1. **Phase 2**: LogObserver + MetricsObserver (感知层)
2. **Phase 3**: StatisticalLearner anomaly detection (认知层)
3. **Phase 4**: TrustManager persistence + full transition logic (信任层)
4. **Phase 5**: ShellInterventionStrategy + HttpInterventionStrategy (行动层)
5. **Phase 6**: Event loop + TOML config loading + main integration
6. **Phase 7**: README polish + Docker + CI/CD

## Git Rules
- Every feature gets its own commit
- Conventional commits: `feat:`, `fix:`, `test:`, `docs:`, `refactor:`
- Always write a CHANGELOG entry for each commit
- Keep commits atomic — one feature per commit
