# Migi — 行动层 (Intervener) 规范

## 概述

Intervener 是 Migi 的"刀刃"——当系统遭遇威胁或 Learner 检测到异常时，在信任边界允许的范围内进行局部介入。

核心原则：所有行动必须可回滚。

---

### Requirement: 介入动作类型
The system SHALL support multiple intervention action types with different permission levels.

#### Scenario: 只读诊断
GIVEN any symbiosis phase
WHEN a Diagnose action is attempted
THEN the Trust layer SHALL always authorize it

#### Scenario: 建议
GIVEN the phase is Assistance or higher
WHEN a Suggest action is attempted
THEN the Trust layer SHALL authorize it

#### Scenario: 热修复
GIVEN the phase is LocalTakeover or higher
AND the target is in the allowed targets list
WHEN a Hotfix action is attempted
THEN the Trust layer SHALL authorize it

#### Scenario: 紧急阻断
GIVEN the phase is ControlledTransition
WHEN an EmergencyBlock action is attempted
THEN the Trust layer SHALL authorize it
AND GIVEN the phase is below ControlledTransition
WHEN an EmergencyBlock action is attempted
THEN the Trust layer SHALL reject with TrustViolation

### Requirement: 介入策略抽象
The system SHALL provide an `InterventionStrategy` trait for implementing different intervention backends.

#### Scenario: 策略注册
GIVEN an Intervener instance
WHEN an InterventionStrategy is registered
THEN the strategy SHALL be added to the strategy list

#### Scenario: 命令执行策略
GIVEN a ShellInterventionStrategy
WHEN execute() is called with a Hotfix action
THEN the command SHALL be executed in an isolated subprocess
AND the output SHALL be captured
AND the result SHALL include rollback information

#### Scenario: HTTP 调用策略
GIVEN an HttpInterventionStrategy
WHEN execute() is called with a Reconfigure action
THEN an HTTP request SHALL be sent to the target endpoint
AND the response SHALL be captured
AND the result SHALL include the original configuration for rollback

### Requirement: 回滚机制
The system SHALL provide rollback capability for all intervention actions.

#### Scenario: 执行回滚
GIVEN a previously executed intervention
WHEN rollback is requested
THEN the corresponding strategy's rollback() method SHALL be called
AND the rollback result SHALL be logged

#### Scenario: 无需回滚
GIVEN a Diagnose intervention
WHEN the intervention completes
THEN rollback_needed SHALL be false

### Requirement: 审计追踪
The system SHALL maintain a history of all interventions.

#### Scenario: 历史记录
GIVEN multiple interventions have been executed
WHEN history() is called
THEN all intervention results SHALL be returned in chronological order

#### Scenario: 介入日志
WHEN an intervention is attempted
THEN the log SHALL contain:
  - intervention ID (UUID)
  - action type
  - target
  - trigger reason
  - execution result (success/failure)
  - output
  - rollback status

### Requirement: 介入触发
The system SHALL support multiple trigger mechanisms for interventions.

#### Scenario: 预测触发
GIVEN the Learner predicts an anomaly with high probability
WHEN the prediction confidence exceeds the configured threshold
THEN an intervention SHALL be triggered with PredictedAnomaly trigger

#### Scenario: 手动触发
GIVEN a manual intervention request from the user (master)
WHEN the request specifies an action and target
THEN an intervention SHALL be triggered with Manual trigger

#### Scenario: 定期健康检查
GIVEN a scheduled check interval
WHEN the interval elapses
THEN a health check intervention SHALL be triggered with ScheduledCheck trigger
