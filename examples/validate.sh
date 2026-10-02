#!/bin/bash
# Env Manager Lite - Bash Validator v0.5
#
# .env-required 三种条目：
#   VAR            必填（缺失则失败）
#   VAR=expected   精确匹配
#   VAR:default    带默认值（Bash 不做注入，仅报告；如未设置且无默认值则失败）
#
# 注意: Bash 无法像 Python/Node 那样注入默认值到当前进程，
#       因此 VAR:default 条目在 Bash 验证器中视为"可选存在性检查"。

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

CI_MODE="Local"
[[ "${GITHUB_ACTIONS:-}" == "true" ]] && CI_MODE="CI"

echo "=============================================="
echo "  Env Manager Lite - Bash Validator v0.5"
echo "  Mode: $CI_MODE"
echo "=============================================="

CONFIG_FILE="$ROOT_DIR/.env-required"

# 解析规则: 用两个数组存 "VAR" (required) 和 "VAR=expected" / "VAR:default"
REQUIRED_LINES=()
MATCH_LINES=()
DEFAULT_LINES=()

if [[ -f "$CONFIG_FILE" ]]; then
  while IFS= read -r line; do
    line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -z "$line" || "$line" =~ ^# ]] && continue
    if [[ "$line" == *:* ]]; then
      DEFAULT_LINES+=("$line")
    elif [[ "$line" == *=* ]]; then
      MATCH_LINES+=("$line")
    else
      REQUIRED_LINES+=("$line")
    fi
  done < "$CONFIG_FILE"
fi

TOTAL=$(( ${#REQUIRED_LINES[@]} + ${#MATCH_LINES[@]} + ${#DEFAULT_LINES[@]} ))
if [[ $TOTAL -eq 0 ]]; then
  REQUIRED_LINES=("DB_HOST" "API_KEY")
fi

echo "  Checking ${#REQUIRED_LINES[@]} required, ${#MATCH_LINES[@]} match, ${#DEFAULT_LINES[@]} default..."
echo ""

ERRORS=0

# 必填
for var in "${REQUIRED_LINES[@]:-}"; do
  [[ -z "$var" ]] && continue
  val="${!var:-}"
  if [[ -z "$val" ]]; then
    echo "  ✗ Missing: $var"
    ERRORS=$((ERRORS + 1))
  else
    display="${val:0:4}..."
    [[ ${#val} -le 8 ]] && display="$val"
    echo "  ✓ Found: $var = $display"
  fi
done

# 精确匹配
for pair in "${MATCH_LINES[@]:-}"; do
  [[ -z "$pair" ]] && continue
  var="${pair%%=*}"
  expected="${pair#*=}"
  val="${!var:-}"
  if [[ -z "$val" ]]; then
    echo "  ✗ Missing: $var (required)"
    ERRORS=$((ERRORS + 1))
  elif [[ "$val" != "$expected" ]]; then
    echo "  ✗ Mismatch: $var (expected '$expected', got '$val')"
    ERRORS=$((ERRORS + 1))
  else
    echo "  ✓ Match: $var = $val"
  fi
done

# 默认值（Bash: 仅报告，不注入）
for pair in "${DEFAULT_LINES[@]:-}"; do
  [[ -z "$pair" ]] && continue
  var="${pair%%:*}"
  default_val="${pair#*:}"
  val="${!var:-}"
  if [[ -z "$val" ]]; then
    echo "  ○ Optional: $var (not set, default: $default_val)"
  else
    echo "  ✓ Found: $var = $val"
  fi
done

echo ""
if [[ $ERRORS -gt 0 ]]; then
  echo "  ✗ Failed: $ERRORS issue(s)."
  exit 1
else
  echo "  ✓ All checks passed!"
  exit 0
fi
