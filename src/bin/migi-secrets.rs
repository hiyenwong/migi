//! migi-secrets — 加密配置管理 CLI
//!
//! 管理 Migi 的加密 secrets 文件。
//!
//! 用法:
//!   migi-secrets gen-key        生成新的 32 字节主密钥
//!   migi-secrets list           列出 secrets 文件中的 key 名称
//!   migi-secrets set <key> <val> 设置一个 secret
//!   migi-secrets get <key>      获取一个 secret（解密后输出）
//!   migi-secrets rm <key>       删除一个 secret
//!   migi-secrets init           交互式初始化 secrets 文件
//!
//! 环境变量:
//!   MIGI_MASTER_KEY    32 字节 hex 格式主密钥
//!   MIGI_SECRETS_FILE  加密 secrets 文件路径（默认 config/secrets.enc）

use migi::secrets::Secrets;
use std::path::Path;

const DEFAULT_SECRETS_FILE: &str = "config/secrets.enc";

fn get_secrets_path() -> String {
    std::env::var("MIGI_SECRETS_FILE").unwrap_or_else(|_| DEFAULT_SECRETS_FILE.to_string())
}

fn get_master_key() -> migi::error::MigiResult<Vec<u8>> {
    Secrets::master_key_from_env()
}

fn load_secrets(path: &Path, key: &[u8]) -> Secrets {
    Secrets::load_or_empty(path, key)
}

fn print_usage() {
    eprintln!(
        r#"migi-secrets — Encrypted secret management

USAGE:
  migi-secrets gen-key                    Generate a 32-byte master key (hex)
  migi-secrets list                       List secret key names
  migi-secrets set <key> <value>          Set/update a secret
  migi-secrets get <key>                  Get/decrypt a secret (stdout)
  migi-secrets rm <key>                   Remove a secret
  migi-secrets init                       Interactive initialization
  migi-secrets help                       Show this help

ENVIRONMENT:
  MIGI_MASTER_KEY    Required — 32-byte hex master key
  MIGI_SECRETS_FILE  Optional — secrets file path (default: config/secrets.enc)
"#
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let secrets_path = get_secrets_path();

    match args[1].as_str() {
        "gen-key" => {
            let key = migi::secrets::generate_master_key();
            println!("{}", key);
        }
        "list" => {
            let path = Path::new(&secrets_path);
            let key = match get_master_key() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let secrets = load_secrets(path, &key);
            for k in secrets.keys() {
                println!("{}", k);
            }
            if secrets.keys().is_empty() {
                println!("(no secrets)");
            }
        }
        "set" => {
            if args.len() < 4 {
                eprintln!("Usage: migi-secrets set <key> <value>");
                std::process::exit(1);
            }
            let path = Path::new(&secrets_path);
            let key = match get_master_key() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let mut secrets = load_secrets(path, &key);
            secrets.set(&args[2], &args[3]);
            if let Err(e) = secrets.save_to_file(path, &key) {
                eprintln!("Error saving secrets: {}", e);
                std::process::exit(1);
            }
            println!("✅ secret '{}' saved to {}", args[2], path.display());
        }
        "get" => {
            if args.len() < 3 {
                eprintln!("Usage: migi-secrets get <key>");
                std::process::exit(1);
            }
            let path = Path::new(&secrets_path);
            let key = match get_master_key() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let secrets = load_secrets(path, &key);
            match secrets.get(&args[2]) {
                Some(val) => println!("{}", val),
                None => {
                    eprintln!("secret '{}' not found", args[2]);
                    std::process::exit(1);
                }
            }
        }
        "rm" | "remove" | "delete" => {
            if args.len() < 3 {
                eprintln!("Usage: migi-secrets rm <key>");
                std::process::exit(1);
            }
            let path = Path::new(&secrets_path);
            let key = match get_master_key() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let mut secrets = load_secrets(path, &key);
            secrets.set(&args[2], "");
            if let Err(e) = secrets.save_to_file(path, &key) {
                eprintln!("Error saving secrets: {}", e);
                std::process::exit(1);
            }
            println!("✅ secret '{}' removed", args[2]);
        }
        "init" => {
            let path = Path::new(&secrets_path);
            let key = match get_master_key() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let mut secrets = Secrets::new();
            println!("🔑 Interactive secrets initialization (leave empty to skip)\n");

            let prompts = [
                "LLM API Key (e.g., sk-...)",
                "LLM Provider (openai / anthropic / custom)",
                "LLM Model (e.g., gpt-4o / claude-sonnet-4)",
            ];
            let keys = ["llm_api_key", "llm_provider", "llm_model"];

            for (i, (&prompt, &key_name)) in prompts.iter().zip(keys.iter()).enumerate() {
                println!("{}/{}: {}", i + 1, prompts.len(), prompt);
                print!("  > ");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                let val = input.trim().to_string();
                if !val.is_empty() {
                    secrets.set(key_name, &val);
                }
            }

            if secrets.keys().is_empty() {
                println!("❌ No secrets entered, nothing saved.");
            } else {
                secrets.save_to_file(path, &key).unwrap();
                println!(
                    "✅ {} secrets saved to {}",
                    secrets.keys().len(),
                    path.display()
                );
            }
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}
