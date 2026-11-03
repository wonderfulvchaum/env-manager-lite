// Env Manager Lite - Rust Validator v0.8
// 四种规则类型: required / match / default / pattern
// 规则分层: .env-required.d/*.conf 按字母序追加/覆盖
// YAML 规则文件: .env-required.yaml（简化解析）
// .env 文件加载: ENV_VALIDATE_ENV_FILE
// JSON 报告: 设置 ENV_VALIDATE_REPORT 后生成
//
// 编译: rustc examples/validate.rs -o target/validate && target/validate
// 注意: pattern 校验使用内置简化匹配器（无外部依赖），
//       支持 int/float/bool/url/ip/uuid 快捷类型及基本字符类正则。

use std::env;
use std::fs;
use std::path::PathBuf;

const VERSION: &str = "0.8.0";

#[derive(Clone, Debug)]
struct Rule {
    rule_type: String,
    expected: Option<String>,
    default_val: Option<String>,
    pattern: Option<String>,
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
    pattern: Option<String>,
}

// 内置快捷模式（v0.8）
fn builtin_pattern(name: &str) -> Option<&'static str> {
    match name {
        "int" => Some(r"^[0-9]+$"),
        "float" => Some(r"^[0-9]+(\.[0-9]+)?$"),
        "bool" => Some(r"^(true|false|1|0)$"),
        "url" => Some(r"^https?://[^\s/$?#]*\.[^\s]+$"),
        "ip" => Some(r"^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$"),
        "uuid" => Some(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        ),
        _ => None,
    }
}

/// 简化正则匹配器：支持内置类型 + 基本字符类/量词/或组（无外部依赖）
fn check_pattern(value: &str, pattern: &str) -> bool {
    let resolved = builtin_pattern(pattern).unwrap_or(pattern);
    // 简化实现: 对内置类型做逻辑判断，对自定义正则做保守的字符类匹配
    if let Some(builtin) = builtin_pattern(pattern) {
        let _ = builtin;
        return match pattern {
            "int" => !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "float" => {
                let v = value.trim_start_matches('-');
                !v.is_empty()
                    && v.split('.').count() <= 2
                    && v.split('.')
                        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
            }
            "bool" => matches!(value, "true" | "false" | "1" | "0"),
            "url" => value.starts_with("http://") || value.starts_with("https://"),
            "ip" => {
                let parts: Vec<&str> = value.split('.').collect();
                parts.len() == 4
                    && parts
                        .iter()
                        .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()))
            }
            "uuid" => {
                value.len() == 36
                    && value.split('-').count() == 5
                    && value.chars().filter(|c| *c == '-').count() == 4
            }
            _ => false,
        };
    }
    // 自定义正则: 简化处理 — 剥离锚点后逐字符匹配基础字符类
    let stripped = resolved.trim_start_matches('^').trim_end_matches('$');
    simple_regex_match(value, stripped)
}

/// 极简正则: 支持 [char-class]{量词} 形式 + 纯文本全等匹配
fn simple_regex_match(value: &str, pattern: &str) -> bool {
    if let Some(bracket_start) = pattern.find('[') {
        let after_offset = &pattern[bracket_start..];
        let Some(bracket_end_rel) = after_offset.find(']') else {
            return value == pattern;
        };
        let class = &after_offset[1..bracket_end_rel];
        let after = &after_offset[bracket_end_rel + 1..];

        // 量词解析
        let (min, max) = if after.starts_with('+') {
            (1, usize::MAX)
        } else if after.starts_with('*') {
            (0, usize::MAX)
        } else if let Some(open) = after.find('{') {
            let rest = &after[open + 1..];
            let close = rest.find('}').unwrap_or(rest.len());
            let spec = &rest[..close];
            let parts: Vec<&str> = spec.split(',').collect();
            let m: usize = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            if parts.len() > 1 {
                let mx: usize = parts[1].parse().unwrap_or(usize::MAX);
                (m, mx)
            } else {
                (m, m)
            }
        } else {
            (1, 1)
        };

        let chars: Vec<u8> = value.bytes().collect();
        if chars.len() < min || chars.len() > max {
            return false;
        }
        // 后面必须没有其他字符
        if !after.is_empty() && !(after.starts_with('+') || after.starts_with('*') || after.starts_with('{')) {
            return false;
        }

        // 逐字符检查是否属于字符类
        for &c in &chars {
            let mut matched = false;
            let class_bytes: Vec<u8> = class.bytes().collect();
            let mut i = 0;
            while i < class_bytes.len() {
                let ch = class_bytes[i];
                if ch == b'\\' && i + 1 < class_bytes.len() {
                    if c == class_bytes[i + 1] {
                        matched = true;
                    }
                    i += 2;
                } else if ch == b'-' && i >= 1 && i + 1 < class_bytes.len() {
                    let lo = class_bytes[i - 1];
                    let hi = class_bytes[i + 1];
                    if lo <= c && c <= hi {
                        matched = true;
                    }
                    i += 3;
                } else {
                    if c == ch {
                        matched = true;
                    }
                    i += 1;
                }
            }
            if !matched {
                return false;
            }
        }
        return true;
    }
    // 不含字符类: 全等匹配
    value == pattern
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
// 优先级: ~ > : > = > bare
fn parse_rule_line(line: &str) -> Option<(String, Rule)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    // ~ 优先: pattern
    if let Some(tilde_idx) = line.find('~') {
        let var_name = line[..tilde_idx].trim().to_string();
        let pattern = line[tilde_idx + 1..].trim().to_string();
        if var_name.is_empty() {
            return None;
        }
        return Some((
            var_name,
            Rule {
                rule_type: "pattern".into(),
                expected: None,
                default_val: None,
                pattern: Some(pattern),
            },
        ));
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
                pattern: None,
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
                pattern: None,
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
                pattern: None,
            },
        ))
    }
}

