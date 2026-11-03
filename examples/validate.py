"""
Env Manager Lite - Python Validator v0.8

规则文件 .env-required 语法（四种条目类型）：
  1. VAR                → 必填，缺失则失败
  2. VAR=expected       → 精确匹配，不存在则失败，存在但值不同则失败
  3. VAR:default        → 带默认值，不存在时注入 default 后通过；存在则保留原值
  4. VAR~pattern        → 正则校验（v0.8），值必须匹配 pattern（Python re 语法）
     支持内置快捷类型:
       VAR~int    → 值必须是整数
       VAR~float  → 值必须是浮点数
       VAR~bool   → 值必须是 true/false/1/0
       VAR~url    → 值必须是合法 URL
       VAR~ip     → 值必须是合法 IPv4 地址

YAML 规则文件（v0.8）：
  若存在 .env-required.yaml，优先使用（.env-required 仍作为 fallback）。
  支持更丰富的 schema 定义，示例：
    DB_HOST:
      type: required
      pattern: "^localhost|^\\d+\\.\\d+\\.\\d+\\.\\d+$"
      description: "数据库主机地址"

规则分层（v0.7）：
  项目根目录的 .env-required.d/*.conf 文件按字母序追加/覆盖规则。

环境变量文件加载（v0.7）：
  设置 ENV_VALIDATE_ENV_FILE=/path/to/.env 后，验证前自动导入该文件。

CI 报告：
  - ENV_VALIDATE_REPORT=/path/to/report.json  → JSON 报告
  - ENV_VALIDATE_SARIF=/path/to/report.sarif  → SARIF 报告（GitHub Code Scanning）
"""

import os
import re
import sys
import json
import glob

if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

VERSION = "0.8.0"
DEFAULT_RULES = {"DB_HOST": {"type": "required"}, "API_KEY": {"type": "required"}}

# 内置快捷模式
BUILTIN_PATTERNS = {
    "int":   r"^-?\d+$",
    "float": r"^-?\d+(\.\d+)?$",
    "bool":  r"^(true|false|1|0)$",
    "url":   r"^https?://[^\s/$?#]*\.[^\s]+$",
    "ip":    r"^(\d{1,3}\.){3}\d{1,3}$",
    "uuid":  r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
}


def _parse_pattern_value(pattern_str):
    """解析 pattern 字符串，支持内置快捷类型和完整正则。"""
    if pattern_str in BUILTIN_PATTERNS:
        return BUILTIN_PATTERNS[pattern_str]
    return pattern_str


def _check_pattern(value, pattern_str):
    """检查值是否匹配 pattern（支持内置快捷类型，自动剥离用户已写的锚点）。"""
    regex = _parse_pattern_value(pattern_str)
    # re.fullmatch 已隐式加 ^...$，剥离用户手写的锚点避免双重锚点
    if regex.startswith("^"):
        regex = regex[1:]
    if regex.endswith("$"):
        regex = regex[:-1]
    try:
        flags = re.IGNORECASE if pattern_str.lower() in ("bool", "url") else 0
        return re.fullmatch(regex, value, flags) is not None
    except re.error:
        # 正则编译失败时，做简单字符串包含检查
        return pattern_str in value


def _parse_line(line, rules):
    """解析单行规则，写入 rules 字典。"""
    line = line.strip()
    if not line or line.startswith("#"):
        return

    # 优先级: ~ > : > = > bare
    if "~" in line:
        var, pattern = line.split("~", 1)
        var = var.strip()
        if var:
            rules[var] = {"type": "pattern", "pattern": pattern.strip()}
    elif ":" in line and "=" not in line:
        var, default_val = line.split(":", 1)
        var = var.strip()
        if var:
            rules[var] = {"type": "default", "default": default_val.strip()}
    elif "=" in line:
        var, expected_val = line.split("=", 1)
        var = var.strip()
        if var:
            rules[var] = {"type": "match", "expected": expected_val.strip()}
    else:
        if line:
            rules[line] = {"type": "required"}


def _load_yaml_rules(path, rules):
    """从 YAML 文件加载规则（简化解析，无需 pyyaml 依赖）。

    支持格式：
      VAR:
        type: required|match|default|pattern
        expected: "value"
        default: "value"
        pattern: "regex"
    """
    found = False
    current_var = None
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip()
            if not line or line.startswith("#"):
                continue
            # VAR:
            if line.endswith(":") and not line.startswith(" ") and not line.startswith("\t"):
                current_var = line[:-1].strip()
                if current_var and current_var not in rules:
                    rules[current_var] = {"type": "required"}
                found = True
            #   key: value
            elif current_var and (line.startswith("  ") or line.startswith("\t")):
                stripped = line.strip()
                if ":" in stripped:
                    key, _, val = stripped.partition(":")
                    key = key.strip()
                    val = val.strip().strip("'\"")
                    spec = rules.get(current_var, {"type": "required"})
                    if key == "type":
                        spec["type"] = val
                    elif key == "expected":
                        spec["expected"] = val
                    elif key == "default":
                        spec["default"] = val
                    elif key == "pattern":
                        spec["pattern"] = val
                        spec["type"] = "pattern"
                    rules[current_var] = spec
                # 空行或无键值对时重置
            else:
                current_var = None

    return found


