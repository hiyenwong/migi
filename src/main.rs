//! Migi — 共生型 AI Agent
//!
//! "寄生而不接管"。观察宿主系统，学习行为模式，在必要时局部介入。

use migi::config::MigiConfig;
use migi::error::MigiResult;
use migi::learner::{Learner, StatisticalLearner};
use migi::trust::TrustManager;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> MigiResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = MigiConfig::default();
    tracing::info!(name = %config.name, phase = ?config.phase, "Migi starting");

    let learner = StatisticalLearner::new();
    let mut trust = TrustManager::new(
        config.phase,
        config.trust_threshold,
        config.allowed_intervention_targets.clone(),
    );

    tracing::info!("Migi initialized in Observation phase");
    tracing::info!("Currently no host endpoints configured — this is a skeleton.");
    tracing::info!("Next: implement ObservationChannel for real host systems.");

    let state = trust.state();
    tracing::info!(
        phase = ?state.phase,
        trust_score = state.trust_score,
        "current trust state"
    );

    let model = learner.get_model();
    if let Some(next) = trust.evaluate_transition(model)? {
        if next != state.phase {
            trust.transition(next)?;
        }
    }

    tracing::info!("Migi idle. Awaiting host connection.");
    Ok(())
}
