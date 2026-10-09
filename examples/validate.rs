// Env Manager Lite - Rust Validator v0.7
// 三种规则类型: required / match / default
// 规则分层: .env-required.d/*.conf 按字母序追加/覆盖
// .env 文件加载: ENV_VALIDATE_ENV_FILE
// JSON 报告: 设置 ENV_VALIDATE_REPORT 后生成
//
// 编译: rustc examples/validate.rs -o target/validate && target/validate

use std::env;
use std::fs;
use std::path::PathBuf;

const VERSION: &str = "0.7.0";

#[derive(Clone, Debug)]
struct Rule {
    rule_type: String,
    expected: Option<String>,
    default_val: Option<String>,
}

#[derive(Clone, Debug)]
struct RuleResult {
    var: String,
    status: String,
    value: Option<String>,
}

#[derive(Clone, Debug)]
struct RuleError {
    var: String,
    issue: String,
    expected: Option<String>,
    got: Option<String>,
}

fn find_root() -> PathBuf {
    let mut dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join(".env-required").is_file() || dir.join("examples").is_dir() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from(".")
}

// 解析单行: 返回 (var, Rule) 或 None
fn parse_rule_line(line: &str) -> Option<(String, Rule)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    if line.contains(':') && !line.contains('=') {
        let mut parts = line.splitn(2, ':');
        let var_name = parts.next()?.trim().to_string();
        let default_val = parts.next().unwrap_or("").trim().to_string();
        if var_name.is_empty() {
            return None;
        }
        Some((
            var_name,
            Rule {
                rule_type: "default".into(),
                expected: None,
                default_val: Some(default_val),
            },
        ))
    } else if line.contains('=') {
        let idx = line.find('=')?;
        let var_name = line[..idx].trim().to_string();
        let expected = line[idx + 1..].trim().to_string();
        if var_name.is_empty() {
            return None;
        }
        Some((
            var_name,
            Rule {
                rule_type: "match".into(),
                expected: Some(expected),
                default_val: None,
            },
        ))
    } else {
        let var_name = line.to_string();
        if var_name.is_empty() {
            return None;
        }
        Some((
            var_name,
            Rule {
                rule_type: "required".into(),
                expected: None,
                default_val: None,
            },
        ))
    }
}

// 加载规则: 主文件 + 分层目录（按字母序，后加载的覆盖）
fn load_rules() -> Vec<(String, Rule)> {
    let root = find_root();
    let mut rules: std::collections::HashMap<String, Rule> = std::collections::HashMap::new();
    let mut found = false;

    // 主文件
    let config_path = root.join(".env-required");
    if config_path.is_file() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            for line in content.lines() {
                if let Some((var_name, rule)) = parse_rule_line(line) {
                    rules.insert(var_name, rule);
                    found = true;
                }
            }
        }
    }

    // 分层目录 .env-required.d/*.conf
    let rules_dir = root.join(".env-required.d");
    if rules_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&rules_dir) {
            let mut conf_files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map(|n| n.to_string_lossy().ends_with(".conf"))
                        .unwrap_or(false)
                })
                .collect();
            conf_files.sort_by_key(|e| e.file_name());
            for entry in conf_files {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        if let Some((var_name, rule)) = parse_rule_line(line) {
                            rules.insert(var_name, rule);
                            found = true;
                        }
                    }
                }
            }
        }
    }

    if !found {
        vec![
            ("DB_HOST".into(), Rule { rule_type: "required".into(), expected: None, default_val: None }),
            ("API_KEY".into(), Rule { rule_type: "required".into(), expected: None, default_val: None }),
        ]
    } else {
        rules.into_iter().collect()
    }
}

// 加载 .env 文件（不覆盖已存在的变量）
fn load_env_file(env_file: &str) -> usize {
    if env_file.is_empty() || !PathBuf::from(env_file).is_file() {
        return 0;
    }
    let Ok(content) = fs::read_to_string(env_file) else {
        return 0;
    };
    let mut imported = 0;
    for raw in content.lines() {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("export ") {
            line = &line[7..];
        }
        let Some(idx) = line.find('=') else {
            continue;
        };
        let key = line[..idx].trim().to_string();
        let mut value = line[idx + 1..].trim();
        if (value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"'))
        {
            value = &value[1..value.len() - 1];
        }
        if !key.is_empty() && env::var(&key).is_err() {
            env::set_var(&key, value);
            imported += 1;
        }
    }
    imported
}

// 使用 std 手动序列化 JSON，避免引入 serde
fn to_json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")
}

