<div align="center">
  <h1>🦁 Migi (右)</h1>
  <p><em>"寄生而不接管" — A Symbiotic AI Agent in Rust</em></p>

  <p>
    <a href="#-architecture">Architecture</a> •
    <a href="#-phases">Phases</a> •
    <a href="#-quick-start">Quick Start</a> •
    <a href="#-testing">Testing</a> •
    <a href="#-design-philosophy">Philosophy</a>
  </p>

  <p>
    <img src="https://img.shields.io/badge/language-Rust-EF5733?style=flat-square&logo=rust" />
    <img src="https://img.shields.io/badge/runtime-tokio-00ADD8?style=flat-square" />
    <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" />
    <img src="https://img.shields.io/badge/tests-63_passing-brightgreen?style=flat-square" />
  </p>
</div>

---

**Migi** (named after Migi/右 from *Parasyte*) is a **symbiotic AI agent** that parasitically integrates into existing systems — observing, learning, and selectively intervening when needed.

Unlike traditional agents that assume full control upfront, Migi **earns trust over time** through a gradual phase transition system:

```
Observation → Assistance → LocalTakeover → ControlledTransition
```

## ✨ Key Features

| Layer | Feature | Status |
|-------|---------|--------|
| 👁️ **Observer** | Async log file tailing with structured parsing | ✅ |
| 👁️ **Observer** | HTTP metrics polling with threshold alerts | ✅ |
| 🧠 **Learner** | Event frequency analysis and statistics | ✅ |
| 🧠 **Learner** | Z-score anomaly detection with sliding windows | ✅ |
| 🧠 **Learner** | Automatic subsystem topology discovery | ✅ |
| 🛡️ **Trust** | Phase-based permission control (4 phases) | ✅ |
| 🛡️ **Trust** | Whitelist + blacklist target management | ✅ |
| 🛡️ **Trust** | State persistence (atomic write) | ✅ |
| 🛡️ **Trust** | Automatic phase transition evaluation | ✅ |
| ⚡ **Intervener** | Shell command execution strategy | ✅ |
| ⚡ **Intervener** | HTTP API call strategy | ✅ |
| ⚡ **Intervener** | Rollback capability for write actions | ✅ |
| ⚙️ **Config** | TOML configuration with default fallback | ✅ |
| 🔄 **Event Loop** | observe → learn → predict → intervene cycle | ✅ |

## 🏗️ Architecture

```
┌──────────────────────────────────────────────────┐
│                   Host System                     │
└──────────────────────┬───────────────────────────┘
                       │ side-channel (read-only)
                       ▼
┌──────────────────────────────────────────────────┐
│                   Migi Agent                      │
│                                                    │
│  ┌──────────┐    events    ┌──────────┐            │
│  │ Observer  │ ─────────▶  │ Learner  │            │
│  │ (感知层)  │             │ (认知层)  │            │
│  └──────────┘              └────┬─────┘            │
│                                 │ predictions       │
│                                 ▼                   │
│                        ┌──────────────┐             │
│                        │  Intervener   │             │
│                        │   (行动层)    │             │
│                        └──────┬───────┘             │
│                               │ approve/reject      │
│                               ▼                     │
│                        ┌──────────────┐             │
│                        │    Trust     │             │
│                        │   (信任层)    │             │
│                        └──────────────┘             │
└──────────────────────────────────────────────────────┘
```

### Layer Dependencies (ONE-WAY, never reverse)

```
config → error → observer → learner → intervener → trust → main
```

## 🌀 Phases

Migi's capabilities grow through four symbiotic phases:

| Phase | Read | Suggest | Write (Isolated) | Emergency Block |
|-------|------|---------|-------------------|-----------------|
| **Observation** | ✅ | ❌ | ❌ | ❌ |
| **Assistance** | ✅ | ✅ | ❌ | ❌ |
| **LocalTakeover** | ✅ | ✅ | ✅ | ❌ |
| **ControlledTransition** | ✅ | ✅ | ✅ | ✅ |

### Phase Transition Rules

| From | To | Requirements |
|------|----|-------------|
| Observation | Assistance | 100+ events observed, model accuracy ≥ (1.0 - threshold) |
| Assistance | LocalTakeover | Trust ≥ 0.8, 10 consecutive successes, model reliable |
| LocalTakeover | ControlledTransition | Trust ≥ 0.8, 10 consecutive successes, accuracy ≥ 0.95 |

> Failures reduce trust score but **never** downgrade the phase.

## 🚀 Quick Start

