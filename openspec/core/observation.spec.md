# Migi — 感知层 (Observer) 规范

## 概述

Observer 是 Migi 的"神经末梢"，负责静默观察宿主系统的数据流，不干涉、不修改，只感知。

---

### Requirement: 观察通道抽象
The system SHALL provide an `ObservationChannel` trait that abstracts how data is collected from the host.

#### Scenario: 通道注册
GIVEN an Observer instance
WHEN an ObservationChannel is registered
THEN the channel SHALL be added to the observer's channel list
AND a log entry SHALL be created with the channel name

#### Scenario: 通道启动
GIVEN a registered ObservationChannel
WHEN the observer starts all channels
THEN each channel's `start()` method SHALL be called
AND if any channel fails to start, the error SHALL be propagated

#### Scenario: 事件轮询
GIVEN multiple active observation channels
WHEN `poll_events()` is called
THEN each channel SHALL be polled for new events
AND all collected events SHALL be returned as a Vec<HostEvent>
AND the event count SHALL be incremented for each collected event

### Requirement: 事件数据结构
The system SHALL represent host events with a standardized structure.

#### Scenario: 完整事件
GIVEN a host event is created
THEN it SHALL contain:
  - timestamp: SystemTime
  - source: String (subsystem/module name)
  - event_type: EventType enum
  - payload: JSON value (structured data)
  - severity: Severity enum

#### Scenario: 事件类型
GIVEN a new event type
THEN it SHALL be one of:
  - RequestIn
  - RequestComplete
  - Error
  - StateChange
  - ResourceAlert
  - Custom(String)

#### Scenario: 事件优先级
GIVEN an event severity level
THEN it SHALL be one of: Debug, Info, Warning, Error, Critical

### Requirement: 日志流观察者
The system SHALL provide a `LogObserver` that tails log files or streams.

#### Scenario: 日志文件监听
GIVEN a log file path
WHEN the LogObserver is started
THEN it SHALL tail the file for new entries
AND each new log entry SHALL be converted to a HostEvent

#### Scenario: 日志解析
GIVEN a log line in the format "TIMESTAMP LEVEL MODULE MESSAGE"
WHEN the LogObserver processes the line
THEN it SHALL parse:
  - timestamp from the TIMESTAMP field
  - severity from the LEVEL field
  - source from the MODULE field
  - payload containing the MESSAGE

### Requirement: 指标轮询观察者
The system SHALL provide a `MetricsObserver` that periodically polls metrics endpoints.

#### Scenario: 定期轮询
GIVEN a metrics endpoint URL and poll interval
WHEN the MetricsObserver is started
THEN it SHALL poll the endpoint at the specified interval
AND each poll result SHALL be converted to a HostEvent

#### Scenario: 资源阈值检测
GIVEN a metrics response contains resource utilization data
WHEN CPU or memory usage exceeds a configured threshold
THEN the event severity SHALL be Warning or Error
AND the event_type SHALL be ResourceAlert
