//! Migi — 共生型 AI Agent
//!
//! "寄生"而不"接管"。观察宿主系统，学习其行为模式，
//! 在必要时局部介入，最终实现受控相变。

use migi::config::MigiConfig;
use migi::error::MigiResult;
use migi::intervener::{Intervener, Intervention, InterventionTrigger};
use migi::learner::{Learner, StatisticalLearner};
use migi::monitor::{MigiSnapshot, MonitorHistory};
use migi::observer::{LogObserver, MetricsObserver, Observer};
use migi::trust::TrustManager;
use std::path::Path;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> MigiResult<()> {
    // 初始化结构化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // 加载配置
    let config_path = Path::new("config/migi.toml");
    let config = MigiConfig::load_or_default(config_path);

    tracing::info!(
        name = %config.name,
        phase = ?config.phase,
        version = env!("CARGO_PKG_VERSION"),
        "Migi starting"
    );

    // 初始化感知层
    let mut observer = Observer::new();

    // 注册日志观察者
    for endpoint in &config.host_observation_endpoints {
        let log_observer = LogObserver::new(endpoint);
        observer.register_channel(log_observer);
        tracing::info!(endpoint = %endpoint, "registered LogObserver");
    }

    // 注册指标观察者（如果有 HTTP 端点）
    let metrics_observer = MetricsObserver::new("http://localhost:9090/metrics", 30, 80.0);
    observer.register_channel(metrics_observer);

    // 初始化认知层
    let mut learner = StatisticalLearner::new();

    // 初始化信任层
    let mut trust = TrustManager::new(
        config.phase,
        config.trust_threshold,
        config.allowed_intervention_targets.clone(),
    );

    // 初始化行动层
    let mut intervener = Intervener::new();
    intervener.register_strategy(migi::intervener::ShellInterventionStrategy::new());

    // 初始化监测
    let state_path = Path::new("var/migi-state.json");
    let history_path = Path::new("var/migi-history.json");
    let start_time = std::time::Instant::now();
    let mut history = MonitorHistory::new(100);
    let mut snapshot_counter: u64 = 0;
    std::fs::create_dir_all("var").ok();

    tracing::info!(channels = 2, "all layers initialized");

    // 主事件循环
    let mut poll_interval = time::interval(Duration::from_secs(5));

    loop {
        poll_interval.tick().await;

        // 1. 感知：轮询事件
        let events = match observer.poll_events().await {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(error = %e, "observer poll failed");
                continue;
            }
        };

        if events.is_empty() {
            continue;
        }

        tracing::info!(
            count = events.len(),
            total_observed = observer.event_count(),
            "events collected"
        );

        // 2. 认知：处理事件，更新模型
        if let Err(e) = learner.process_events(&events) {
            tracing::warn!(error = %e, "learner process_events failed");
            continue;
        }

        let model = learner.get_model();
        tracing::debug!(
            version = model.version,
            accuracy = model.prediction_accuracy,
            patterns = model.identified_patterns,
            subsystems = model.identified_subsystems,
            "model updated"
        );

        // 3. 预测：检查是否有异常需要介入
        let predictions = match learner.predict() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "learner predict failed");
                continue;
            }
        };

        if predictions.anomaly_probability > 0.7 {
            tracing::warn!(
                anomaly_prob = predictions.anomaly_probability,
                "high anomaly detected — triggering intervention"
            );

            // 构造介入请求
            let intervention = Intervention {
                id: uuid::Uuid::new_v4(),
                trigger: InterventionTrigger::PredictedAnomaly,
                target: "system".to_string(),
                action: migi::intervener::Action::Diagnose {
                    command: "echo 'diagnosing anomaly'".into(),
                },
                executed: false,
                rollbackable: false,
                rollback_action: None,
            };

            // 信任层授权检查
            if let Err(e) = trust.authorize(&intervention) {
                tracing::warn!(error = %e, "intervention rejected by trust layer");
            } else {
                // 执行介入
                match intervener.attempt(&intervention).await {
                    Ok(result) => {
                        tracing::info!(
                            id = %result.intervention_id,
                            success = result.success,
                            "intervention completed"
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "intervention failed");
                    }
                }
            }
        }

        // 4. 信任：评估相变条件
        match trust.evaluate_transition(model) {
            Ok(Some(new_phase)) => {
                if let Err(e) = trust.transition(new_phase) {
                    tracing::error!(error = %e, "phase transition failed");
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = %e, "phase transition evaluation failed");
            }
        }

        // 5. 持久化信任状态（定期）
        if observer.event_count() % 100 == 0 {
            let _ = trust.save_state();
        }

        // 6. 监测：写入状态快照（每 5 轮）
        snapshot_counter += 1;
        if snapshot_counter % 5 == 0 {
            let model = learner.get_model();
            let snap = MigiSnapshot::take(trust.phase(), trust.state(), model, start_time);
            let _ = snap.save(state_path);
            history.push(snap);
            let _ = history.save(history_path);
        }
    }
}
