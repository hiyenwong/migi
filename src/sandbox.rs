//! Sandbox — 沙盒模拟系统
//!
//! 在隔离环境中模拟宿主系统行为，测试 Migi 的完整生命周期。
//! 支持定义多阶段场景：正常 → 异常 → 恢复 → 终极目标。

use crate::error::MigiResult;
use crate::intervener::{Intervener, InterventionTrigger, ShellInterventionStrategy};
use crate::learner::{Learner, Predictions, StatisticalLearner};
use crate::observer::{EventType, HostEvent, ObservationChannel, Observer, Severity};
use crate::trust::TrustManager;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ─── 场景定义 ───────────────────────────────────────────

/// 模拟阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimPhase {
    /// 正常基线 — 标准的请求/响应流
    Normal,
    /// 异常注入 — CPU 飙升、错误率上升
    Anomaly,
    /// 恢复 — 系统自我修复
    Recovery,
    /// 终极测试 — 触发相变
    TransitionTest,
}

/// 一个模拟轮次的事件配置
#[derive(Debug, Clone)]
pub struct SimRound {
    pub phase: SimPhase,
    pub event_count: usize,
    pub name: String,
}

impl SimRound {
    fn events(&self, round_index: usize) -> Vec<HostEvent> {
        match self.phase {
            SimPhase::Normal => generate_normal_events(self.event_count, round_index),
            SimPhase::Anomaly => generate_anomaly_events(self.event_count),
            SimPhase::Recovery => generate_recovery_events(self.event_count),
            SimPhase::TransitionTest => generate_transition_events(self.event_count, round_index),
        }
    }
}

/// 完整模拟场景
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub rounds: Vec<SimRound>,
}

impl Scenario {
    /// 标准正常场景 — 逐步建立模型基线
    pub fn baseline() -> Self {
        Self {
            name: "baseline".into(),
            description: "正常业务流量，建立系统基线".into(),
            rounds: (0..20)
                .map(|i| SimRound {
                    phase: SimPhase::Normal,
                    event_count: 20 + (i % 3) * 5,
                    name: format!("normal-round-{}", i + 1),
                })
                .collect(),
        }
    }

    /// 异常检测场景 — 正常 → 异常突发 → 恢复
    pub fn anomaly_detection() -> Self {
        Self {
            name: "anomaly-detection".into(),
            description: "模拟异常突发，测试异常检测能力".into(),
            rounds: vec![
                // 10 轮正常
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 25,
                    name: "normal-1".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 30,
                    name: "normal-2".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 20,
                    name: "normal-3".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 28,
                    name: "normal-4".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 22,
                    name: "normal-5".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 25,
                    name: "normal-6".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 30,
                    name: "normal-7".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 20,
                    name: "normal-8".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 28,
                    name: "normal-9".into(),
                },
                SimRound {
                    phase: SimPhase::Normal,
                    event_count: 22,
                    name: "normal-10".into(),
                },
                // 3 轮突发异常
                SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 60,
                    name: "anomaly-spike-1".into(),
                },
                SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 80,
                    name: "anomaly-spike-2".into(),
                },
                SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 50,
                    name: "anomaly-spike-3".into(),
                },
                // 5 轮恢复
                SimRound {
                    phase: SimPhase::Recovery,
                    event_count: 25,
                    name: "recovery-1".into(),
                },
                SimRound {
                    phase: SimPhase::Recovery,
                    event_count: 22,
                    name: "recovery-2".into(),
                },
                SimRound {
                    phase: SimPhase::Recovery,
                    event_count: 20,
                    name: "recovery-3".into(),
                },
                SimRound {
                    phase: SimPhase::Recovery,
                    event_count: 25,
                    name: "recovery-4".into(),
                },
                SimRound {
                    phase: SimPhase::Recovery,
                    event_count: 28,
                    name: "recovery-5".into(),
                },
            ],
        }
    }

    /// 相变测试场景 — 大量正常事件驱动相变
    pub fn phase_transition() -> Self {
        Self {
            name: "phase-transition".into(),
            description: "模拟大量正常交互，触发信任积累和相变".into(),
            rounds: (0..30)
                .map(|i| SimRound {
                    phase: if i < 20 {
                        SimPhase::Normal
                    } else {
                        SimPhase::TransitionTest
                    },
                    event_count: 30 + (i * 5) % 40,
                    name: format!("transition-round-{}", i + 1),
                })
                .collect(),
        }
    }

    /// 完整生命周期场景 — 从观察到休眠
    pub fn full_lifecycle() -> Self {
        Self {
            name: "full-lifecycle".into(),
            description: "完整的 Migi 生命周期：观察 → 学习 → 介入 → 教育 → 休眠".into(),
            rounds: {
                let mut r = Vec::new();
                // 15 轮观察（正常建立基线）
                for i in 0..15 {
                    r.push(SimRound {
                        phase: SimPhase::Normal,
                        event_count: 25 + (i % 3) * 5,
                        name: format!("observe-{}", i + 1),
                    });
                }
                // 3 轮异常（触发介入）
                r.push(SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 70,
                    name: "crisis-1".into(),
                });
                r.push(SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 90,
                    name: "crisis-2".into(),
                });
                r.push(SimRound {
                    phase: SimPhase::Anomaly,
                    event_count: 60,
                    name: "crisis-3".into(),
                });
                // 5 轮恢复
                for i in 0..5 {
                    r.push(SimRound {
                        phase: SimPhase::Recovery,
                        event_count: 25,
                        name: format!("recover-{}", i + 1),
                    });
                }
                // 5 轮相变测试
                for i in 0..5 {
                    r.push(SimRound {
                        phase: SimPhase::TransitionTest,
                        event_count: 35,
                        name: format!("transition-{}", i + 1),
                    });
                }
                r
            },
        }
    }
}

