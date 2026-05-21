//! migi-sim — 沙盒模拟运行器

use migi::sandbox::{Sandbox, Scenario};
use std::path::Path;
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
        println!("Usage: migi-sim [--monitor] <scenario>");
        println!();
        list_scenarios();
        std::process::exit(1);
    }

    let monitor = args[1] == "--monitor" || args[2..].contains(&"--monitor".to_string());
    let scenario_name = if args[1] == "--monitor" || args[1] == "--state-file" {
        args[2..]
            .iter()
            .find(|a| !a.starts_with("--"))
            .cloned()
            .unwrap_or_else(|| "baseline".into())
    } else {
        args[1].clone()
    };

    match scenario_name.as_str() {
        "list" => list_scenarios(),
        "baseline" | "normal" => {
            run_scenario("baseline", &Scenario::baseline(), monitor);
        }
        "anomaly" | "anomaly-detection" => {
            run_scenario("anomaly-detection", &Scenario::anomaly_detection(), monitor);
        }
        "transition" | "phase-transition" => {
            run_scenario("phase-transition", &Scenario::phase_transition(), monitor);
        }
        "lifecycle" | "full-lifecycle" => {
            run_scenario("full-lifecycle", &Scenario::full_lifecycle(), monitor);
        }
        "all" | "full" => {
            let scenarios = [
                ("baseline", Scenario::baseline()),
                ("anomaly-detection", Scenario::anomaly_detection()),
                ("phase-transition", Scenario::phase_transition()),
                ("full-lifecycle", Scenario::full_lifecycle()),
            ];
            for (name, scenario) in &scenarios {
                run_scenario(name, scenario, monitor);
            }
        }
        _ => {
            eprintln!("Unknown scenario: {}", scenario_name);
            list_scenarios();
            std::process::exit(1);
        }
    }
}

fn run_scenario(name: &str, scenario: &Scenario, monitor: bool) {
    println!(
        "\n  🚀 Running scenario: {} — {}",
        name, scenario.description
    );

    let start = Instant::now();

    let state_path = if monitor {
        Some(Path::new("var/migi-state.json"))
    } else {
        None
    };
    let history_path = if monitor {
        Some(Path::new("var/migi-history.json"))
    } else {
        None
    };

    if monitor {
        let _ = std::fs::create_dir_all("var");
    }

    let sandbox = Sandbox::new("sim", 0.05)
        .with_log_channel(scenario)
        .with_metrics_channel()
        .with_shell_strategy();

    let result = match sandbox.run_scenario_with_monitor(scenario, state_path, history_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("\n  ❌ Scenario failed: {}\n", e);
            return;
        }
    };

    let elapsed = start.elapsed();
    Sandbox::print_summary(&result);
    println!("  Duration:     {:?}", elapsed);

    if monitor {
        println!("  💾 State snapshot written to var/migi-state.json");
        println!("  💾 History written to var/migi-history.json");
    }
    println!();
}
