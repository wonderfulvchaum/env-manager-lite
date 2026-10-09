//go:build ignore
// +build ignore

// Env Manager Lite - Go Validator v0.7
// 三种规则类型: required / match / default
// 规则分层: .env-required.d/*.conf 按字母序追加/覆盖
// .env 文件加载: ENV_VALIDATE_ENV_FILE
// JSON 报告: 设置 ENV_VALIDATE_REPORT 后生成
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"strings"
)

type Rule struct {
	Type     string `json:"type"`
	Expected string `json:"expected,omitempty"`
	Default  string `json:"default,omitempty"`
}

type RuleResult struct {
	Var    string `json:"var"`
	Status string `json:"status"`
	Value  string `json:"value,omitempty"`
}

type RuleError struct {
	Var      string `json:"var"`
	Issue    string `json:"issue"`
	Expected string `json:"expected,omitempty"`
	Got      string `json:"got,omitempty"`
}

type Report struct {
	Tool        string     `json:"tool"`
	Version     string     `json:"version"`
	Mode        string     `json:"mode"`
	Passed      bool       `json:"passed"`
	Total       int        `json:"total"`
	PassedCount int        `json:"passed_count"`
	FailedCount int        `json:"failed_count"`
	Results     []RuleResult `json:"results"`
	Errors      []RuleError  `json:"errors"`
}

func main() {
	isCI := os.Getenv("GITHUB_ACTIONS") == "true"
	mode := "local"
	if isCI {
		mode = "ci"
	}

	fmt.Println("==============================================")
	fmt.Println("  Env Manager Lite - Go Validator v0.7")
	fmt.Printf("  Mode: %s\n", strings.ToUpper(mode))
	fmt.Println("==============================================")

	// 加载 .env 文件
	if envFile := os.Getenv("ENV_VALIDATE_ENV_FILE"); envFile != "" {
		if data, err := os.ReadFile(envFile); err == nil {
			imported := 0
			for _, raw := range strings.Split(string(data), "\n") {
				line := strings.TrimSpace(raw)
				if line == "" || strings.HasPrefix(line, "#") {
					continue
				}
				if strings.HasPrefix(line, "export ") {
					line = line[7:]
				}
				idx := strings.Index(line, "=")
				if idx < 0 {
					continue
				}
				key := strings.TrimSpace(line[:idx])
				val := strings.TrimSpace(line[idx+1:])
				val = strings.Trim(val, `'"`)
				if key != "" && os.Getenv(key) == "" {
					os.Setenv(key, val)
					imported++
				}
			}
			fmt.Printf("  📦 Loaded .env file: %s (%d variable(s) imported)\n", envFile, imported)
		}
	}

	rules := loadRules()
	typeCounts := map[string]int{}
	for _, r := range rules {
		typeCounts[r.Type]++
	}
	fmt.Printf("  Checking %d rule(s): %d required, %d match, %d default...\n\n",
		len(rules), typeCounts["required"], typeCounts["match"], typeCounts["default"])

	results, errors := validate(rules)
	writeReport(rules, results, errors, mode)

	fmt.Println()
	if len(errors) > 0 {
		fmt.Printf("  ✗ Failed: %d issue(s).\n", len(errors))
		var missing, mismatched []string
		for _, e := range errors {
			if e.Issue == "missing" {
				missing = append(missing, e.Var)
			} else {
				mismatched = append(mismatched, e.Var)
			}
		}
		if len(missing) > 0 {
			fmt.Printf("    Missing: %s\n", strings.Join(missing, ", "))
		}
		if len(mismatched) > 0 {
			fmt.Printf("    Mismatch: %s\n", strings.Join(mismatched, ", "))
		}
		os.Exit(1)
	}
	fmt.Println("  ✓ All checks passed!")
}

// loadRules 解析 .env-required + .env-required.d/*.conf（项目根目录）
// VAR           → required
// VAR=expected  → match
// VAR:default   → default（无 = 时）
func loadRules() map[string]Rule {
	root := findRoot()

	defaults := map[string]Rule{
		"DB_HOST": {Type: "required"},
		"API_KEY": {Type: "required"},
	}

	rules := map[string]Rule{}
	found := false

	// 主文件
	configPath := root + "/.env-required"
	if data, err := os.ReadFile(configPath); err == nil {
		for _, raw := range strings.Split(string(data), "\n") {
			if varName, rule, ok := parseRuleLine(raw); ok {
				rules[varName] = rule
				found = true
			}
		}
	}

	// 分层目录
	rulesDir := root + "/.env-required.d"
	if dir, err := os.ReadDir(rulesDir); err == nil {
		for _, entry := range dir {
			if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".conf") {
				continue
			}
			data, err := os.ReadFile(path.Join(rulesDir, entry.Name()))
			if err != nil {
				continue
			}
			for _, raw := range strings.Split(string(data), "\n") {
				if varName, rule, ok := parseRuleLine(raw); ok {
					rules[varName] = rule
					found = true
				}
			}
		}
	}

	if !found {
		return defaults
	}
	return rules
}