// ─── 事件生成器 ─────────────────────────────────────────

fn generate_normal_events(count: usize, seed: usize) -> Vec<HostEvent> {
    let sources = ["api-gateway", "auth-service", "database", "cache", "worker"];
    let types = [
        EventType::RequestIn,
        EventType::RequestComplete,
        EventType::StateChange,
    ];
    let severities = [
        Severity::Debug,
        Severity::Info,
        Severity::Info,
        Severity::Info,
        Severity::Warning,
    ];

    (0..count)
        .map(|i| {
            let source = sources[(seed + i) % sources.len()];
            let event_type = types[(seed + i * 3) % types.len()].clone();
            let severity = severities[(seed + i) % severities.len()];
            HostEvent {
                timestamp: SystemTime::now(),
                source: source.to_string(),
                event_type,
                payload: serde_json::json!({
                    "latency_ms": 10 + (seed * 7 + i * 3) % 100,
                    "status": 200,
                }),
                severity,
            }
        })
        .collect()
}

fn generate_anomaly_events(count: usize) -> Vec<HostEvent> {
    let sources = ["api-gateway", "database", "worker"];
    let types = [EventType::Error, EventType::ResourceAlert, EventType::Error];
    let severities = [
        Severity::Error,
        Severity::Critical,
        Severity::Warning,
        Severity::Error,
    ];

    (0..count)
        .map(|i| {
            let source = sources[i % sources.len()];
            let severity = severities[i % severities.len()];
            HostEvent {
                timestamp: SystemTime::now(),
                source: source.to_string(),
                event_type: types[i % types.len()].clone(),
                payload: serde_json::json!({
                    "error": match severity {
                        Severity::Critical => "out_of_memory",
                        Severity::Error => "connection_timeout",
                        _ => "high_latency",
                    },
                    "latency_ms": 5000 + (i * 100) % 30000,
                }),
                severity,
            }
        })
        .collect()
}

fn generate_recovery_events(count: usize) -> Vec<HostEvent> {
    let sources = ["api-gateway", "auth-service", "database", "cache", "worker"];
    let severities = [Severity::Info, Severity::Info, Severity::Warning];

    (0..count)
        .map(|i| {
            let source = sources[i % sources.len()];
            HostEvent {
                timestamp: SystemTime::now(),
                source: source.to_string(),
                event_type: if i % 5 == 0 {
                    EventType::StateChange
                } else {
                    EventType::RequestComplete
                },
                payload: serde_json::json!({
                    "latency_ms": 20 + (i * 3) % 50,
                    "status": 200,
                    "recovered": true,
                }),
                severity: severities[i % severities.len()],
            }
        })
        .collect()
}

fn generate_transition_events(count: usize, seed: usize) -> Vec<HostEvent> {
    let sources = ["api-gateway", "auth-service", "database", "cache", "worker"];
    let severities = [Severity::Info, Severity::Debug];

    (0..count)
        .map(|i| {
            let source = sources[(seed + i) % sources.len()];
            HostEvent {
                timestamp: SystemTime::now(),
                source: source.to_string(),
                event_type: EventType::RequestComplete,
                payload: serde_json::json!({
                    "latency_ms": 5 + (i * 2) % 30,
                    "status": 200,
                }),
                severity: severities[(seed + i) % severities.len()],
            }
        })
        .collect()
}

// ─── 模拟观察通道 ────────────────────────────────────────

