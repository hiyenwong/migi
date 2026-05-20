# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `LogObserver`: log file tailing observer with structured parsing
- `MetricsObserver`: HTTP metrics polling observer with resource threshold detection
- StatisticalLearner anomaly detection with baseline frequency analysis
- TrustManager state persistence to disk
- `ShellInterventionStrategy`: subprocess-based intervention execution
- `HttpInterventionStrategy`: HTTP-based intervention with automatic rollback
- TOML configuration loading with default fallback
- Main event loop integrating all four layers

## [0.1.0] - 2026-05-20

### Added
- Initial project skeleton with 4-layer architecture (Observer, Learner, Intervener, Trust)
- OpenSpec specifications (5 core specs)
- Documentation (plan, philosophy, architecture overview)
