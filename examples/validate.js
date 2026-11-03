/**
 * Env Manager Lite - Node.js Validator v0.8
 *
 * 规则文件 .env-required 四种条目类型：
 *   VAR            → 必填，缺失则失败
 *   VAR=expected   → 精确匹配
 *   VAR:default    → 带默认值，不存在时注入
 *   VAR~pattern    → 正则校验（v0.8），支持内置快捷类型 int/float/bool/url/ip/uuid
 *
 * YAML 规则文件（v0.8）：
 *   若存在 .env-required.yaml 则优先加载。
 *
 * 规则分层（v0.7）：
 *   .env-required.d/*.conf 按字母序追加/覆盖规则。
 *
 * .env 文件加载（v0.7）：
 *   设置 ENV_VALIDATE_ENV_FILE 后自动导入 .env 文件（不覆盖已存在的变量）。
 *
 * CI 报告：
 *   ENV_VALIDATE_REPORT / ENV_VALIDATE_SARIF
 */

const fs = require('fs');
const path = require('path');

const VERSION = '0.8.0';
const DEFAULT_RULES = {
  DB_HOST: { type: 'required' },
  API_KEY: { type: 'required' },
};

const BUILTIN_PATTERNS = {
  int:   '^-?\\d+$',
  float: '^-?\\d+(\\.\\d+)?$',
  bool:  '^(true|false|1|0)$',
  url:   '^https?://[^\\s/$?#]*\\.[^\\s]+$',
  ip:    '^(\\d{1,3}\\.) {3}\\d{1,3}$',
  uuid:  '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
};

function resolvePattern(patternStr) {
  return BUILTIN_PATTERNS[patternStr] || patternStr;
}

function checkPattern(value, patternStr) {
  let regexStr = resolvePattern(patternStr);
  // 剥离已有的锚点，避免 ^(...) 双重锚点导致匹配异常
  if (regexStr.startsWith('^')) regexStr = regexStr.slice(1);
  if (regexStr.endsWith('$')) regexStr = regexStr.slice(0, -1);
  try {
    const flags = /^(bool|url)$/i.test(patternStr) ? 'i' : '';
    return new RegExp(`^${regexStr}$`, flags).test(value);
  } catch {
    return value.includes(patternStr);
  }
}

function parseLine(line, rules) {
  line = line.trim();
  if (!line || line.startsWith('#')) return;
  // 优先级: ~ > : > = > bare
  if (line.includes('~')) {
    const [varName, pattern] = line.split('~', 2);
    if (varName.trim()) {
      rules[varName.trim()] = { type: 'pattern', pattern: pattern.trim() };
    }
  } else if (line.includes(':') && !line.includes('=')) {
    const [varName, defaultVal] = line.split(':');
    if (varName.trim()) {
      rules[varName.trim()] = { type: 'default', default: defaultVal.trim() };
    }
  } else if (line.includes('=')) {
    const idx = line.indexOf('=');
    const varName = line.slice(0, idx).trim();
    const expectedVal = line.slice(idx + 1).trim();
    if (varName) {
      rules[varName] = { type: 'match', expected: expectedVal };
    }
  } else {
    rules[line] = { type: 'required' };
  }
}

function loadYamlRules(filePath, rules) {
  let found = false;
  let currentVar = null;
  const content = fs.readFileSync(filePath, 'utf-8');
  for (const rawLine of content.split('\n')) {
    const line = rawLine.replace(/\r$/, '');
    if (!line.trim() || line.trim().startsWith('#')) continue;
    const trimmed = line.trim();
    // VAR:  顶层键
    if (line.endsWith(':') && !line.startsWith(' ') && !line.startsWith('\t')) {
      currentVar = line.slice(0, -1).trim();
      if (currentVar && !(currentVar in rules)) {
        rules[currentVar] = { type: 'required' };
      }
      found = true;
    } else if (currentVar && (line.startsWith('  ') || line.startsWith('\t'))) {
      const stripped = line.trim();
      if (stripped.includes(':')) {
        const idx = stripped.indexOf(':');
        const key = stripped.slice(0, idx).trim();
        let val = stripped.slice(idx + 1).trim();
        if ((val.startsWith("'") && val.endsWith("'")) || (val.startsWith('"') && val.endsWith('"'))) {
          val = val.slice(1, -1);
        }
        if (!(currentVar in rules)) rules[currentVar] = { type: 'required' };
        if (key === 'type') rules[currentVar].type = val;
        else if (key === 'expected') rules[currentVar].expected = val;
        else if (key === 'default') rules[currentVar].default = val;
        else if (key === 'pattern') { rules[currentVar].pattern = val; rules[currentVar].type = 'pattern'; }
      }
    } else {
      currentVar = null;
    }
  }
  return found;
}

