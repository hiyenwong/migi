# Migi Project Plan

> "寄生而不接管" — Build a symbiotic AI agent in Rust

---

## Vision

Migi 是一个 **共生型 AI Agent**，灵感来自《寄生兽》中的小右（Migi）。

它不试图"接管"整个系统，而是 **寄生在系统边缘**，通过侧信道观察、学习宿主行为，在必要时局部介入——就像小右静默观察新一，在危险时刻变形为刀刃。

## 核心原则

1. **渐进式信任** — 权限从观察到介入是逐步获得的，不是预设的
2. **故障域隔离** — Agent 出问题不影响宿主核心
3. **所有行动可回滚** — 每次介入都有 undo 路径
4. **相变需验证** — 阶段提升需要模型准确率 + 信任评分双重验证
5. **OpenSpec 驱动** — 所有功能先写 spec，再实现

## 阶段划分

### Phase 0: 骨架（已完成 ✅）

- [x] Rust 项目初始化
- [x] 四层架构骨架（observer / learner / intervener / trust）
- [x] 编译通过，零警告

### Phase 1: 规范定义（已完成 ✅）

- [x] 完成整体架构设计文档
- [x] 编写 OpenSpec 核心规范
- [x] 定义接口契约（trait 定义）
- [x] 设计配置与数据格式

### Phase 2: 感知层实现（已完成 ✅）

- [x] 实现 `LogObserver`（日志流观察者）
- [x] 实现 `MetricsObserver`（指标轮询观察者）
- [x] 实现事件缓冲与流聚合
- [x] 单元测试覆盖

### Phase 3: 认知层实现（已完成 ✅）

- [x] 增强 `StatisticalLearner`
- [x] 实现异常检测算法
- [x] 实现系统拓扑推断
- [x] 单元测试覆盖

### Phase 4: 信任层实现（已完成 ✅）

- [x] 实现 TrustManager 的持久化
- [x] 实现相变门控的完整逻辑
- [x] 实现白名单/黑名单管理
- [x] 单元测试覆盖

### Phase 5: 行动层实现（已完成 ✅）

- [x] 实现 `ShellInterventionStrategy`（命令执行）
- [x] 实现 `HttpInterventionStrategy`（HTTP 调用）
- [x] 实现回滚机制
- [x] 审计日志

### Phase 6: 系统集成（已完成 ✅）

- [x] 实现主循环（event loop）
- [x] 配置文件加载（TOML）
- [x] 接入 sqlite-knowledge-graph 作为第一个宿主
- [x] E2E 测试

### Phase 7: 生产化（未开始 🟡）

- [ ] 结构化日志 + 指标导出
- [ ] Docker 支持
- [ ] CI/CD 配置
- [ ] 文档完善

## 项目结构

```
migi/
├── Cargo.toml
├── README.md
├── AGENTS.md
├── .gitignore
├── docs/                          # 设计文档
│   ├── plan.md                    # 本文件
│   ├── philosophy.md              # 设计理念（寄生兽隐喻）
│   └── architecture/
│       └── overview.md            # 架构总览
├── openspec/                      # OpenSpec 规范
│   ├── README.md
│   ├── core/
│   │   ├── symbiosis.spec.md      # 共生系统核心规范
│   │   ├── observation.spec.md    # 感知层规范
│   │   ├── learning.spec.md       # 认知层规范
│   │   ├── intervention.spec.md   # 行动层规范
│   │   └── trust.spec.md          # 信任层规范
│   └── changes/                   # 变更跟踪
├── config/
│   └── migi.toml                  # 配置文件模板
└── src/
    ├── main.rs
    ├── lib.rs
    ├── config.rs
    ├── error.rs
    ├── observer.rs
    ├── learner.rs
    ├── intervener.rs
    └── trust.rs
```

## 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust 2021 |
| 异步运行时 | tokio |
| 序列化 | serde + serde_json |
| 日志 | tracing + tracing-subscriber |
| 配置 | toml |
| 测试 | cargo test |
| Lint | clippy + rustfmt |

## 开发流程

1. 写 OpenSpec → 2. 实现代码 → 3. 写测试 → 4. clippy 通过 → 5. commit → 6. 重复
