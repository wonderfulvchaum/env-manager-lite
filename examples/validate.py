"""
Env Manager Lite - Python Validator v0.7

规则文件 .env-required 语法（三种条目类型）：
  1. VAR            → 必填，缺失则失败
  2. VAR=expected   → 精确匹配，不存在则失败，存在但值不同则失败
  3. VAR:default    → 带默认值，不存在时注入 default 后通过；存在则保留原值

规则分层（v0.7）：
  项目根目录的 .env-required.d/*.conf 文件按字母序追加/覆盖规则。
  后加载的文件中的同名变量覆盖先加载的。

环境变量文件加载（v0.7）：
  设置 ENV_VALIDATE_ENV_FILE=/path/to/.env 后，验证前自动导入该文件
  中的 KEY=VALUE 对到 os.environ（不覆盖已存在的变量）。

CI 报告：
  - ENV_VALIDATE_REPORT=/path/to/report.json  → JSON 报告
  - ENV_VALIDATE_SARIF=/path/to/report.sarif  → SARIF 报告（GitHub Code Scanning）
"""

import os
import sys
import json
import glob

if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

VERSION = "0.7.0"
# 默认规则（.env-required 不存在时的 fallback）
DEFAULT_RULES = {"DB_HOST": {"type": "required"}, "API_KEY": {"type": "required"}}


def _parse_line(line, rules):
    """解析单行规则，写入 rules 字典。返回是否解析成功。"""
    line = line.strip()
    if not line or line.startswith("#"):
        return True

    if ":" in line and "=" not in line:
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
    return True


def parse_rules():
    """解析 .env-required + .env-required.d/*.conf，返回 {var: spec} 字典。

    加载顺序：
      1. .env-required（主文件）
      2. .env-required.d/*.conf（按文件名字母序，后加载的覆盖同名变量）
    """
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    config_path = os.path.join(root, ".env-required")
    rules_dir = os.path.join(root, ".env-required.d")

    rules = {}
    found = False

    # 主文件
    if os.path.isfile(config_path):
        with open(config_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                found = True
                _parse_line(line, rules)

    # 分层目录
    if os.path.isdir(rules_dir):
        conf_files = sorted(glob.glob(os.path.join(rules_dir, "*.conf")))
        for conf_path in conf_files:
            with open(conf_path, encoding="utf-8") as f:
                for line in f:
                    _parse_line(line.strip(), rules)
        if conf_files:
            found = True

    if not found:
        rules = dict(DEFAULT_RULES)

    return rules, rules_dir


def load_env_file(env_file):
    """从 .env 文件导入环境变量（不覆盖已存在的变量）。

    支持 KEY=VALUE 和 export KEY=VALUE 格式，# 开头为注释。
    返回导入的变量数量。
    """
    if not env_file or not os.path.isfile(env_file):
        return 0

    imported = 0
    with open(env_file, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            # 支持 "export KEY=VALUE" 格式
            if line.startswith("export "):
                line = line[7:]
            if "=" not in line:
                continue
            key, _, value = line.partition("=")
            key = key.strip()
            value = value.strip().strip("'\"")
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
                # 注入默认值
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
    """生成 JSON 报告（如设置了 ENV_VALIDATE_REPORT）。"""
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

    # 确保目录存在
    report_dir = os.path.dirname(report_path)
    if report_dir:
        os.makedirs(report_dir, exist_ok=True)

    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    print(f"  📄 Report written to: {report_path}")


def write_sarif(results, errors):
    """生成 SARIF 2.1.0 报告（GitHub Code Scanning 集成）。"""
    sarif_path = os.environ.get("ENV_VALIDATE_SARIF", "")
    if not sarif_path:
        return

    runs = []
    if errors:
        rules_def = [
            {
                "id": "env-missing",
                "name": "EnvironmentVariableMissing",
                "shortDescription": {"text": "Required environment variable is missing"},
                "defaultConfiguration": {"level": "error"},
            },
            {
                "id": "env-mismatch",
                "name": "EnvironmentVariableMismatch",
                "shortDescription": {"text": "Environment variable value does not match expected"},
                "defaultConfiguration": {"level": "error"},
            },
        ]

        sarif_errors = []
        for e in errors:
            rule_id = "env-missing" if e["issue"] == "missing" else "env-mismatch"
            if e["issue"] == "missing":
                msg = f"Environment variable '{e['var']}' is missing"
            else:
                msg = f"Environment variable '{e['var']}' expected '{e.get('expected', '')}' but got '{e.get('got', '')}'"
            sarif_errors.append(
                {
                    "ruleId": rule_id,
                    "message": {"text": msg},
                    "locations": [
                        {
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": ".env-required",
                                    "index": 0,
                                },
                                "region": {"startLine": 1},
                            }
                        }
                    ],
                }
            )

        runs.append(
            {
                "tool": {
                    "driver": {
                        "name": "env-manager-lite",
                        "version": VERSION,
                        "informationUri": "https://github.com/wonderfulvchaum/env-manager-lite",
                        "rules": rules_def,
                    }
                },
                "results": sarif_errors,
            }
        )

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

    # 1. 加载 .env 文件（如设置了 ENV_VALIDATE_ENV_FILE）
    env_file = os.environ.get("ENV_VALIDATE_ENV_FILE", "")
    if env_file:
        imported = load_env_file(env_file)
        print(f"  📦 Loaded .env file: {env_file} ({imported} variable(s) imported)")

    # 2. 解析规则（主文件 + 分层目录）
    rules, rules_dir = parse_rules()
    layer_files = []
    if rules_dir and os.path.isdir(rules_dir):
        layer_files = sorted(os.listdir(rules_dir))
    types_count = {}
    for spec in rules.values():
        types_count[spec["type"]] = types_count.get(spec["type"], 0) + 1

    layer_info = f" ({len(layer_files)} layer file(s) in .env-required.d)" if layer_files else ""
    print(
        f"  Checking {len(rules)} rule(s): "
        f"{types_count.get('required', 0)} required, "
        f"{types_count.get('match', 0)} match, "
        f"{types_count.get('default', 0)} default{layer_info}...\n"
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
        if missing:
            print(f"    Missing: {', '.join(missing)}")
        if mismatched:
            print(f"    Mismatch: {', '.join(mismatched)}")
        sys.exit(1)
    else:
        print("  ✓ All checks passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