function parseRules() {
  const root = path.resolve(__dirname, '..');
  const yamlPath = path.join(root, '.env-required.yaml');
  const configPath = path.join(root, '.env-required');
  const rulesDir = path.join(root, '.env-required.d');

  const rules = {};
  let found = false;
  let source = 'default';

  // YAML 优先
  if (fs.existsSync(yamlPath)) {
    found = loadYamlRules(yamlPath, rules);
    source = '.env-required.yaml';
  }

  if (fs.existsSync(configPath)) {
    const lines = fs.readFileSync(configPath, 'utf-8').split('\n');
    for (const rawLine of lines) {
      const line = rawLine.trim();
      if (!line || line.startsWith('#')) continue;
      found = true;
      parseLine(line, rules);
    }
    if (source === 'default') source = '.env-required';
  }

  // 分层目录
  let layerCount = 0;
  if (fs.existsSync(rulesDir)) {
    const files = fs
      .readdirSync(rulesDir)
      .filter((f) => f.endsWith('.conf'))
      .sort();
    for (const file of files) {
      const content = fs.readFileSync(path.join(rulesDir, file), 'utf-8');
      for (const rawLine of content.split('\n')) {
        parseLine(rawLine, rules);
      }
      // 也支持分层 .yaml
      const yamlConf = path.join(rulesDir, file.replace('.conf', '.yaml'));
      if (fs.existsSync(yamlConf)) {
        loadYamlRules(yamlConf, rules);
      }
      layerCount++;
    }
    if (layerCount > 0) found = true;
  }

  if (!found) {
    return { ...DEFAULT_RULES, source };
  }
  return rules;
}

function loadEnvFile(envFile) {
  if (!envFile || !fs.existsSync(envFile)) return 0;
  let imported = 0;
  const lines = fs.readFileSync(envFile, 'utf-8').split('\n');
  for (const rawLine of lines) {
    let line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    if (line.startsWith('export ')) line = line.slice(7);
    const idx = line.indexOf('=');
    if (idx === -1) continue;
    const key = line.slice(0, idx).trim();
    let value = line.slice(idx + 1).trim();
    if ((value.startsWith("'") && value.endsWith("'")) || (value.startsWith('"') && value.endsWith('"'))) {
      value = value.slice(1, -1);
    }
    if (key && process.env[key] === undefined) {
      process.env[key] = value;
      imported++;
    }
  }
  return imported;
}

function validate(rules) {
  const results = [];
  const errors = [];

  for (const [varName, spec] of Object.entries(rules)) {
    let current = process.env[varName] || '';

    if (spec.type === 'default') {
      const defaultVal = spec.default || '';
      if (!current) {
        process.env[varName] = defaultVal;
        current = defaultVal;
        results.push({ var: varName, status: 'defaulted', value: current });
        console.log(`  ◑ Defaulted: ${varName} = ${current}`);
      } else {
        const display = current.length > 8 ? `${current.slice(0, 4)}...` : current;
        results.push({ var: varName, status: 'ok', value: current });
        console.log(`  ✓ Found: ${varName} = ${display}`);
      }
    } else if (spec.type === 'match') {
      const expected = spec.expected;
      if (!current) {
        errors.push({ var: varName, issue: 'missing' });
        results.push({ var: varName, status: 'missing' });
        console.log(`  ✗ Missing: ${varName} (required)`);
      } else if (current !== expected) {
        errors.push({ var: varName, issue: 'mismatch', expected, got: current });
        results.push({ var: varName, status: 'mismatch' });
        console.log(`  ✗ Mismatch: ${varName} (expected '${expected}', got '${current}')`);
      } else {
        results.push({ var: varName, status: 'ok', value: current });
        console.log(`  ✓ Match: ${varName} = ${current}`);
      }
    } else if (spec.type === 'pattern') {
      const pattern = spec.pattern || '';
      if (!current) {
        errors.push({ var: varName, issue: 'missing' });
        results.push({ var: varName, status: 'missing' });
        console.log(`  ✗ Missing: ${varName} (pattern required: ${pattern})`);
      } else if (!checkPattern(current, pattern)) {
        errors.push({ var: varName, issue: 'pattern_fail', pattern, got: current });
        results.push({ var: varName, status: 'pattern_fail' });
        console.log(`  ✗ Pattern fail: ${varName} (pattern '${pattern}', got '${current}')`);
      } else {
        results.push({ var: varName, status: 'ok', value: current });
        console.log(`  ✓ Pattern ok: ${varName} matches '${pattern}'`);
      }
    } else {
      if (!current) {
        errors.push({ var: varName, issue: 'missing' });
        results.push({ var: varName, status: 'missing' });
        console.log(`  ✗ Missing: ${varName}`);
      } else {
        const display = current.length > 8 ? `${current.slice(0, 4)}...` : current;
        results.push({ var: varName, status: 'ok', value: current });
        console.log(`  ✓ Found: ${varName} = ${display}`);
      }
    }
  }

  return { results, errors };
}

