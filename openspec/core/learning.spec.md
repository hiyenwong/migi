# Migi — 认知层 (Learner) 规范

## 概述

Learner 是 Migi 的"大脑"——从 Observer 的事件流中学习宿主系统的行为模式，构建内部世界模型，并生成预测。

---

### Requirement: 系统行为模型
The system SHALL maintain a SystemModel that represents the learner's understanding of the host.

#### Scenario: 模型字段
GIVEN a SystemModel
THEN it SHALL contain:
  - version: incrementing counter for each update
  - observed_events: total number of events processed
  - prediction_accuracy: float 0..1
  - model_entropy: float 0..1 (lower = more certain)
  - identified_subsystems: count of recognized subsystems
  - identified_patterns: count of recognized event patterns

#### Scenario: 模型可靠性判断
GIVEN a SystemModel with prediction_accuracy = 0.95
AND trust_threshold = 0.05
WHEN is_reliable(trust_threshold) is called
THEN it SHALL return true (0.95 >= 1.0 - 0.05)

GIVEN a SystemModel with prediction_accuracy = 0.80
AND trust_threshold = 0.05
WHEN is_reliable(trust_threshold) is called
THEN it SHALL return false (0.80 < 0.95)

### Requirement: 事件处理
The system SHALL update the internal model when processing events.

#### Scenario: 批量事件处理
GIVEN a Learner with an empty model
WHEN process_events() is called with a batch of events
THEN each event SHALL be processed
AND the model version SHALL increment
AND observed_events SHALL increase by the batch size
AND identified_patterns SHALL reflect unique event patterns

#### Scenario: 模式识别
GIVEN events from multiple sources and types
WHEN process_events() is called
THEN the learner SHALL identify unique source-type patterns
AND update the identified_patterns count

### Requirement: 预测生成
The system SHALL generate predictions about future events.

#### Scenario: 预测结构
GIVEN a Learner has processed events
WHEN predict() is called
THEN it SHALL return:
  - event_distribution: list of (pattern, probability) pairs
  - anomaly_probability: float 0..1
  - confidence: float 0..1 (based on model accuracy)

#### Scenario: 预测分布归一化
GIVEN the event_distribution contains multiple patterns
WHEN the probabilities are summed
THEN the sum SHALL be approximately 1.0

#### Scenario: 无数据预测
GIVEN a Learner has not processed any events
WHEN predict() is called
THEN the event_distribution SHALL be empty
AND confidence SHALL be 0.0

### Requirement: 异常检测
The system SHALL detect anomalous event patterns.

#### Scenario: 频率偏离检测
GIVEN the learner has established a baseline frequency for event patterns
WHEN a new event batch shows significantly different frequencies
THEN the anomaly_probability SHALL increase
AND the deviation SHALL be measurable against the baseline

#### Scenario: 新事件类型检测
GIVEN the learner has observed events from known patterns
WHEN a completely new event type appears
THEN the anomaly_probability SHALL increase
AND the new pattern SHALL be recorded for future baseline

### Requirement: 学习者抽象
The system SHALL provide a Learner trait for different learning implementations.

#### Scenario: 统计学习器
GIVEN a StatisticalLearner
WHEN events are processed
THEN it SHALL use frequency-based analysis
AND predictions SHALL be based on historical event distributions

#### Scenario: 可替换学习器
GIVEN the Learner trait interface
WHEN a new learning implementation is created
THEN it SHALL implement:
  - process_events(&mut self, events) -> Result<()>
  - get_model(&self) -> &SystemModel
  - predict(&self) -> Result<Predictions>