/// 模拟日志观察通道
pub struct SimLogChannel {
    name: String,
    rounds: Vec<SimRound>,
    round_index: usize,
    event_index: usize,
    total_events: u64,
    running: bool,
}

impl SimLogChannel {
    pub fn new(name: &str, scenario: &Scenario) -> Self {
        Self {
            name: name.to_string(),
            rounds: scenario.rounds.clone(),
            round_index: 0,
            event_index: 0,
            total_events: 0,
            running: false,
        }
    }

    pub fn round_name(&self) -> &str {
        if self.round_index < self.rounds.len() {
            &self.rounds[self.round_index].name
        } else {
            "done"
        }
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.round_index, self.rounds.len())
    }
}

#[async_trait]
impl ObservationChannel for SimLogChannel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self) -> MigiResult<()> {
        self.running = true;
        self.round_index = 0;
        self.event_index = 0;
        self.total_events = 0;
        Ok(())
    }

    async fn next_event(&mut self) -> MigiResult<Option<HostEvent>> {
        if !self.running || self.round_index >= self.rounds.len() {
            return Ok(None);
        }

        let round = &self.rounds[self.round_index];
        if self.event_index >= round.events(self.round_index).len() {
            // Move to next round
            self.round_index += 1;
            self.event_index = 0;
            return Ok(None); // No event this poll
        }

        let events = round.events(self.round_index);
        let event = events[self.event_index].clone();
        self.event_index += 1;
        self.total_events += 1;
        Ok(Some(event))
    }

    async fn stop(&mut self) -> MigiResult<()> {
        self.running = false;
        Ok(())
    }
}

/// 模拟指标观察通道
pub struct SimMetricsChannel {
    name: String,
    phase: std::sync::Arc<std::sync::atomic::AtomicU64>,
    poll_count: u64,
    running: bool,
}

impl SimMetricsChannel {
    pub fn new(name: &str, phase_tracker: std::sync::Arc<std::sync::atomic::AtomicU64>) -> Self {
        Self {
            name: name.to_string(),
            phase: phase_tracker,
            poll_count: 0,
            running: false,
        }
    }
}

#[async_trait]
impl ObservationChannel for SimMetricsChannel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self) -> MigiResult<()> {
        self.running = true;
        self.poll_count = 0;
        Ok(())
    }

    async fn next_event(&mut self) -> MigiResult<Option<HostEvent>> {
        if !self.running {
            return Ok(None);
        }

        self.poll_count += 1;
        let phase_code = self.phase.load(std::sync::atomic::Ordering::Relaxed);

        let (cpu, mem) = if phase_code == 1 {
            // ANOMALY phase
            (92.0, 88.0)
        } else if phase_code == 2 {
            // RECOVERY
            (55.0, 60.0)
        } else if phase_code == 3 {
            // TRANSITION TEST
            (30.0, 45.0)
        } else {
            // NORMAL
            (
                35.0 + (self.poll_count % 20) as f64,
                50.0 + (self.poll_count % 10) as f64,
            )
        };

        Ok(Some(HostEvent {
            timestamp: SystemTime::now(),
            source: "metrics".to_string(),
            event_type: if cpu > 80.0 || mem > 80.0 {
                EventType::ResourceAlert
            } else {
                EventType::StateChange
            },
            payload: serde_json::json!({
                "cpu_percent": cpu,
                "memory_percent": mem,
                "threshold": 80.0,
                "exceeded": cpu > 80.0 || mem > 80.0,
            }),
            severity: if cpu > 80.0 || mem > 80.0 {
                if cpu > 90.0 {
                    Severity::Error
                } else {
                    Severity::Warning
                }
            } else {
                Severity::Info
            },
        }))
    }

    async fn stop(&mut self) -> MigiResult<()> {
        self.running = false;
        Ok(())
    }
}

// ─── 沙盒引擎 ────────────────────────────────────────────

/// 单次模拟结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimLog {
    pub round: usize,
    pub round_name: String,
    pub events_in_round: usize,
    pub total_events: u64,
    pub phase: String,
    pub model_version: u64,
    pub model_accuracy: f64,
    pub anomaly_prob: f64,
    pub trust_score: f64,
    pub consecutive_successes: u64,
    pub interventions_attempted: u64,
}

/// 模拟运行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    pub scenario_name: String,
    pub scenario_description: String,
    pub rounds_completed: usize,
    pub total_events_processed: u64,
    pub final_phase: String,
    pub final_trust_score: f64,
    pub final_model_accuracy: f64,
    pub phase_transitions: usize,
    pub interventions: u64,
    pub logs: Vec<SimLog>,
}

