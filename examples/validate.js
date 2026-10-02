/**
 * Env Manager Lite - Node.js Validator
 *
 * .env-required 语法：
 *   纯变量名（DB_HOST）：必须存在且非空
 *   VAR=value（DB_PORT=5432）：存在则校验值匹配，不存在则跳过（可选）
 */

const fs = require('fs');
const path = require('path');

function loadRules() {
  const root = path.resolve(__dirname, '..');
  const configPath = path.join(root, '.env-required');

  const defaultRequired = ['DB_HOST', 'API_KEY'];
  const defaultOptional = [];

  if (!fs.existsSync(configPath)) {
    return { required: [...defaultRequired], optional: defaultOptional };
  }

  const lines = fs.readFileSync(configPath, 'utf-8').split('\n');
  const required = [];
  const optional = [];

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    if (line.includes('=')) {
      const [varName, val] = line.split('=');
      if (varName.trim()) {
        optional.push({ name: varName.trim(), expected: val.trim() });
      }
    } else {
      required.push(line);
    }
  }

  if (required.length === 0 && optional.length === 0) {
    required.push(...defaultRequired);
  }

  return { required, optional };
}

function main() {
  const isCI = process.env.GITHUB_ACTIONS === 'true';

  console.log('==============================================');
  console.log('  Env Manager Lite - Node.js Validator');
  console.log(`  Mode: ${isCI ? 'CI' : 'Local'}`);
  console.log('==============================================');

  const { required, optional } = loadRules();
  console.log(`  Checking ${required.length} required + ${optional.length} optional...\n`);

  const missing = [];
  const mismatches = [];

  for (const varName of required) {
    const value = process.env[varName] || '';
    if (!value) {
      missing.push(varName);
      console.log(`  ✗ Missing: ${varName}`);
    } else {
      const display = value.length > 8 ? `${value.slice(0, 4)}...` : value;
      console.log(`  ✓ Found: ${varName} = ${display}`);
    }
  }

  for (const { name, expected } of optional) {
    const value = process.env[name] || '';
    if (!value) {
      console.log(`  ○ Optional: ${name} (not set)`);
    } else if (value !== expected) {
      mismatches.push(name);
      console.log(`  ✗ Mismatch: ${name} (expected '${expected}', got '${value}')`);
    } else {
      console.log(`  ✓ Match: ${name} = ${value}`);
    }
  }

  console.log();
  const totalErrors = missing.length + mismatches.length;
  if (totalErrors > 0) {
    console.log(`  ✗ Failed: ${totalErrors} issue(s).`);
    if (missing.length) console.log(`    Missing: ${missing.join(', ')}`);
    if (mismatches.length) console.log(`    Mismatch: ${mismatches.join(', ')}`);
    process.exit(1);
  } else {
    console.log('  ✓ All checks passed!');
    process.exit(0);
  }
}

main();
