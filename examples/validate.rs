// Env Manager Lite - Rust Validator v0.6
// 三种规则类型: required / match / default
// JSON 报告: 设置 ENV_VALIDATE_REPORT 环境变量后生成
//
// 编译: rustc examples/validate.rs -o target/validate && target/validate

use std::env;
use std::fs;
use std::process;

#[derive(Debug, Clone)]
enum RuleType {
    Required,
    Match(String),
    Default(String),
}

struct Rule {
    name: String,
    rule_type: RuleType,
}

#[derive(Debug, Clone)]
struct RuleResult {
    var: String,
    status: String,
    value: Option<String>,
}

struct RuleError {
    var: String,
    issue: String,
    expected: Option<String>,
    got: Option<String>,
}

fn main() {
    let is_ci = env::var("GITHUB_ACTIONS").unwrap_or_default() == "true";
    let mode = if is_ci { "ci" } else { "local" };

    println!("==============================================");
    println!("  Env Manager Lite - Rust Validator v0.6");
    println!("  Mode: {}", mode.to_uppercase());
    println!("==============================================");

    let rules = load_rules();

    let mut type_counts: [usize; 3] = [0; 3];
    for r in &rules {
        match r.rule_type {
            RuleType::Required => type_counts[0] += 1,
            RuleType::Match(_) => type_counts[1] += 1,
            RuleType::Default(_) => type_counts[2] += 1,
        }
    }
    println!(
        "  Checking {} rule(s): {} required, {} match, {} default...\n",
        rules.len(),
        type_counts[0],
        type_counts[1],
        type_counts[2]
    );

    let (results, errors) = validate(&rules);
    write_report(&results, &errors, mode);

    println!();
    if !errors.is_empty() {
        println!("  ✗ Failed: {} issue(s).", errors.len());
        let missing: Vec<&str> = errors.iter().filter(|e| e.issue == "missing").map(|e| e.var.as_str()).collect();
        let mismatched: Vec<&str> = errors.iter().filter(|e| e.issue == "mismatch").map(|e| e.var.as_str()).collect();
        if !missing.is_empty() {
            println!("    Missing: {}", missing.join(", "));
        }
        if !mismatched.is_empty() {
            println!("    Mismatch: {}", mismatched.join(", "));
        }
        process::exit(1);
    }
    println!("  ✓ All checks passed!");
}

fn load_rules() -> Vec<Rule> {
    let default_rules = vec![
        Rule { name: "DB_HOST".into(), rule_type: RuleType::Required },
        Rule { name: "API_KEY".into(), rule_type: RuleType::Required },
    ];

    let path = find_config();
    let content = match path {
        Some(p) => match fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => return default_rules,
        },
        None => return default_rules,
    };

    let mut rules = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // 优先级: 有 : 且无 = → default; 有 = → match; 否则 → required
        if line.contains(':') && !line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            let name = parts[0].trim().to_string();
            let default_val = parts.get(1).unwrap_or(&"").trim().to_string();
            rules.push(Rule {
                name: name.clone(),
                rule_type: RuleType::Default(default_val),
            });
        } else if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            let name = parts[0].trim().to_string();
            let expected = parts.get(1).unwrap_or(&"").trim().to_string();
            rules.push(Rule {
                name: name.clone(),
                rule_type: RuleType::Match(expected),
            });
        } else {
            rules.push(Rule {
                name: line.to_string(),
                rule_type: RuleType::Required,
            });
        }
    }

    if rules.is_empty() {
        default_rules
    } else {
        rules
    }
}

fn find_config() -> Option<String> {
    let candidates = [".env-required", "../.env-required"];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return Some(c.to_string());
        }
    }
    None
}

fn validate(rules: &[Rule]) -> (Vec<RuleResult>, Vec<RuleError>) {
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for rule in rules {
        let current = env::var(&rule.name).unwrap_or_default();

        match &rule.rule_type {
            RuleType::Default(default_val) => {
                if current.is_empty() {
                    env::set_var(&rule.name, default_val);
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "defaulted".into(),
                        value: Some(default_val.clone()),
                    });
                    println!("  ◑ Defaulted: {} = {}", rule.name, default_val);
                } else {
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Found: {} = {}", rule.name, mask(&current));
                }
            }
            RuleType::Match(expected) => {
                if current.is_empty() {
                    errors.push(RuleError {
                        var: rule.name.clone(),
                        issue: "missing".into(),
                        expected: Some(expected.clone()),
                        got: None,
                    });
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "missing".into(),
                        value: None,
                    });
                    println!("  ✗ Missing: {} (required)", rule.name);
                } else if &current != expected {
                    errors.push(RuleError {
                        var: rule.name.clone(),
                        issue: "mismatch".into(),
                        expected: Some(expected.clone()),
                        got: Some(current.clone()),
                    });
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "mismatch".into(),
                        value: None,
                    });
                    println!(
                        "  ✗ Mismatch: {} (expected '{}', got '{}')",
                        rule.name, expected, current
                    );
                } else {
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Match: {} = {}", rule.name, current);
                }
            }
            RuleType::Required => {
                if current.is_empty() {
                    errors.push(RuleError {
                        var: rule.name.clone(),
                        issue: "missing".into(),
                        expected: None,
                        got: None,
                    });
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "missing".into(),
                        value: None,
                    });
                    println!("  ✗ Missing: {}", rule.name);
                } else {
                    results.push(RuleResult {
                        var: rule.name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Found: {} = {}", rule.name, mask(&current));
                }
            }
        }
    }

    (results, errors)
}

fn write_report(results: &[RuleResult], errors: &[RuleError], mode: &str) {
    let report_path = match env::var("ENV_VALIDATE_REPORT") {
        Ok(p) => p,
        Err(_) => return,
    };

    // 手写 JSON（避免外部 serde 依赖）
    let passed = errors.is_empty();
    let total = results.len();
    let failed = errors.len();

    let results_json: Vec<String> = results
        .iter()
        .map(|r| {
            let value_part = match &r.value {
                Some(v) => format!(", \"value\": \"{}\"", v),
                None => String::new(),
            };
            format!(
                "{{\"var\": \"{}\", \"status\": \"{}\"{}}}",
                r.var, r.status, value_part
            )
        })
        .collect();

    let errors_json: Vec<String> = errors
        .iter()
        .map(|e| {
            let extra = match (&e.expected, &e.got) {
                (Some(exp), Some(got)) => format!(", \"expected\": \"{}\", \"got\": \"{}\"", exp, got),
                (Some(exp), None) => format!(", \"expected\": \"{}\"", exp),
                _ => String::new(),
            };
            format!(
                "{{\"var\": \"{}\", \"issue\": \"{}\"{}}}",
                e.var, e.issue, extra
            )
        })
        .collect();

    let json = format!(
        "{{\n  \"tool\": \"env-manager-lite\",\n  \"version\": \"0.6.0\",\n  \"mode\": \"{}\",\n  \"passed\": {},\n  \"total\": {},\n  \"passed_count\": {},\n  \"failed_count\": {},\n  \"results\": [{}],\n  \"errors\": [{}]\n}}",
        mode,
        passed,
        total,
        total - failed,
        failed,
        results_json.join(",\n    "),
        errors_json.join(",\n    ")
    );

    if let Err(e) = fs::write(&report_path, json) {
        eprintln!("  ⚠ Failed to write report: {}", e);
    } else {
        println!("  📄 Report written to: {}", report_path);
    }
}

fn mask(v: &str) -> String {
    if v.len() > 8 {
        format!("{}...", &v[..4])
    } else {
        v.to_string()
    }
}
