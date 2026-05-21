//! migi-stats — Migi 监测工具
//!
//! 读取运行中 Migi 写入的状态快照并展示。
//!
//! 用法:
//!   migi-stats               显示当前状态
//!   migi-stats --watch       持续监听（每 2s 刷新）
//!   migi-stats --history     显示历史趋势
//!   migi-stats --json        以 JSON 格式输出

use migi::monitor::{MigiSnapshot, MonitorHistory};
use std::path::Path;
use std::time::Duration;

const DEFAULT_STATE_FILE: &str = "var/migi-state.json";
const DEFAULT_HISTORY_FILE: &str = "var/migi-history.json";

fn print_usage() {
    eprintln!(
        r#"migi-stats — Migi Monitoring Tool

USAGE:
  migi-stats              Show current state snapshot
  migi-stats --watch      Watch mode (refresh every 2s)
  migi-stats --history    Show trend history
  migi-stats --json       Output raw JSON

ENVIRONMENT:
  MIGI_STATE_FILE     State snapshot path (default: var/migi-state.json)
  MIGI_HISTORY_FILE   History file path   (default: var/migi-history.json)
"#
    );
}

fn get_state_path() -> String {
    std::env::var("MIGI_STATE_FILE").unwrap_or_else(|_| DEFAULT_STATE_FILE.to_string())
}

fn get_history_path() -> String {
    std::env::var("MIGI_HISTORY_FILE").unwrap_or_else(|_| DEFAULT_HISTORY_FILE.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // If not inside the project dir, expand from $HOME/ai_github/migi
    let state_path = get_state_path();
    let history_path = get_history_path();

    let state_file = Path::new(&state_path);
    let history_file = Path::new(&history_path);

    let watch = args.get(1).map(|s| s.as_str()) == Some("--watch");
    let history = args.get(1).map(|s| s.as_str()) == Some("--history");
    let json_mode = args.get(1).map(|s| s.as_str()) == Some("--json");

    if args.get(1).map(|s| s.as_str()) == Some("--help")
        || args.get(1).map(|s| s.as_str()) == Some("-h")
    {
        print_usage();
        return;
    }

    if history {
        match MonitorHistory::load(history_file) {
            Ok(h) => {
                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&h).unwrap());
                } else {
                    println!(
                        "📈 Migi History ({} snapshots, cap: {})",
                        h.snapshots.len(),
                        h.capacity
                    );
                    println!("{:-<60}", "");
                    for snap in &h.snapshots {
                        println!("  {}", snap.summary());
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to load history from {}: {}",
                    history_file.display(),
                    e
                );
                std::process::exit(1);
            }
        }
        return;
    }

    if watch {
        println!("🔍 Watching {} (Ctrl+C to stop)", state_file.display());
        let mut prev: Option<String> = None;
        loop {
            match MigiSnapshot::load(state_file) {
                Ok(snap) => {
                    let text = snap.format();
                    // Only redraw if state changed
                    if Some(&text) != prev.as_ref() {
                        // Clear screen
                        print!("\x1B[2J\x1B[H");
                        println!("{}", text);
                        prev = Some(text);
                    }
                }
                Err(e) => {
                    print!("\x1B[2J\x1B[H");
                    println!("⏳ Waiting for Migi to write state snapshot...");
                    println!("   (expected at {})", state_file.display());
                    println!("   error: {}", e);
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    // Default: single snapshot
    match MigiSnapshot::load(state_file) {
        Ok(snap) => {
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&snap).unwrap());
            } else {
                println!("{}", snap.format());
            }
        }
        Err(e) => {
            eprintln!("❌ No Migi state found at {}", state_file.display());
            eprintln!("   Make sure Migi is running, or check MIGI_STATE_FILE.");
            eprintln!("   Error: {}", e);
            std::process::exit(1);
        }
    }
}
