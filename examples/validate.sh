#!/bin/bash
# Env Manager Lite - Bash Validator
#
# .env-required 语法：
#   纯变量名（DB_HOST）：必须存在且非空
#   VAR=value（DB_PORT=5432）：存在则校验值匹配，不存在则跳过（可选）

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

CI_MODE="Local"
if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  CI_MODE="CI"
fi

echo "=============================================="
echo "  Env Manager Lite - Bash Validator"
echo "  Mode: $CI_MODE"
echo "=============================================="

# 解析 .env-required
CONFIG_FILE="$ROOT_DIR/.env-required"
REQUIRED_VARS=()
OPTIONAL_VARS=()   # "name=expected" 对

if [[ -f "$CONFIG_FILE" ]]; then
  while IFS= read -r line; do
    line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -z "$line" || "$line" =~ ^# ]] && continue
    if [[ "$line" == *=* ]]; then
      OPTIONAL_VARS+=("$line")
    else
      REQUIRED_VARS+=("$line")
    fi
  done < "$CONFIG_FILE"
fi

# 默认值
if [[ ${#REQUIRED_VARS[@]} -eq 0 && ${#OPTIONAL_VARS[@]} -eq 0 ]]; then
  REQUIRED_VARS=("DB_HOST" "API_KEY")
fi

echo "  Checking ${#REQUIRED_VARS[@]} required + ${#OPTIONAL_VARS[@]} optional..."
echo ""

MISSING=()
MISMATCH=()

# 检查必填
for var in "${REQUIRED_VARS[@]}"; do
  val="${!var:-}"
  if [[ -z "$val" ]]; then
    MISSING+=("$var")
    echo "  ✗ Missing: $var"
  else
    display="${val:0:4}..."
    [[ ${#val} -le 8 ]] && display="$val"
    echo "  ✓ Found: $var = $display"
  fi
done

# 检查可选
for pair in "${OPTIONAL_VARS[@]}"; do
  var="${pair%%=*}"
  expected="${pair#*=}"
  val="${!var:-}"
  if [[ -z "$val" ]]; then
    echo "  ○ Optional: $var (not set)"
  elif [[ "$val" != "$expected" ]]; then
    MISMATCH+=("$var")
    echo "  ✗ Mismatch: $var (expected '$expected', got '$val')"
  else
    echo "  ✓ Match: $var = $val"
  fi
done

echo ""
TOTAL=$(( ${#MISSING[@]} + ${#MISMATCH[@]} ))
if [[ $TOTAL -gt 0 ]]; then
  echo "  ✗ Failed: $TOTAL issue(s)."
  [[ ${#MISSING[@]} -gt 0 ]] && echo "    Missing: ${MISSING[*]}"
  [[ ${#MISMATCH[@]} -gt 0 ]] && echo "    Mismatch: ${MISMATCH[*]}"
  exit 1
else
  echo "  ✓ All checks passed!"
  exit 0
fi
