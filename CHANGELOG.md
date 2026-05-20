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

## [0.1.0] - 2026-05-20

### Added
- Initial project skeleton with 4-layer architecture (Observer, Learner, Intervener, Trust)
- OpenSpec specifications (5 core specs)
- Documentation (plan, philosophy, architecture overview)
