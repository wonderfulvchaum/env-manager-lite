"""
Env Manager Lite - Python Validator

规则文件 .env-required 语法（三种条目类型）：
  1. VAR            → 必填，缺失则失败
  2. VAR=expected   → 精确匹配，不存在则失败，存在但值不同则失败
  3. VAR:default    → 带默认值，不存在时注入 default 后通过；存在则保留原值
     若 default 为空字符串（VAR:），则仅做"存在性检查"（不强制非空）

CI 报告：
  - ENV_VALIDATE_REPORT=/path/to/report.json  → JSON 报告
  - ENV_VALIDATE_SARIF=/path/to/report.sarif  → SARIF 报告（GitHub Code Scanning）
"""

import os
import sys
import json

if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

# 默认规则（.env-required 不存在时的 fallback）
DEFAULT_RULES = {"DB_HOST": {"type": "required"}, "API_KEY": {"type": "required"}}


def parse_rules():
    """解析 .env-required，返回 {var: {type, expected/default}} 字典。

    条目类型：
      required  - VAR
      match     - VAR=expected
      default   - VAR:default
    """
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    config_path = os.path.join(root, ".env-required")

    rules = {}
    found = False

    if os.path.isfile(config_path):
        with open(config_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                found = True
                if ":" in line and "=" not in line:
                    var, default_val = line.split(":", 1)
                    rules[var.strip()] = {"type": "default", "default": default_val.strip()}
                elif "=" in line:
                    var, expected_val = line.split("=", 1)
                    rules[var.strip()] = {"type": "match", "expected": expected_val.strip()}
                else:
                    rules[line] = {"type": "required"}

    if not found:
        rules = dict(DEFAULT_RULES)

    return rules


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
        "version": "0.5.0",
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
        rule_ids = []
        for e in errors:
            rule_id = "env-missing" if e["issue"] == "missing" else "env-mismatch"
            if rule_id not in rule_ids:
                rule_ids.append(rule_id)

        rules = [
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
            msg = f"Environment variable '{e['var']}' is missing"
            if e["issue"] == "mismatch":
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
                        "version": "0.6.0",
                        "informationUri": "https://github.com/wonderfulvchaum/env-manager-lite",
                        "rules": rules,
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
    print("  Env Manager Lite - Python Validator v0.5")
    print(f"  Mode: {'CI' if is_ci else 'Local'}")
    print("=" * 46)

    rules = parse_rules()
    types_count = {}
    for spec in rules.values():
        types_count[spec["type"]] = types_count.get(spec["type"], 0) + 1

    print(
        f"  Checking {len(rules)} rule(s): "
        f"{types_count.get('required', 0)} required, "
        f"{types_count.get('match', 0)} match, "
        f"{types_count.get('default', 0)} default...\n"
    )

    results, errors = validate(rules)
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