// parseRuleLine 解析单行，成功时返回 true
func parseRuleLine(raw string) (string, Rule, bool) {
	line := strings.TrimSpace(raw)
	if line == "" || strings.HasPrefix(line, "#") {
		return "", Rule{}, false
	}
	// 优先级: 有 : 且无 = → default; 有 = → match; 否则 → required
	if strings.Contains(line, ":") && !strings.Contains(line, "=") {
		parts := strings.SplitN(line, ":", 2)
		vars := strings.TrimSpace(parts[0])
		if vars == "" {
			return "", Rule{}, false
		}
		defaultVal := ""
		if len(parts) > 1 {
			defaultVal = strings.TrimSpace(parts[1])
		}
		return vars, Rule{Type: "default", Default: defaultVal}, true
	} else if strings.Contains(line, "=") {
		parts := strings.SplitN(line, "=", 2)
		vars := strings.TrimSpace(parts[0])
		if vars == "" {
			return "", Rule{}, false
		}
		expected := ""
		if len(parts) > 1 {
			expected = strings.TrimSpace(parts[1])
		}
		return vars, Rule{Type: "match", Expected: expected}, true
	} else {
		return line, Rule{Type: "required"}, true
	}
}


func validate(rules map[string]Rule) ([]RuleResult, []RuleError) {
	var results []RuleResult
	var errors []RuleError

	// 稳定顺序: 按 map 迭代（Go 1.12+ map 迭代顺序随机，用切片排序）
	keys := make([]string, 0, len(rules))
	for k := range rules {
		keys = append(keys, k)
	}
	// 简单排序保持输出稳定
	for i := 0; i < len(keys); i++ {
		for j := i + 1; j < len(keys); j++ {
			if keys[i] > keys[j] {
				keys[i], keys[j] = keys[j], keys[i]
			}
		}
	}

	for _, varName := range keys {
		spec := rules[varName]
		current := os.Getenv(varName)

		switch spec.Type {
		case "default":
			if current == "" {
				// 注入默认值
				os.Setenv(varName, spec.Default)
				current = spec.Default
				results = append(results, RuleResult{Var: varName, Status: "defaulted", Value: current})
				fmt.Printf("  ◑ Defaulted: %s = %s\n", varName, current)
			} else {
				results = append(results, RuleResult{Var: varName, Status: "ok", Value: current})
				fmt.Printf("  ✓ Found: %s = %s\n", varName, mask(current))
			}

		case "match":
			if current == "" {
				errors = append(errors, RuleError{Var: varName, Issue: "missing"})
				results = append(results, RuleResult{Var: varName, Status: "missing"})
				fmt.Printf("  ✗ Missing: %s (required)\n", varName)
			} else if current != spec.Expected {
				errors = append(errors, RuleError{Var: varName, Issue: "mismatch", Expected: spec.Expected, Got: current})
				results = append(results, RuleResult{Var: varName, Status: "mismatch"})
				fmt.Printf("  ✗ Mismatch: %s (expected '%s', got '%s')\n", varName, spec.Expected, current)
			} else {
				results = append(results, RuleResult{Var: varName, Status: "ok", Value: current})
				fmt.Printf("  ✓ Match: %s = %s\n", varName, current)
			}

		default: // required
			if current == "" {
				errors = append(errors, RuleError{Var: varName, Issue: "missing"})
				results = append(results, RuleResult{Var: varName, Status: "missing"})
				fmt.Printf("  ✗ Missing: %s\n", varName)
			} else {
				results = append(results, RuleResult{Var: varName, Status: "ok", Value: current})
				fmt.Printf("  ✓ Found: %s = %s\n", varName, mask(current))
			}
		}
	}

	return results, errors
}

func writeReport(rules map[string]Rule, results []RuleResult, errors []RuleError, mode string) {
	reportPath := os.Getenv("ENV_VALIDATE_REPORT")
	if reportPath == "" {
		return
	}

	passed := len(errors) == 0
	report := Report{
		Tool:        "env-manager-lite",
		Version:     "0.7.0",
		Mode:        mode,
		Passed:      passed,
		Total:       len(results),
		PassedCount: len(results) - len(errors),
		FailedCount: len(errors),
		Results:     results,
		Errors:      errors,
	}

	data, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return
	}
	if err := os.WriteFile(reportPath, data, 0644); err == nil {
		fmt.Printf("  📄 Report written to: %s\n", reportPath)
	}
}

func mask(v string) string {
	if len(v) > 8 {
		return v[:4] + "..."
	}
	return v
}

func findRoot() string {
	// 向上查找 .env-required 或 git 目录
	dir, _ := os.Getwd()
	for i := 0; i < 5; i++ {
		if _, err := os.Stat(dir + "/.env-required"); err == nil {
			return dir
		}
		if _, err := os.Stat(dir + "/.git"); err == nil {
			return dir
		}
		parent := dir + "/.."
		if parent == dir {
			break
		}
		dir = parent
	}
	return "."
}