def parse_rules():
    """解析规则文件，返回 (rules_dict, source_info)。

    优先级：
      1. .env-required.yaml（若存在）
      2. .env-required（主文件）
      3. .env-required.d/*.conf（分层）
      4. DEFAULT_RULES（fallback）
    """
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    yaml_path = os.path.join(root, ".env-required.yaml")
    config_path = os.path.join(root, ".env-required")
    rules_dir = os.path.join(root, ".env-required.d")

    rules = {}
    found = False
    source = "default"

    # YAML 文件优先
    if os.path.isfile(yaml_path):
        found = _load_yaml_rules(yaml_path, rules)
        source = ".env-required.yaml"

    # 主文件
    if os.path.isfile(config_path):
        with open(config_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                found = True
                _parse_line(line, rules)
        if source == "default":
            source = ".env-required"

    # 分层目录
    if os.path.isdir(rules_dir):
        conf_files = sorted(glob.glob(os.path.join(rules_dir, "*.conf")))
        for conf_path in conf_files:
            with open(conf_path, encoding="utf-8") as f:
                for line in f:
                    _parse_line(line.strip(), rules)
            yaml_conf = conf_path.replace(".conf", ".yaml")
            if os.path.isfile(yaml_conf):
                _load_yaml_rules(yaml_conf, rules)
        if conf_files:
            found = True

    if not found:
        rules = dict(DEFAULT_RULES)
        source = "default"

    return rules, source, rules_dir


def load_env_file(env_file):
    """从 .env 文件导入环境变量（不覆盖已存在的变量）。"""
    if not env_file or not os.path.isfile(env_file):
        return 0

    imported = 0
    with open(env_file, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("export "):
                line = line[7:]
            if "=" not in line:
                continue
            key, _, value = line.partition("=")
            key = key.strip()
            value = value.strip().strip("'\''")
            if key and key not in os.environ:
                os.environ[key] = value
                imported += 1

    return imported


def validate(rules):
    """执行校验，返回 (results, errors) 列表。"""
    results = []
    errors = []

    for var, spec in rules.items():
        current = os.environ.get(var, "")
        rule_type = spec["type"]

        if rule_type == "default":
            default_val = spec.get("default", "")
            if not current:
                os.environ[var] = default_val
                results.append({"var": var, "status": "defaulted", "value": default_val})
                print(f"  ◑ Defaulted: {var} = {default_val}")
            else:
                results.append({"var": var, "status": "ok", "value": current})
                display = current[:4] + "..." if len(current) > 8 else current
                print(f"  ✓ Found: {var} = {display}")

        elif rule_type == "match":
            expected = spec["expected"]
            if not current:
                errors.append({"var": var, "issue": "missing"})
                results.append({"var": var, "status": "missing"})
                print(f"  ✗ Missing: {var} (required)")
            elif current != expected:
                errors.append({"var": var, "issue": "mismatch", "expected": expected, "got": current})
                results.append({"var": var, "status": "mismatch"})
                print(f"  ✗ Mismatch: {var} (expected '{expected}', got '{current}')")
            else:
                results.append({"var": var, "status": "ok", "value": current})
                print(f"  ✓ Match: {var} = {current}")

        elif rule_type == "pattern":
            pattern = spec.get("pattern", "")
            if not current:
                errors.append({"var": var, "issue": "missing"})
                results.append({"var": var, "status": "missing"})
                print(f"  ✗ Missing: {var} (pattern required: {pattern})")
            elif not _check_pattern(current, pattern):
                errors.append({"var": var, "issue": "pattern_fail", "pattern": pattern, "got": current})
                results.append({"var": var, "status": "pattern_fail"})
                print(f"  ✗ Pattern fail: {var} (pattern '{pattern}', got '{current}')")
            else:
                results.append({"var": var, "status": "ok", "value": current})
                display = current[:4] + "..." if len(current) > 8 else current
                print(f"  ✓ Pattern ok: {var} matches '{pattern}'")

        else:  # required
            if not current:
                errors.append({"var": var, "issue": "missing"})
                results.append({"var": var, "status": "missing"})
                print(f"  ✗ Missing: {var}")
            else:
                display = current[:4] + "..." if len(current) > 8 else current
                results.append({"var": var, "status": "ok", "value": current})
                print(f"  ✓ Found: {var} = {display}")

    return results, errors


def write_report(rules, results, errors, is_ci):
    """生成 JSON 报告。"""
    report_path = os.environ.get("ENV_VALIDATE_REPORT", "")
    if not report_path:
        return

    report = {
        "tool": "env-manager-lite",
        "version": VERSION,
        "mode": "ci" if is_ci else "local",
        "passed": len(errors) == 0,
        "total": len(results),
        "passed_count": len(results) - len(errors),
        "failed_count": len(errors),
        "results": results,
        "errors": errors,
    }

    report_dir = os.path.dirname(report_path)
    if report_dir:
        os.makedirs(report_dir, exist_ok=True)

    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"  📄 Report written to: {report_path}")


def write_sarif(results, errors):
    """生成 SARIF 2.1.0 报告。"""
    sarif_path = os.environ.get("ENV_VALIDATE_SARIF", "")
    if not sarif_path:
        return

    runs = []
    if errors:
        rules_def = [
            {"id": "env-missing", "name": "EnvironmentVariableMissing",
             "shortDescription": {"text": "Required environment variable is missing"},
             "defaultConfiguration": {"level": "error"}},
            {"id": "env-mismatch", "name": "EnvironmentVariableMismatch",
             "shortDescription": {"text": "Environment variable value does not match expected"},
             "defaultConfiguration": {"level": "error"}},
            {"id": "env-pattern-fail", "name": "EnvironmentVariablePatternFail",
             "shortDescription": {"text": "Environment variable does not match required pattern"},
             "defaultConfiguration": {"level": "error"}},
        ]

        sarif_errors = []
        for e in errors:
            if e["issue"] == "missing":
                rule_id = "env-missing"
                msg = f"Environment variable '{e['var']}' is missing"
            elif e["issue"] == "mismatch":
                rule_id = "env-mismatch"
                msg = f"Environment variable '{e['var']}' expected '{e.get('expected', '')}' but got '{e.get('got', '')}'"
            else:  # pattern_fail
                rule_id = "env-pattern-fail"
                msg = f"Environment variable '{e['var']}' does not match pattern '{e.get('pattern', '')}'"

            sarif_errors.append({
                "ruleId": rule_id,
                "message": {"text": msg},
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {"uri": ".env-required", "index": 0},
                        "region": {"startLine": 1},
                    }
                }],
            })

        runs.append({
            "tool": {
                "driver": {
                    "name": "env-manager-lite",
                    "version": VERSION,
                    "informationUri": "https://github.com/wonderfulvchaum/env-manager-lite",
                    "rules": rules_def,
                }
            },
            "results": sarif_errors,
        })

    sarif = {
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": runs,
    }

    sarif_dir = os.path.dirname(sarif_path)
    if sarif_dir:
        os.makedirs(sarif_dir, exist_ok=True)

    with open(sarif_path, "w", encoding="utf-8") as f:
        json.dump(sarif, f, indent=2, ensure_ascii=False)

    print(f"  📄 SARIF report written to: {sarif_path}")