```bash
# Clone & build
cd /path/to/migi
cargo build --release

# Create config
cp config/migi.toml.example config/migi.toml
# Edit config/migi.toml to match your environment

# Run (adjust RUST_LOG for verbosity)
RUST_LOG=info cargo run
```

### Configuration

```toml
name = "migi"
phase = "observation"                   # observation | assistance | local_takeover | controlled_transition

# 宿主观察端点（日志文件路径、metrics URL等）
host_observation_endpoints = ["/var/log/syslog"]

# 允许介入的目标子系统（空 = 全部禁止）
allowed_intervention_targets = ["database", "cache"]

# 信任阈值：模型误差低于此值时考虑相变
trust_threshold = 0.05

# 最大并发介入数
max_concurrent_interventions = 1
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level filter (`trace`, `debug`, `info`, `warn`, `error`) |

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Quality gate (CI-style)
cargo fmt && cargo clippy -- -D warnings && cargo test
```

**Current coverage:** 63 tests across all layers.

| Module | Tests | Coverage |
|--------|-------|----------|
| `observer/log_observer` | 15 | Timestamp parsing, severity mapping, edge cases |
| `observer/metrics_observer` | 9 | JSON parsing, threshold detection, error handling |
| `learner` | 8 | Model updates, predictions, anomaly detection, subsystems |
| `trust` | 19 | Authorization, phase transitions, persistence, blacklist |
| `intervener` | 8 | Shell commands, HTTP, rollback flags, strategy registration |
| `config` | 4 | Default config, TOML loading, fallback |

## 📖 Design Philosophy

> *"不要试图制造后藤，而要培养一个小右，然后观察它何时准备好进化。"*

Inspired by the manga/anime *Parasyte* (寄生獣), Migi's design mirrors the relationship between Shinichi Izumi and Migi (right hand parasite):

1. **Progressive Trust** — Permission is earned, not given
2. **Failure Isolation** — Agent malfunction doesn't affect host core
3. **All Actions Reversible** — Every intervention has an undo path
4. **Phase Transition Requires Validation** — Model accuracy + trust score
5. **OpenSpec Driven** — All features start as specifications

## 📦 Project Structure

```
migi/
├── Cargo.toml          # Package manifest
├── CLAUDE.md           # Agent context for Claude Code
├── CHANGELOG.md        # History of changes
├── README.md           # This file
├── config/
│   └── migi.toml       # Configuration template
├── docs/
│   ├── plan.md         # Development roadmap
│   ├── philosophy.md   # Parasyte-inspired design philosophy
│   └── architecture/
│       └── overview.md # Architecture documentation
├── openspec/
│   ├── README.md       # Spec index
│   └── core/
│       ├── symbiosis.spec.md      # Core system spec
│       ├── observation.spec.md    # Observer layer spec
│       ├── learning.spec.md       # Learner layer spec
│       ├── intervention.spec.md   # Intervener layer spec
│       └── trust.spec.md          # Trust layer spec
└── src/
    ├── main.rs         # Entry point + event loop
    ├── lib.rs          # Module declarations
    ├── config.rs       # Configuration (TOML)
    ├── error.rs        # Unified error types
    ├── observer.rs     # Observer trait + types
    ├── observer/
    │   ├── log_observer.rs
    │   └── metrics_observer.rs
    ├── learner.rs      # Learner trait + StatisticalLearner
    ├── intervener.rs   # Intervener + strategies
    └── trust.rs        # TrustManager + state persistence
```

## 📋 Roadmap

| Phase | Status | Focus |
|-------|--------|-------|
| Phase 0: Skeleton | ✅ | Project setup, layer stubs |
| Phase 1: Specs | ✅ | OpenSpec definitions |
| Phase 2: Observer | ✅ | LogObserver, MetricsObserver |
| Phase 3: Learner | ✅ | Anomaly detection, topology |
| Phase 4: Trust | ✅ | Persistence, transitions |
| Phase 5: Intervener | ✅ | Shell/HTTP strategies |
| Phase 6: Integration | ✅ | Event loop, config, main |
| Phase 7: Production | 🔜 | Docker, CI/CD, docs polish |

## 🛠️ Requirements

- **Rust** 1.70+ (edition 2021)
- **Dependencies:** tokio, serde, reqwest, tracing, uuid, chrono, async-trait

## 📜 License

MIT © HiYen Wong. See [LICENSE](LICENSE) for details.

## 🔗 Links

- **Repository:** <https://github.com/hiyenwong/migi>
- **OpenSpec:** [openspec.dev](https://openspec.dev/)
- **Inspiration:** [Parasyte (寄生獣)](https://en.wikipedia.org/wiki/Parasyte)