/// 沙盒引擎 — 在隔离环境中运行 Migi
pub struct Sandbox {
    pub observer: Observer,
    pub learner: StatisticalLearner,
    pub trust: TrustManager,
    pub intervener: Intervener,
    phase_tracker: std::sync::Arc<std::sync::atomic::AtomicU64>,
    intervention_count: u64,
    phase_transitions: usize,
}

impl Sandbox {
    pub fn new(_name: &str, trust_threshold: f64) -> Self {
        Self {
            observer: Observer::new(),
            learner: StatisticalLearner::new(),
            trust: TrustManager::new(
                crate::config::SymbiosisPhase::Observation,
                trust_threshold,
                vec!["system".to_string()],
            ),
            intervener: Intervener::new(),
            phase_tracker: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            intervention_count: 0,
            phase_transitions: 0,
        }
    }

    /// 注册模拟日志通道
    pub fn with_log_channel(mut self, scenario: &Scenario) -> Self {
        let channel = SimLogChannel::new("sim-log", scenario);
        self.observer.register_channel(channel);
        self
    }

    /// 注册模拟指标通道
    pub fn with_metrics_channel(mut self) -> Self {
        let channel = SimMetricsChannel::new("sim-metrics", self.phase_tracker.clone());
        self.observer.register_channel(channel);
        self
    }

    /// 注册 Shell 策略
    pub fn with_shell_strategy(mut self) -> Self {
        self.intervener
            .register_strategy(ShellInterventionStrategy::new());
        self
    }

    /// 更新阶段跟踪器（给指标通道用）
    fn update_phase_tracker(&self, phase: &SimPhase) {
        let code: u64 = match phase {
            SimPhase::Normal => 0,
            SimPhase::Anomaly => 1,
            SimPhase::Recovery => 2,
            SimPhase::TransitionTest => 3,
        };
        self.phase_tracker
            .store(code, std::sync::atomic::Ordering::Relaxed);
    }

    /// 运行一轮模拟
    pub fn run_round(&mut self, round: &SimRound, round_index: usize) -> MigiResult<SimLog> {
        self.update_phase_tracker(&round.phase);

        let round_events = round.events(round_index);

        // Process events through learner
        if !round_events.is_empty() {
            self.learner.process_events(&round_events)?;
        }

        // Get predictions
        let predictions = self.learner.predict().unwrap_or(Predictions {
            event_distribution: vec![],
            anomaly_probability: 0.0,
            confidence: 0.0,
        });

        let model = self.learner.get_model().clone();

        // If high anomaly, attempt intervention
        let _intervention_attempted = if predictions.anomaly_probability > 0.5 {
            let intervention = crate::intervener::Intervention {
                id: uuid::Uuid::new_v4(),
                trigger: if predictions.anomaly_probability > 0.7 {
                    InterventionTrigger::PredictedAnomaly
                } else {
                    InterventionTrigger::DetectedAnomaly
                },
                target: "system".to_string(),
                action: crate::intervener::Action::Diagnose {
                    command: format!("simulate diagnostic for round {}", round_index + 1),
                },
                executed: false,
                rollbackable: false,
                rollback_action: None,
            };

            if self.trust.authorize(&intervention).is_ok() {
                self.intervention_count += 1;
                true
            } else {
                false
            }
        } else {
            false
        };

        // Evaluate phase transition
        if let Ok(Some(new_phase)) = self.trust.evaluate_transition(&model) {
            if self.trust.transition(new_phase).is_ok() {
                self.phase_transitions += 1;
            }
        }

        Ok(SimLog {
            round: round_index + 1,
            round_name: round.name.clone(),
            events_in_round: round_events.len(),
            total_events: model.observed_events,
            phase: format!("{:?}", self.trust.state().phase),
            model_version: model.version,
            model_accuracy: model.prediction_accuracy,
            anomaly_prob: predictions.anomaly_probability,
            trust_score: self.trust.state().trust_score,
            consecutive_successes: self.trust.state().consecutive_successes,
            interventions_attempted: self.intervention_count,
        })
    }

    /// 运行完整场景
    pub fn run_scenario(mut self, scenario: &Scenario) -> MigiResult<SimResult> {
        let mut logs = Vec::new();

        for (i, round) in scenario.rounds.iter().enumerate() {
            let log = self.run_round(round, i)?;
            logs.push(log);
        }

        let final_model = self.learner.get_model().clone();

        Ok(SimResult {
            scenario_name: scenario.name.clone(),
            scenario_description: scenario.description.clone(),
            rounds_completed: logs.len(),
            total_events_processed: final_model.observed_events,
            final_phase: format!("{:?}", self.trust.state().phase),
            final_trust_score: self.trust.state().trust_score,
            final_model_accuracy: final_model.prediction_accuracy,
            phase_transitions: self.phase_transitions,
            interventions: self.intervention_count,
            logs,
        })
    }

