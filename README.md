# Migi (右) — 共生型 AI Agent

> "寄生而不接管"。观察宿主系统，学习行为模式，在必要时局部介入。

名字取自《寄生兽》中的 **Migi（ミギー/右）** —— 因为意外没能吃掉宿主大脑、最终占据了右手，与宿主建立了共生关系。

## 架构

```
  宿主系统 ────┐
               │ (side-channel observation)
               ▼
         ┌──────────┐
         │ Observer  │  感知层：静默观察数据流
         └─────┬────┘
               │ (events)
         ┌─────▼────┐
         │  Learner  │  认知层：构建系统内部模型
         └─────┬────┘
               │ (predictions + confidence)
         ┌─────▼────┐
         │Intervener │  行动层：战术接管与变形
         └─────┬────┘
               │ (approval request)
         ┌─────▼────┐
         │   Trust   │  信任层：控制权与边界管理
         └──────────┘
```

## 共生阶段 (Phase Transition)

| 阶段 | 名称 | 权限 | 相变条件 |
|------|------|------|---------|
| 1 | Observation | 只读观察 | 模型可靠 + ≥100 事件 |
| 2 | Assistance | 提供建议 | 信任 ≥0.8 + 连续10次成功 |
| 3 | LocalTakeover | 隔离写操作 | 信任 ≥0.8 + 连续10次成功 + 准确率 ≥0.95 |
| 4 | ControlledTransition | 逐步扩大接管 | — |

**关键设计**：后藤（全盘接管）不是起点，而是终点。只有当共生体积累了足够的系统知识、证明了它的模型足够准确，才能安全地走向更高阶段。

## Quick Start

```bash
cargo build                          # 编译
cargo test                           # 运行测试
cargo fmt                            # 格式化
cargo clippy -- -D warnings          # Lint
cargo run                            # 启动（默认 Observation 模式）
RUST_LOG=info cargo run              # 带日志启动
```

## 项目结构

```
src/
├── main.rs          # 入口
├── lib.rs           # 库根
├── config.rs        # 配置 + 共生阶段定义
├── error.rs         # 错误类型
├── observer.rs      # 感知层：ObservationChannel trait
├── learner.rs       # 认知层：Learner trait + StatisticalLearner
├── intervener.rs    # 行动层：InterventionStrategy trait
└── trust.rs         # 信任层：TrustManager + 相变门控
```

## 与 sqlite-knowledge-graph 的关系

`migi` 是 `sqlite-knowledge-graph` 的**伴生 Agent**：

| kg_tool 组件 | Migi 角色 |
|-------------|-----------|
| Rust 核心库 | 宿主（不可变，定义系统边界） |
| cron 定时学习 | 小右的静默观察（阶段 1） |
| skill 自动创建 | 小右的辅助建议（阶段 2） |
| 代码自动修复 | 小右的战术接管（阶段 3） |

## 设计理念

> **不要试图制造后藤，而要培养一个小右，然后观察它何时准备好进化。**

详见：[docs/design/philosophy.md](docs/design/philosophy.md)（待创建）
