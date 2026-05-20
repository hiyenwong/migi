# Migi 架构总览

## 设计理念

Migi 是一个**共生型 AI Agent**，灵感来自《寄生兽》中的小右。

它不试图接管系统，而是寄生在系统边缘，通过侧信道观察、学习宿主行为，在信任边界允许的范围内进行局部介入。

## 核心原则

1. **渐进式信任** — 权限是争取来的，不是预设的
2. **故障域隔离** — Agent 出问题不影响宿主核心
3. **所有行动可回滚** — 每次介入都有 undo 路径
4. **相变需验证** — 阶段提升需要模型准确率 + 信任评分双重验证

## 架构图

```
┌─────────────────────────────────────────────────────┐
│                    Host System                       │
│  (sqlite-knowledge-graph / any target system)        │
└──────────────────────┬──────────────────────────────┘
                       │ side-channel (read-only)
                       ▼
┌──────────────────────────────────────────────────────┐
│                   Migi Agent                          │
│                                                      │
│  ┌──────────┐    events    ┌──────────┐              │
│  │ Observer  │ ─────────▶  │ Learner  │              │
│  │ (感知层)  │             │ (认知层)  │              │
│  └──────────┘              └────┬─────┘              │
│                                 │ predictions         │
│                                 ▼                     │
│                        ┌──────────────┐               │
│                        │  Intervener   │               │
│                        │   (行动层)    │               │
│                        └──────┬───────┘               │
│                               │ approve/reject        │
│                               ▼                       │
│                        ┌──────────────┐               │
│                        │    Trust     │               │
│                        │   (信任层)    │               │
│                        └──────────────┘               │
│                                                      │
└──────────────────────────────────────────────────────┘
```

## 模块设计

### 1. Observer（感知层）

| 属性 | 值 |
|------|-----|
| 路径 | `src/observer.rs` |
| 职责 | 静默观察宿主数据流 |
| 接口 | `ObservationChannel` trait |
| 依赖 | error |
| 被依赖 | Learner |

核心结构：
- `Observer`: 管理多个观察通道
- `HostEvent`: 标准化的事件结构
- `ObservationChannel`: 观察通道 trait
- `EventType`: 事件类型枚举
- `Severity`: 事件优先级枚举

实现类：
- `LogObserver`: 日志流观察者
- `MetricsObserver`: 指标轮询观察者

### 2. Learner（认知层）

| 属性 | 值 |
|------|-----|
| 路径 | `src/learner.rs` |
| 职责 | 构建系统内部模型，生成预测 |
| 接口 | `Learner` trait |
| 依赖 | error, observer |
| 被依赖 | Intervener, Trust |

核心结构：
- `SystemModel`: 系统行为模型
- `Predictions`: 预测结果
- `Learner`: 学习器 trait
- `StatisticalLearner`: 基于统计的默认学习器

### 3. Intervener（行动层）

| 属性 | 值 |
|------|-----|
| 路径 | `src/intervener.rs` |
| 职责 | 在信任边界内执行介入动作 |
| 接口 | `InterventionStrategy` trait |
| 依赖 | error, trust |
| 被依赖 | 无（叶子模块） |

核心结构：
- `Intervention`: 介入动作
- `InterventionResult`: 介入结果
- `InterventionTrigger`: 触发原因
- `Action`: 动作类型
- `InterventionStrategy`: 策略 trait
- `Intervener`: 介入执行器

### 4. Trust（信任层）

| 属性 | 值 |
|------|-----|
| 路径 | `src/trust.rs` |
| 职责 | 管理操作权限和相变门控 |
| 接口 | `TrustManager` |
| 依赖 | config, error, learner, intervener |
| 被依赖 | Intervener |

核心结构：
- `TrustState`: 信任状态
- `TrustManager`: 信任管理器
- `SymbiosisPhase`: 共生阶段枚举

## 数据流

```
Host Event → Observer → Learner → Predictions
                                      ↓
                              Intervener attempts
                                      ↓
                              Trust authorizes
                                      ↓
                              Execute or Reject
                                      ↓
                              TrustState updates
                                      ↓
                              Phase transition check
```

## 依赖层级

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
trust (信任层，依赖所有上层)
  ↓
main (入口，组装所有层)
```

**关键约束**：下层不能引用上层。observer 不引用 learner，learner 不引用 intervener，intervener 不引用 trust（trust 反过来引用 intervener 来做授权检查）。

## 扩展点

1. **新的 ObservationChannel** — 实现 trait 即可接入新的数据源
2. **新的 Learner** — 替换 StatisticalLearner 为 ML 模型
3. **新的 InterventionStrategy** — 实现不同的执行后端
4. **新的相变条件** — 修改 TrustManager 的评估逻辑
