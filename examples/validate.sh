#!/bin/bash
# Env Manager Lite - Bash Validator v0.7
#
# .env-required 三种条目：
#   VAR            必填（缺失则失败）
#   VAR=expected   精确匹配
#   VAR:default    带默认值（Bash 不做注入，仅报告；可选存在性检查）
#
# 规则分层（v0.7）：
#   .env-required.d/*.conf 按字母序追加/覆盖规则
#
# .env 文件加载（v0.7）：
#   设置 ENV_VALIDATE_ENV_FILE 后 source 该文件导入变量（不覆盖已存在的）

set -uo pipefail

VERSION="0.7.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

CI_MODE="Local"
[[ "${GITHUB_ACTIONS:-}" == "true" ]] && CI_MODE="CI"

echo "=============================================="
echo "  Env Manager Lite - Bash Validator v${VERSION}"
echo "  Mode: $CI_MODE"
echo "=============================================="

# 加载 .env 文件（不覆盖已存在的变量）
if [[ -n "${ENV_VALIDATE_ENV_FILE:-}" && -f "${ENV_VALIDATE_ENV_FILE}" ]]; then
  _imported=0
  while IFS= read -r _line; do
    _line="$(echo "$_line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -z "$_line" || "$_line" =~ ^# ]] && continue
    [[ "$_line" == export\ * ]] && _line="${_line#export }"
    [[ "$_line" != *=* ]] && continue
    _key="${_line%%=*}"
    _val="${_line#*=}"
    _val="$(echo "$_val" | sed "s/^['\"]//;s/['\"]\$//")"
    _key="$(echo "$_key" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    if [[ -z "${!_key:-}" ]]; then
      eval "export $_key=\$_val"
      _imported=$(( _imported + 1 ))
    fi
  done < "$ENV_VALIDATE_ENV_FILE"
  echo "  📦 Loaded .env file: $ENV_VALIDATE_ENV_FILE ($_imported variable(s) imported)"
fi

CONFIG_FILE="$ROOT_DIR/.env-required"
RULES_DIR="$ROOT_DIR/.env-required.d"

# 解析规则文件到三个数组
REQUIRED_LINES=()
MATCH_LINES=()
DEFAULT_LINES=()

_parse_rules_file() {
  local file="$1"
  [[ ! -f "$file" ]] && return
  while IFS= read -r line; do
    line="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
    [[ -z "$line" || "$line" =~ ^# ]] && continue
    if [[ "$line" == *:* ]] && [[ "$line" != *=* ]]; then
      DEFAULT_LINES+=("$line")
    elif [[ "$line" == *=* ]]; then
      MATCH_LINES+=("$line")
    else
      REQUIRED_LINES+=("$line")
    fi
  done < "$file"
}

# 主文件
_parse_rules_file "$CONFIG_FILE"

# 分层目录
LAYER_COUNT=0
if [[ -d "$RULES_DIR" ]]; then
  for conf in "$RULES_DIR"/*.conf; do
    [[ -f "$conf" ]] || continue
    _parse_rules_file "$conf"
    LAYER_COUNT=$(( LAYER_COUNT + 1 ))
  done
fi

# 去重: 后加载的分层文件中的同名变量应覆盖主文件
# 使用关联数组去重（保留每个变量最后一次出现的完整行）
declare -A _dedup_req _dedup_match _dedup_def
for _v in "${REQUIRED_LINES[@]:-}"; do
  [[ -z "$_v" ]] && continue
  _dedup_req[$_v]=1
done
REQUIRED_LINES=("${!_dedup_req[@]}")

for _v in "${MATCH_LINES[@]:-}"; do
  [[ -z "$_v" ]] && continue
  # 按变量名去重，保留最后出现
  _name="${_v%%=*}"
  _dedup_match[$_name]="$_v"
done
MATCH_LINES=()
for _n in "${!_dedup_match[@]}"; do
  MATCH_LINES+=("${_dedup_match[$_n]}")
done

for _v in "${DEFAULT_LINES[@]:-}"; do
  [[ -z "$_v" ]] && continue
  _name="${_v%%:*}"
  _dedup_def[$_name]="$_v"
done
DEFAULT_LINES=()
for _n in "${!_dedup_def[@]}"; do
  DEFAULT_LINES+=("${_dedup_def[$_n]}")
done

# 默认值（无任何规则时）
if [[ ${#REQUIRED_LINES[@]} -eq 0 && ${#MATCH_LINES[@]} -eq 0 && ${#DEFAULT_LINES[@]} -eq 0 ]]; then
  REQUIRED_LINES=("DB_HOST" "API_KEY")
fi

LAYER_INFO=""
if [[ $LAYER_COUNT -gt 0 ]]; then
  LAYER_INFO=" ($LAYER_COUNT layer file(s) in .env-required.d)"
fi

echo "  Checking ${#REQUIRED_LINES[@]} required, ${#MATCH_LINES[@]} match, ${#DEFAULT_LINES[@]} default${LAYER_INFO}..."
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