    /// 打印人类可读的模拟摘要
    pub fn print_summary(result: &SimResult) {
        println!("\n═══════════════════════════════════════════");
        println!("  🦁 Migi Sandbox — Simulation Report");
        println!("═══════════════════════════════════════════");
        println!();
        println!("  Scenario:     {}", result.scenario_name);
        println!("  Description:  {}", result.scenario_description);
        println!("  Rounds:       {}", result.rounds_completed);
        println!("  Events:       {}", result.total_events_processed);
        println!();
        println!("  ── Final State ──");
        println!("  Phase:        {}", result.final_phase);
        println!("  Trust Score:  {:.3}", result.final_trust_score);
        println!("  Accuracy:     {:.3}", result.final_model_accuracy);
        println!("  Transitions:  {}", result.phase_transitions);
        println!("  Interventions: {}", result.interventions);
        println!();
        println!("  ── Round Log (first 5 + last 5) ──");
        println!(
            "  {:>4} │ {:<18} │ {:>6} │ {:>6} │ {:>6} │ {:>5}",
            "Rnd", "Name", "Events", "Acc", "Anom", "Trust"
        );
        println!(
            "  {:-<4}─┼─{:-<18}─┼─{:-<6}─┼─{:-<6}─┼─{:-<6}─┼─{:-<5}",
            "", "", "", "", "", ""
        );

        let total = result.logs.len();
        let show: Vec<&SimLog> = if total <= 10 {
            result.logs.iter().collect()
        } else {
            let mut v: Vec<&SimLog> = result.logs.iter().take(5).collect();
            v.push(&result.logs[total / 2]);
            v.extend(result.logs.iter().rev().take(4).rev());
            v
        };

        for log in show {
            println!(
                "  {:>4} │ {:<18} │ {:>6} │ {:>6.3} │ {:>6.3} │ {:>5.3}",
                log.round,
                log.round_name,
                log.events_in_round,
                log.model_accuracy,
                log.anomaly_prob,
                log.trust_score,
            );
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_scenario_runs() {
        let sandbox = Sandbox::new("test-baseline", 0.05).with_log_channel(&Scenario::baseline());
        let result = sandbox.run_scenario(&Scenario::baseline()).unwrap();
        assert!(result.rounds_completed > 0);
        assert!(result.total_events_processed > 0);
    }

    #[test]
    fn test_anomaly_scenario_runs() {
        let sandbox = Sandbox::new("test-anomaly", 0.05)
            .with_log_channel(&Scenario::anomaly_detection())
            .with_metrics_channel();
        let result = sandbox
            .run_scenario(&Scenario::anomaly_detection())
            .unwrap();
        assert!(result.rounds_completed > 0);
        assert!(result.total_events_processed > 0);
    }

    #[test]
    fn test_phase_transition_scenario() {
        let sandbox = Sandbox::new("test-transition", 0.05)
            .with_log_channel(&Scenario::phase_transition())
            .with_shell_strategy();
        let result = sandbox.run_scenario(&Scenario::phase_transition()).unwrap();
        assert!(result.rounds_completed > 0);
    }

    #[test]
    fn test_full_lifecycle_scenario() {
        let sandbox = Sandbox::new("test-lifecycle", 0.05)
            .with_log_channel(&Scenario::full_lifecycle())
            .with_metrics_channel()
            .with_shell_strategy();
        let result = sandbox.run_scenario(&Scenario::full_lifecycle()).unwrap();
        assert!(result.rounds_completed > 0);
        assert!(result.total_events_processed >= 200);
    }

    #[test]
    fn test_normal_events_have_valid_structure() {
        let events = generate_normal_events(10, 0);
        assert_eq!(events.len(), 10);
        for e in &events {
            assert!(!e.source.is_empty());
            assert!(e.payload.get("latency_ms").is_some());
        }
    }

    #[test]
    fn test_anomaly_events_have_high_latency() {
        let events = generate_anomaly_events(5);
        for e in &events {
            let latency = e
                .payload
                .get("latency_ms")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            assert!(
                latency >= 5000,
                "anomaly latency should be high, got {}",
                latency
            );
        }
    }

    #[test]
    fn test_recovery_events_mark_recovered() {
        let events = generate_recovery_events(10);
        for e in &events {
            let recovered = e
                .payload
                .get("recovered")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            assert!(recovered, "recovery events should be marked recovered");
        }
    }
}