def main():
    is_ci = os.environ.get("GITHUB_ACTIONS", "").lower() == "true" or "--ci" in sys.argv

    print("=" * 46)
    print(f"  Env Manager Lite - Python Validator v{VERSION}")
    print(f"  Mode: {'CI' if is_ci else 'Local'}")
    print("=" * 46)

    # 1. 加载 .env 文件
    env_file = os.environ.get("ENV_VALIDATE_ENV_FILE", "")
    if env_file:
        imported = load_env_file(env_file)
        print(f"  📦 Loaded .env file: {env_file} ({imported} variable(s) imported)")

    # 2. 解析规则
    rules, source, rules_dir = parse_rules()
    layer_files = []
    if rules_dir and os.path.isdir(rules_dir):
        layer_files = sorted(os.listdir(rules_dir))
    types_count = {}
    for spec in rules.values():
        types_count[spec["type"]] = types_count.get(spec["type"], 0) + 1

    layer_info = f" ({len(layer_files)} layer file(s))" if layer_files else ""
    print(
        f"  Source: {source}"
        f"  Checking {len(rules)} rule(s): "
        f"{types_count.get('required', 0)} required, "
        f"{types_count.get('match', 0)} match, "
        f"{types_count.get('default', 0)} default, "
        f"{types_count.get('pattern', 0)} pattern{layer_info}...\n"
    )

    # 3. 执行校验
    results, errors = validate(rules)

    # 4. 输出报告
    write_report(rules, results, errors, is_ci)
    write_sarif(results, errors)

    print()
    if errors:
        print(f"  ✗ Failed: {len(errors)} issue(s).")
        missing = [e["var"] for e in errors if e["issue"] == "missing"]
        mismatched = [e["var"] for e in errors if e["issue"] == "mismatch"]
        pattern_failed = [e["var"] for e in errors if e["issue"] == "pattern_fail"]
        if missing:
            print(f"    Missing: {', '.join(missing)}")
        if mismatched:
            print(f"    Mismatch: {', '.join(mismatched)}")
        if pattern_failed:
            print(f"    Pattern fail: {', '.join(pattern_failed)}")
        sys.exit(1)
    else:
        print("  ✓ All checks passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
