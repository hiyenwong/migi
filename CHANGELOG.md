# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `LogObserver`: async log file tailing with structured timestamp/severity parsing (15 tests)
- `MetricsObserver`: HTTP metrics polling with resource threshold detection (9 tests)
- Multiple timestamp format support (RFC3339, NaiveDateTime, Unix epoch)
- Case-insensitive log level mapping with sensible defaults
- Automatic ResourceAlert generation when CPU/memory exceeds threshold
- StatisticalLearner anomaly detection with Z-score baseline frequency analysis (8 tests)
- System topology inference — auto-discovers subsystems from event patterns
- BaselineStats with sliding window for progressive anomaly detection
- TrustManager state persistence to disk (atomic write via temp+rename)
- Trust blacklist (blocked_targets) alongside whitelist
- ShellInterventionStrategy: async subprocess execution with history tracking (6 tests)
- HttpInterventionStrategy: HTTP-based intervention with error handling (1 test)
- Action.needs_rollback() for intelligent rollback decisions
- 19 new tests for Trust (persistence, transitions, authorization, blacklist)
- 9 new tests for Intervener (shell commands, HTTP, rollback, strategies)
- TOML configuration loading with default fallback (4 tests)
- Main event loop integrating all four layers (Phase 6)
- Encrypted secrets management with AES-256-GCM (6 tests)
- `migi-secrets` CLI for managing encrypted API keys and config
- TOML config with optional LLM section for provider/model/endpoint
- Sandbox simulation system with 4 scenarios (7 tests)
- `migi-sim` CLI for running simulations: baseline, anomaly, transition, lifecycle
- Updated design philosophy with 4-stage ultimate goal (adaptive learning → host education → graceful dormancy)

## [0.1.0] - 2026-05-20

### Added
- Initial project skeleton with 4-layer architecture (Observer, Learner, Intervener, Trust)
- OpenSpec specifications (5 core specs)
- Documentation (plan, philosophy, architecture overview)
