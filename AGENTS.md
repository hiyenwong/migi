# Agent Guide: migi

## Quick Start

```bash
cargo build                          # 编译
cargo test                           # 运行测试
cargo fmt                            # 格式化
cargo clippy -- -D warnings          # Lint（CI 强制通过）
cargo run                            # 启动
RUST_LOG=info cargo run              # 带日志启动
```

## Architecture

层级依赖方向（单向，不得逆向引用）：

```
config (配置定义)
  ↓
error (错误类型)
  ↓
observer (感知层)
  ↓
learner (认知层)
  ↓
intervener (行动层)
  ↓
trust (信任层)
  ↓
main (入口)
```

详见 [docs/architecture/overview.md](docs/architecture/overview.md)

## Key Domains

| 域 | 路径 | 设计文档 | 状态 |
|----|------|----------|------|
| config | `src/config.rs` | — | 完整实现 |
| error | `src/error.rs` | — | 完整实现 |
| observer | `src/observer.rs` | [docs/architecture/overview.md](docs/architecture/overview.md) | 完整实现 (LogObserver + MetricsObserver) |
| learner | `src/learner.rs` | [docs/architecture/overview.md](docs/architecture/overview.md) | 完整实现 (StatisticalLearner + 异常检测) |
| intervener | `src/intervener.rs` | [docs/architecture/overview.md](docs/architecture/overview.md) | 完整实现 (Shell + HTTP 策略) |
| trust | `src/trust.rs` | [docs/architecture/overview.md](docs/architecture/overview.md) | 完整实现 (持久化 + 相变门控) |

## OpenSpec

所有功能必须先写 spec，再实现。

| 规范 | 路径 |
|------|------|
| 共生系统核心 | [openspec/core/symbiosis.spec.md](openspec/core/symbiosis.spec.md) |
| 感知层 | [openspec/core/observation.spec.md](openspec/core/observation.spec.md) |
| 认知层 | [openspec/core/learning.spec.md](openspec/core/learning.spec.md) |
| 行动层 | [openspec/core/intervention.spec.md](openspec/core/intervention.spec.md) |
| 信任层 | [openspec/core/trust.spec.md](openspec/core/trust.spec.md) |

## Quality Gates

| 检查项 | 命令 | 要求 |
|--------|------|------|
| 编译 | `cargo build` | 零错误 |
| 测试 | `cargo test` | 全通过 |
| 格式 | `cargo fmt -- --check` | 零 diff |
| Lint | `cargo clippy -- -D warnings` | 零警告 |

## Conventions

- **错误类型**：各域使用 `MigiError` 统一错误，向上用 `?` 传播
- **日志**：使用 `tracing` crate，结构化字段
- **函数签名**：所有公开函数必须有类型标注
- **文件大小**：单文件不超过 300 行，超出时拆分为子模块

## 设计理念

详见 [docs/philosophy.md](docs/philosophy.md)

> **不要试图制造后藤，而要培养一个小右，然后观察它何时准备好进化。**
