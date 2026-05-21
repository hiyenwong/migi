//! migi-sim — 沙盒模拟运行器
//!
//! 在隔离环境中模拟宿主系统行为，测试 Migi 的完整生命周期。
//!
//! 用法:
//!   migi-sim baseline          正常基线场景
//!   migi-sim anomaly           异常检测场景（正常→异常→恢复）
//!   migi-sim transition        相变测试场景
//!   migi-sim lifecycle         完整生命周期场景
//!   migi-sim list              列出所有场景
//!   migi-sim all               运行所有场景

use migi::sandbox::{Sandbox, Scenario};
use std::time::Instant;

fn list_scenarios() {
    println!("Available scenarios:");
    println!("  baseline     — Normal baseline, build system model");
    println!("  anomaly      — Normal → Anomaly spike → Recovery");
    println!("  transition   — Phase transition trigger test");
    println!("  lifecycle    — Full lifecycle: observe→learn→intervene→transition");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: migi-sim <scenario>");
        println!();
        list_scenarios();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "list" => {
            list_scenarios();
        }
        "baseline" | "normal" => {
            run_scenario("baseline", &Scenario::baseline());
        }
        "anomaly" | "anomaly-detection" => {
            run_scenario("anomaly-detection", &Scenario::anomaly_detection());
        }
        "transition" | "phase-transition" => {
            run_scenario("phase-transition", &Scenario::phase_transition());
        }
        "lifecycle" | "full-lifecycle" => {
            run_scenario("full-lifecycle", &Scenario::full_lifecycle());
        }
        "all" | "full" => {
            let scenarios = [
                ("baseline", Scenario::baseline()),
                ("anomaly-detection", Scenario::anomaly_detection()),
                ("phase-transition", Scenario::phase_transition()),
                ("full-lifecycle", Scenario::full_lifecycle()),
            ];
            for (name, scenario) in &scenarios {
                run_scenario(name, scenario);
            }
        }
        _ => {
            eprintln!("Unknown scenario: {}", args[1]);
            list_scenarios();
            std::process::exit(1);
        }
    }
}

fn run_scenario(name: &str, scenario: &Scenario) {
    println!(
        "\n  🚀 Running scenario: {} — {}",
        name, scenario.description
    );

    let start = Instant::now();

    let sandbox = Sandbox::new("sim", 0.05)
        .with_log_channel(scenario)
        .with_metrics_channel()
        .with_shell_strategy();

    let result = match sandbox.run_scenario(scenario) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("\n  ❌ Scenario failed: {}\n", e);
            return;
        }
    };

    let elapsed = start.elapsed();

    Sandbox::print_summary(&result);

    println!("  Duration:     {:?}", elapsed);
    println!();
}
