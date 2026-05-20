# Migi — 信任层 (Trust) 规范

## 概述

Trust 是 Migi 的"边界"——管理 Agent 的操作权限，决定是否允许介入、何时可以相变到更高阶段。

核心隐喻：新一与小右之间的信任博弈。

---

### Requirement: 信任状态管理
The system SHALL maintain a trust state that tracks the agent's reliability.

#### Scenario: 信任评分计算
GIVEN the TrustState tracks successful and failed interventions
WHEN record_success() is called
THEN successful_interventions SHALL increment by 1
AND consecutive_successes SHALL increment by 1
AND trust_score SHALL be recalculated as successful / total

WHEN record_failure() is called
THEN failed_interventions SHALL increment by 1
AND consecutive_successes SHALL reset to 0
AND trust_score SHALL be recalculated

#### Scenario: 初始状态
GIVEN a new TrustState
WHEN the state is created
THEN trust_score SHALL be 0.0
AND all counters SHALL be 0

### Requirement: 授权检查
The system SHALL check intervention authorization based on phase and target whitelist.

#### Scenario: 目标白名单检查
GIVEN allowed_targets = ["database", "cache"]
WHEN an intervention targets "database"
THEN the target check SHALL pass
WHEN an intervention targets "api-gateway"
THEN a TrustViolation SHALL be returned

#### Scenario: 阶段权限检查
GIVEN the current phase is Observation
WHEN a Suggest action is attempted
THEN the check SHALL fail (Observation does not allow suggestions)
WHEN a Diagnose action is attempted
THEN the check SHALL pass (diagnose is always allowed)

### Requirement: 相变评估
The system SHALL evaluate phase transition conditions based on model quality and trust metrics.

#### Scenario: 观察期 → 辅助期
GIVEN phase = Observation
AND model.prediction_accuracy >= (1.0 - trust_threshold)
AND model.observed_events >= 100
WHEN evaluate_transition() is called
THEN it SHALL return Some(Assistance)

#### Scenario: 辅助期 → 局部接管
GIVEN phase = Assistance
AND trust_score >= 0.8
AND consecutive_successes >= 10
AND model is reliable
WHEN evaluate_transition() is called
THEN it SHALL return Some(LocalTakeover)

#### Scenario: 局部接管 → 受控相变
GIVEN phase = LocalTakeover
AND trust_score >= 0.8
AND consecutive_successes >= 10
AND model.prediction_accuracy >= 0.95
WHEN evaluate_transition() is called
THEN it SHALL return Some(ControlledTransition)

#### Scenario: 最高阶段不再相变
GIVEN phase = ControlledTransition
WHEN evaluate_transition() is called
THEN it SHALL return None

#### Scenario: 条件不满足
GIVEN phase = Observation
AND model.observed_events < 100
WHEN evaluate_transition() is called
THEN it SHALL return None

### Requirement: 相变执行
The system SHALL execute phase transitions with appropriate logging.

#### Scenario: 执行相变
GIVEN a new phase has been evaluated as valid
WHEN transition(new_phase) is called
THEN the phase SHALL be updated
AND consecutive_successes SHALL reset to 0
AND a WARN level log entry SHALL be created with old and new phase

### Requirement: 信任降级
The system SHALL NOT automatically downgrade the phase on failure.

#### Scenario: 失败不降级
GIVEN Migi is in the LocalTakeover phase
AND an intervention fails
WHEN evaluate_transition() is called
THEN the returned phase SHALL NOT be lower than the current phase
AND trust_score SHALL decrease due to the failure
