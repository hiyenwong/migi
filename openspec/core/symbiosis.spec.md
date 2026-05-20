# Migi — 共生型 AI Agent 核心规范

## 概述

Migi 是一个共生型 AI Agent，用 Rust 编写。它寄生在宿主系统中，通过观察、学习、介入三阶段循环，逐步建立信任并最终获得局部操作权限。

核心隐喻：《寄生兽》中的小右（Migi/右）。

---

## 核心概念

### 共生阶段 (Symbiosis Phase)

Migi 的操作权限由当前共生阶段决定：

| 阶段 | 名称 | 读 | 写(隔离) | 建议 | 紧急阻断 |
|------|------|---|---------|------|---------|
| 1 | Observation | ✅ | ❌ | ❌ | ❌ |
| 2 | Assistance | ✅ | ❌ | ✅ | ❌ |
| 3 | LocalTakeover | ✅ | ✅ | ✅ | ❌ |
| 4 | ControlledTransition | ✅ | ✅ | ✅ | ✅ |

---

## 需求定义

### Requirement: 四层架构
The system SHALL implement four distinct layers: Observer, Learner, Intervener, and Trust. Each layer communicates only with adjacent layers.

#### Scenario: 层间通信方向
GIVEN a running Migi instance
WHEN the Observer detects a host event
THEN the event is passed to the Learner
AND the Learner updates its internal model
AND the Intervener is notified of predictions
AND the Trust layer evaluates whether intervention is authorized
AND no layer communicates with a non-adjacent layer

#### Scenario: 架构不可逆向依赖
GIVEN the source code structure
WHEN a module in the Learner layer is analyzed
THEN it SHALL NOT import modules from Intervener or Trust layers
AND Observer modules SHALL NOT import from Learner, Intervener, or Trust

### Requirement: 共生阶段控制
The system SHALL restrict agent capabilities based on the current symbiosis phase.

#### Scenario: 观察期只读
GIVEN Migi is in the Observation phase
WHEN an intervention of type Hotfix is attempted
THEN the Trust layer SHALL reject the intervention
AND the rejection SHALL be logged

#### Scenario: 辅助期可提供建议
GIVEN Migi is in the Assistance phase
WHEN an intervention of type Suggest is attempted
THEN the Trust layer SHALL authorize the intervention
AND WHEN an intervention of type Hotfix is attempted
THEN the Trust layer SHALL reject the intervention

#### Scenario: 局部接管可隔离写操作
GIVEN Migi is in the LocalTakeover phase
WHEN an intervention of type Hotfix is attempted on an allowed target
THEN the Trust layer SHALL authorize the intervention
AND the intervention SHALL be executed in an isolated environment

### Requirement: 相变机制
The system SHALL provide a phase transition mechanism that upgrades the symbiosis phase when specific conditions are met.

#### Scenario: 从观察期到辅助期
GIVEN Migi is in the Observation phase
AND the Learner's model accuracy exceeds the trust threshold (1.0 - threshold)
AND at least 100 events have been observed
WHEN the phase transition is evaluated
THEN Migi SHALL transition to the Assistance phase

#### Scenario: 从辅助期到局部接管
GIVEN Migi is in the Assistance phase
AND the trust score is >= 0.8
AND there have been at least 10 consecutive successful interventions
AND the model remains reliable
WHEN the phase transition is evaluated
THEN Migi SHALL transition to the LocalTakeover phase

#### Scenario: 相变失败不降级
GIVEN Migi has reached the LocalTakeover phase
AND an intervention fails
WHEN the phase transition is evaluated
THEN Migi SHALL NOT automatically downgrade to a lower phase
AND the trust score SHALL decrease

### Requirement: 所有行动可回滚
The system SHALL ensure every intervention action is rollbackable.

#### Scenario: 热修复回滚
GIVEN a Hotfix intervention was successfully executed
AND the intervention has a rollback_action defined
WHEN rollback is requested
THEN the rollback_action SHALL be executed
AND the result SHALL be logged

#### Scenario: 诊断操作无需回滚
GIVEN a Diagnose intervention was executed
WHEN the intervention completes
THEN the rollback_needed flag SHALL be false

### Requirement: 操作目标白名单
The system SHALL restrict interventions to a configurable whitelist of targets.

#### Scenario: 白名单允许
GIVEN the allowed targets list includes ["database", "cache"]
WHEN an intervention targets "database"
THEN the Trust layer SHALL authorize based on phase rules

#### Scenario: 白名单拒绝
GIVEN the allowed targets list includes ["database"]
WHEN an intervention targets "api-gateway"
THEN the Trust layer SHALL reject with a TrustViolation error

### Requirement: 审计日志
The system SHALL log all interventions, phase transitions, and trust violations.

#### Scenario: 干预日志
WHEN an intervention is attempted
THEN the log SHALL contain: intervention ID, action type, target, authorization result
AND the log level SHALL be INFO for authorized, WARN for rejected

#### Scenario: 相变日志
WHEN a phase transition occurs
THEN the log SHALL contain: old phase, new phase, reason
AND the log level SHALL be WARN

### Requirement: 配置驱动
The system SHALL load configuration from a TOML file.

#### Scenario: 默认配置
GIVEN no configuration file is provided
WHEN Migi starts
THEN it SHALL use default configuration:
  - phase: Observation
  - trust_threshold: 0.05
  - allowed_intervention_targets: []
  - max_concurrent_interventions: 1

#### Scenario: 自定义配置
GIVEN a configuration file at `config/migi.toml`
WHEN Migi starts
THEN it SHALL load the configuration
AND override defaults with values from the file
AND log the loaded configuration
