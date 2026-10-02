"""
Env Manager Lite - Python Validator

支持两种验证模式：
1. 本地验证：python examples/validate.py
2. CI 验证：GITHUB_ACTIONS=true 时自动启用

规则文件 .env-required 语法：
  - 纯变量名（DB_HOST）：必须存在且非空
  - VAR=value（DB_PORT=5432）：存在则校验值匹配，不存在则跳过（可选）
"""

import os
import sys

if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if sys.stderr.encoding and sys.stderr.encoding.lower() not in ("utf-8", "utf8"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


def load_rules():
    """解析 .env-required，返回 (required, optional) 两个列表。

    required: 纯变量名列表（必须存在且非空）
    optional: (变量名, 期望值) 元组列表（存在时校验值）
    """
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    config_path = os.path.join(root, ".env-required")

    default_required = ["DB_HOST", "API_KEY"]
    default_optional = []

    if not os.path.isfile(config_path):
        return default_required, default_optional

    required = []
    optional = []
    with open(config_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                var, val = line.split("=", 1)
                var = var.strip()
                val = val.strip()
                if var:
                    optional.append((var, val))
            else:
                required.append(line)

    if not required and not optional:
        required = default_required

    return required, optional


def main():
    is_ci = os.environ.get("GITHUB_ACTIONS", "").lower() == "true"

    print("=" * 46)
    print("  Env Manager Lite - Python Validator")
    print(f"  Mode: {'CI' if is_ci else 'Local'}")
    print("=" * 46)

    required, optional = load_rules()
    print(f"  Checking {len(required)} required + {len(optional)} optional...\n")

    missing = []
    mismatches = []

    for var in required:
        value = os.environ.get(var, "")
        if not value:
            missing.append(var)
            print(f"  ✗ Missing: {var}")
        else:
            display = value[:4] + "..." if len(value) > 8 else value
            print(f"  ✓ Found: {var} = {display}")

    for var, expected in optional:
        value = os.environ.get(var, "")
        if not value:
            print(f"  ○ Optional: {var} (not set)")
        elif value != expected:
            mismatches.append(var)
            print(f"  ✗ Mismatch: {var} (expected '{expected}', got '{value}')")
        else:
            print(f"  ✓ Match: {var} = {value}")

    print()
    total_errors = len(missing) + len(mismatches)
    if total_errors > 0:
        print(f"  ✗ Failed: {total_errors} issue(s).")
        if missing:
            print(f"    Missing: {', '.join(missing)}")
        if mismatches:
            print(f"    Mismatch: {', '.join(mismatches)}")
        sys.exit(1)
    else:
        print("  ✓ All checks passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