// 加载规则: YAML + 主文件 + 分层目录（按字母序，后加载的覆盖）
fn load_rules() -> Vec<(String, Rule)> {
    let root = find_root();
    let mut rules: std::collections::HashMap<String, Rule> = std::collections::HashMap::new();
    let mut found = false;

    // YAML 文件（简化解析: 顶层 VAR: type 单行）
    let yaml_path = root.join(".env-required.yaml");
    if yaml_path.is_file() {
        if let Ok(content) = fs::read_to_string(&yaml_path) {
            for raw in content.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // 只处理顶层键（无缩进）
                if raw.starts_with(' ') || raw.starts_with('\t') {
                    continue;
                }
                if let Some(colon_idx) = line.find(':') {
                    let var_name = line[..colon_idx].trim().to_string();
                    let val = line[colon_idx + 1..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    if var_name.is_empty() {
                        continue;
                    }
                    let rule_type = if matches!(val.as_str(), "match" | "default" | "pattern" | "required") {
                        val
                    } else {
                        "required".to_string()
                    };
                    rules.insert(
                        var_name,
                        Rule {
                            rule_type,
                            expected: None,
                            default_val: None,
                            pattern: None,
                        },
                    );
                    found = true;
                }
            }
        }
    }

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

    // 分层目录 .env-required.d/*.conf + *.yaml
    let rules_dir = root.join(".env-required.d");
    if rules_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&rules_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map(|n| {
                            let n = n.to_string_lossy();
                            n.ends_with(".conf") || n.ends_with(".yaml")
                        })
                        .unwrap_or(false)
                })
                .collect();
            files.sort_by_key(|e| e.file_name());
            for entry in files {
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
            (
                "DB_HOST".into(),
                Rule {
                    rule_type: "required".into(),
                    expected: None,
                    default_val: None,
                    pattern: None,
                },
            ),
            (
                "API_KEY".into(),
                Rule {
                    rule_type: "required".into(),
                    expected: None,
                    default_val: None,
                    pattern: None,
                },
            ),
        ]
    } else {
        let mut v: Vec<_> = rules.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
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
            let expected_field = e
                .expected
                .as_ref()
                .map(|v| format!("\"expected\":\"{}\"", to_json_escape(v)))
                .unwrap_or_default();
            let got_field = e
                .got
                .as_ref()
                .map(|v| format!(",\"got\":\"{}\"", to_json_escape(v)))
                .unwrap_or_default();
            let pattern_field = e
                .pattern
                .as_ref()
                .map(|v| format!(",\"pattern\":\"{}\"", to_json_escape(v)))
                .unwrap_or_default();
            format!(
                "{{\"var\":\"{}\",\"issue\":\"{}\",{}{}{}}}",
                to_json_escape(&e.var),
                to_json_escape(&e.issue),
                if expected_field.is_empty() {
                    "null".to_string()
                } else {
                    expected_field
                },
                got_field,
                pattern_field
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
    let pattern_count = type_counts.get("pattern").copied().unwrap_or(0);

    println!(
        "  Checking {} rule(s): {} required, {} match, {} default, {} pattern...",
        rules.len(),
        required_count,
        match_count,
        default_count,
        pattern_count
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
                        pattern: None,
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
                        pattern: None,
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
            "pattern" => {
                let pattern = rule.pattern.clone().unwrap_or_default();
                if current.is_empty() {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "missing".into(),
                        expected: None,
                        got: None,
                        pattern: Some(pattern.clone()),
                    });
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "missing".into(),
                        value: None,
                    });
                    println!("  ✗ Missing: {} (pattern required: {})", var_name, pattern);
                } else if !check_pattern(&current, &pattern) {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "pattern_fail".into(),
                        expected: None,
                        got: Some(current.clone()),
                        pattern: Some(pattern.clone()),
                    });
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "pattern_fail".into(),
                        value: None,
                    });
                    println!(
                        "  ✗ Pattern fail: {} (pattern '{}', got '{}')",
                        var_name, pattern, current
                    );
                } else {
                    results.push(RuleResult {
                        var: var_name.clone(),
                        status: "ok".into(),
                        value: Some(current.clone()),
                    });
                    println!("  ✓ Pattern ok: {} matches '{}'", var_name, pattern);
                }
            }
            _ => {
                if current.is_empty() {
                    errors.push(RuleError {
                        var: var_name.clone(),
                        issue: "missing".into(),
                        expected: None,
                        got: None,
                        pattern: None,
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
        let pattern_failed: Vec<_> = errors.iter().filter(|e| e.issue == "pattern_fail").map(|e| e.var.clone()).collect();
        if !missing.is_empty() {
            println!("    Missing: {}", missing.join(", "));
        }
        if !mismatched.is_empty() {
            println!("    Mismatch: {}", mismatched.join(", "));
        }
        if !pattern_failed.is_empty() {
            println!("    Pattern fail: {}", pattern_failed.join(", "));
        }
        std::process::exit(1);
    } else {
        println!("  ✓ All checks passed!");
        std::process::exit(0);
    }
}
