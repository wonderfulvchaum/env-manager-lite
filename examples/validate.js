/**
 * Env Manager Lite - Node.js Validator v0.5
 *
 * 规则文件 .env-required 三种条目类型：
 *   VAR            → 必填，缺失则失败
 *   VAR=expected   → 精确匹配
 *   VAR:default    → 带默认值，不存在时注入
 *
 * CI 报告：设置 ENV_VALIDATE_REPORT 后输出 JSON。
 */

const fs = require('fs');
const path = require('path');

const DEFAULT_RULES = {
  DB_HOST: { type: 'required' },
  API_KEY: { type: 'required' },
};

function parseRules() {
  const root = path.resolve(__dirname, '..');
  const configPath = path.join(root, '.env-required');

  if (!fs.existsSync(configPath)) {
    return { ...DEFAULT_RULES };
  }

  const lines = fs.readFileSync(configPath, 'utf-8').split('\n');
  const rules = {};
  let found = false;

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    found = true;

    if (line.includes(':') && !line.includes('=')) {
      const [varName, defaultVal] = line.split(':');
      rules[varName.trim()] = { type: 'default', default: defaultVal.trim() };
    } else if (line.includes('=')) {
      const [varName, expectedVal] = line.split('=');
      rules[varName.trim()] = { type: 'match', expected: expectedVal.trim() };
    } else {
      rules[line] = { type: 'required' };
    }
  }

  if (!found) return { ...DEFAULT_RULES };
  return rules;
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
    version: '0.5.0',
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

function main() {
  const isCI = process.env.GITHUB_ACTIONS === 'true' || process.argv.includes('--ci');

  console.log('==============================================');
  console.log('  Env Manager Lite - Node.js Validator v0.5');
  console.log(`  Mode: ${isCI ? 'CI' : 'Local'}`);
  console.log('==============================================');

  const rules = parseRules();
  const typeCounts = {};
  for (const spec of Object.values(rules)) {
    typeCounts[spec.type] = (typeCounts[spec.type] || 0) + 1;
  }
  console.log(
    `  Checking ${Object.keys(rules).length} rule(s): ` +
    `${typeCounts.required || 0} required, ` +
    `${typeCounts.match || 0} match, ` +
    `${typeCounts.default || 0} default...\n`
  );

  const { results, errors } = validate(rules);
  writeReport(rules, { results, errors }, isCI);

  console.log();
  if (errors.length > 0) {
    console.log(`  ✗ Failed: ${errors.length} issue(s).`);
    const missing = errors.filter((e) => e.issue === 'missing').map((e) => e.var);
    const mismatched = errors.filter((e) => e.issue === 'mismatch').map((e) => e.var);
    if (missing.length) console.log(`    Missing: ${missing.join(', ')}`);
    if (mismatched.length) console.log(`    Mismatch: ${mismatched.join(', ')}`);
    process.exit(1);
  } else {
    console.log('  ✓ All checks passed!');
    process.exit(0);
  }
}

main();