function writeReport(rules, { results, errors }, isCI) {
  const reportPath = process.env.ENV_VALIDATE_REPORT;
  if (!reportPath) return;

  const report = {
    tool: 'env-manager-lite',
    version: VERSION,
    mode: isCI ? 'ci' : 'local',
    passed: errors.length === 0,
    total: results.length,
    passed_count: results.length - errors.length,
    failed_count: errors.length,
    results,
    errors,
  };

  const dir = path.dirname(reportPath);
  if (dir && !fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2), 'utf-8');
  console.log(`  📄 Report written to: ${reportPath}`);
}

function writeSarif(results, errors) {
  const sarifPath = process.env.ENV_VALIDATE_SARIF;
  if (!sarifPath) return;

  const runs = [];
  if (errors.length > 0) {
    const rulesDef = [
      {
        id: 'env-missing',
        name: 'EnvironmentVariableMissing',
        shortDescription: { text: 'Required environment variable is missing' },
        defaultConfiguration: { level: 'error' },
      },
      {
        id: 'env-mismatch',
        name: 'EnvironmentVariableMismatch',
        shortDescription: { text: 'Environment variable value does not match expected' },
        defaultConfiguration: { level: 'error' },
      },
      {
        id: 'env-pattern-fail',
        name: 'EnvironmentVariablePatternFail',
        shortDescription: { text: 'Environment variable does not match required pattern' },
        defaultConfiguration: { level: 'error' },
      },
    ];

    const sarifErrors = errors.map((e) => {
      let ruleId;
      let msg;
      if (e.issue === 'missing') {
        ruleId = 'env-missing';
        msg = `Environment variable '${e.var}' is missing`;
      } else if (e.issue === 'mismatch') {
        ruleId = 'env-mismatch';
        msg = `Environment variable '${e.var}' expected '${e.expected || ''}' but got '${e.got || ''}'`;
      } else {
        ruleId = 'env-pattern-fail';
        msg = `Environment variable '${e.var}' does not match pattern '${e.pattern || ''}'`;
      }
      return {
        ruleId,
        message: { text: msg },
        locations: [
          {
            physicalLocation: {
              artifactLocation: { uri: '.env-required', index: 0 },
              region: { startLine: 1 },
            },
          },
        ],
      };
    });

    runs.push({
      tool: {
        driver: {
          name: 'env-manager-lite',
          version: VERSION,
          informationUri: 'https://github.com/wonderfulvchaum/env-manager-lite',
          rules: rulesDef,
        },
      },
      results: sarifErrors,
    });
  }

  const sarif = {
    '$schema': 'https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json',
    version: '2.1.0',
    runs,
  };

  const dir = path.dirname(sarifPath);
  if (dir && !fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(sarifPath, JSON.stringify(sarif, null, 2), 'utf-8');
  console.log(`  📄 SARIF report written to: ${sarifPath}`);
}

function main() {
  const isCI = process.env.GITHUB_ACTIONS === 'true' || process.argv.includes('--ci');

  console.log('==============================================');
  console.log(`  Env Manager Lite - Node.js Validator v${VERSION}`);
  console.log(`  Mode: ${isCI ? 'CI' : 'Local'}`);
  console.log('==============================================');

  // 1. 加载 .env 文件
  const envFile = process.env.ENV_VALIDATE_ENV_FILE;
  if (envFile) {
    const imported = loadEnvFile(envFile);
    console.log(`  📦 Loaded .env file: ${envFile} (${imported} variable(s) imported)`);
  }

  // 2. 解析规则
  const rules = parseRules();
  const typeCounts = {};
  for (const spec of Object.values(rules)) {
    typeCounts[spec.type] = (typeCounts[spec.type] || 0) + 1;
  }
  console.log(
    `  Checking ${Object.keys(rules).length} rule(s): ` +
    `${typeCounts.required || 0} required, ` +
    `${typeCounts.match || 0} match, ` +
    `${typeCounts.default || 0} default, ` +
    `${typeCounts.pattern || 0} pattern...\n`
  );

  // 3. 执行校验
  const { results, errors } = validate(rules);

  // 4. 输出报告
  writeReport(rules, { results, errors }, isCI);
  writeSarif(results, errors);

  console.log();
  if (errors.length > 0) {
    console.log(`  ✗ Failed: ${errors.length} issue(s).`);
    const missing = errors.filter((e) => e.issue === 'missing').map((e) => e.var);
    const mismatched = errors.filter((e) => e.issue === 'mismatch').map((e) => e.var);
    const patternFailed = errors.filter((e) => e.issue === 'pattern_fail').map((e) => e.var);
    if (missing.length) console.log(`    Missing: ${missing.join(', ')}`);
    if (mismatched.length) console.log(`    Mismatch: ${mismatched.join(', ')}`);
    if (patternFailed.length) console.log(`    Pattern fail: ${patternFailed.join(', ')}`);
    process.exit(1);
  } else {
    console.log('  ✓ All checks passed!');
    process.exit(0);
  }
}

main();
