// Env Manager Lite - Rust Validator
// 编译: rustc examples/validate.rs -o target/validate && target/validate

use std::env;
use std::fs;
use std::process;

fn main() {
    let required = load_required_vars();

    println!("==============================================");
    println!("  Env Manager Lite - Rust Validator");
    println!("==============================================");
    println!("  Checking {} variable(s)...", required.len());
    println!();

    let mut missing: Vec<&String> = Vec::new();

    for var in &required {
        match env::var(var) {
            Ok(val) => {
                let display = if val.len() > 8 {
                    format!("{}...", &val[..4])
                } else {
                    val.clone()
                };
                println!("  ✓ Found: {} = {}", var, display);
            }
            Err(_) => {
                missing.push(var);
                println!("  ✗ Missing: {}", var);
            }
        }
    }

    println!();
    if !missing.is_empty() {
        println!("  ✗ Failed: {} variable(s) missing.", missing.len());
        for m in &missing {
            println!("    - {}", m);
        }
        process::exit(1);
    } else {
        println!("  ✓ All required variables present!");
    }
}

fn load_required_vars() -> Vec<String> {
    let default = vec!["DB_HOST".to_string(), "API_KEY".to_string()];
    let path = "examples/.env-required";
    let alt_path = ".env-required";

    let content = if let Ok(c) = fs::read_to_string(path) {
        c
    } else if let Ok(c) = fs::read_to_string(alt_path) {
        c
    } else {
        return default;
    };

    let vars: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('=').next().unwrap().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if vars.is_empty() { default } else { vars }
}