fn to_json_report(results: &[RuleResult], errors: &[RuleError], mode: &str) -> String {
    let results_json: Vec<String> = results
        .iter()
        .map(|r| {
            let value = r
                .value
                .as_ref()
                .map(|v| format!("\"{}\"", to_json_escape(v)))
                .unwrap_or_else(|| "null".into());
            format!(
                "{{\"var\":\"{}\",\"status\":\"{}\",\"value\":{}}}",
                to_json_escape(&r.var),
                to_json_escape(&r.status),
                value
            )
        })
        .collect();

    let errors_json: Vec<String> = errors
        .iter()
        .map(|e| {
            let expected = e
                .expected
                .as_ref()
                .map(|v| format!("\"expected\":\"{}\"", to_json_escape(v)))
                .unwrap_or_default();
            let got = e
                .got
                .as_ref()
                .map(|v| format!(",\"got\":\"{}\"", to_json_escape(v)))
                .unwrap_or_default();
            let expected_field = if expected.is_empty() {
                expected.clone()
            } else {
                format!("\"{}\"", to_json_escape(&e.expected.clone().unwrap_or_default()))
            };
            let got_field = if let Some(g) = &e.got {
                format!("\"got\":\"{}\"", to_json_escape(g))
            } else {
                "null".into()
            };
            format!(
                "{{\"var\":\"{}\",\"issue\":\"{}\",\"expected\":{}{},\"got\":{}}}",
                to_json_escape(&e.var),
                to_json_escape(&e.issue),
                expected_field,
                if expected_field.is_empty() { String::new() } else { ", ".into() },
                got_field
            )
        })
        .collect();

    format!(
        "{{\"tool\":\"env-manager-lite\",\"version\":\"{}\",\"mode\":\"{}\",\"passed\":{},\"total\":{},\"passed_count\":{},\"failed_count\":{},\"results\":[{}],\"errors\":[{}]}}",
        VERSION,
        mode,
        errors.is_empty(),
        results.len(),
        results.len().saturating_sub(errors.len()),
        errors.len(),
        results_json.join(","),
        errors_json.join(",")
    )
}

fn main() {
    let is_ci = env::var("GITHUB_ACTIONS").map(|v| v == "true").unwrap_or(false);
    let mode = if is_ci { "CI" } else { "Local" };

    println!("==============================================");
    println!("  Env Manager Lite - Rust Validator v{}", VERSION);
    println!("  Mode: {}", mode);
    println!("==============================================");

    // 加载 .env 文件
    let env_file = env::var("ENV_VALIDATE_ENV_FILE").unwrap_or_default();
    if !env_file.is_empty() {
        let imported = load_env_file(&env_file);
        println!("  📦 Loaded .env file: {} ({} variable(s) imported)", env_file, imported);
    }

    let rules = load_rules();
    let type_counts: std::collections::HashMap<String, usize> = rules
        .iter()
        .fold(std::collections::HashMap::new(), |mut m, (_, r)| {
            *m.entry(r.rule_type.clone()).or_insert(0) += 1;
            m
        });

    let required_count = type_counts.get("required").copied().unwrap_or(0);
    let match_count = type_counts.get("match").copied().unwrap_or(0);
    let default_count = type_counts.get("default").copied().unwrap_or(0);

    println!(
        "  Checking {} rule(s): {} required, {} match, {} default...",
        rules.len(),
        required_count,
        match_count,
        default_count
    );
    println!();

    let mut results: Vec<RuleResult> = Vec::new();
    let mut errors: Vec<RuleError> = Vec::new();

    for (var_name, rule) in &rules {
        let current = env::var(var_name).unwrap_or_default();

        match rule.rule_type.as_str() {
            "default" => {
                let default_val = rule.default_val.clone().unwrap_or_default();
                if current.is_empty() {
                    env::set_var(var_name, &default_val);
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "defaulted".into(),
                        value: Some(default_val.clone()),
                    });
                    println!("  ◑ Defaulted: {} = {}", var_name, default_val);
                } else {
                    let display: String = if current.len() > 8 {
                        format!("{}...", &current[..4])
                    } else {
                        current.clone()
                    };
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Found: {} = {}", var_name, display);
                }
            }
            "match" => {
                let expected = rule.expected.clone().unwrap_or_default();
                if current.is_empty() {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "missing".into(),
                        expected: None,
                        got: None,
                    });
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "missing".into(),
                        value: None,
                    });
                    println!("  ✗ Missing: {} (required)", var_name);
                } else if current != expected {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "mismatch".into(),
                        expected: Some(expected.clone()),
                        got: Some(current.clone()),
                    });
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "mismatch".into(),
                        value: None,
                    });
                    println!(
                        "  ✗ Mismatch: {} (expected '{}', got '{}')",
                        var_name, expected, current
                    );
                } else {
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Match: {} = {}", var_name, current);
                }
            }
            _ => {
                if current.is_empty() {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "missing".into(),
                        expected: None,
                        got: None,
                    });
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "missing".into(),
                        value: None,
                    });
                    println!("  ✗ Missing: {}", var_name);
                } else {
                    let display: String = if current.len() > 8 {
                        format!("{}...", &current[..4])
                    } else {
                        current.clone()
                    };
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Found: {} = {}", var_name, display);
                }
            }
        }
    }

    // JSON 报告
    let report_path = env::var("ENV_VALIDATE_REPORT").unwrap_or_default();
    if !report_path.is_empty() {
        let json = to_json_report(&results, &errors, mode);
        if let Some(parent) = PathBuf::from(&report_path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&report_path, &json) {
            eprintln!("  ⚠ Failed to write JSON report: {}", e);
        } else {
            println!("  📄 Report written to: {}", report_path);
        }
    }

    println!();
    if !errors.is_empty() {
        println!("  ✗ Failed: {} issue(s).", errors.len());
        let missing: Vec<_> = errors.iter().filter(|e| e.issue == "missing").map(|e| e.var.clone()).collect();
        let mismatched: Vec<_> = errors.iter().filter(|e| e.issue == "mismatch").map(|e| e.var.clone()).collect();
        if !missing.is_empty() {
            println!("    Missing: {}", missing.join(", "));
        }
        if !mismatched.is_empty() {
            println!("    Mismatch: {}", mismatched.join(", "));
        }
        std::process::exit(1);
    } else {
        println!("  ✓ All checks passed!");
        std::process::exit(0);
    }
}
