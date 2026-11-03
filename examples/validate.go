//go:build ignore
// +build ignore

// Env Manager Lite - Go Validator v0.8
// 四种规则类型: required / match / default / pattern
// 规则分层: .env-required.d/*.conf 按字母序追加/覆盖
// YAML 规则文件: .env-required.yaml（简化解析）
// .env 文件加载: ENV_VALIDATE_ENV_FILE
// JSON 报告: 设置 ENV_VALIDATE_REPORT 后生成
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"regexp"
	"sort"
	"strings"
)

type Rule struct {
	Type     string `json:"type"`
	Expected string `json:"expected,omitempty"`
	Default  string `json:"default,omitempty"`
	Pattern  string `json:"pattern,omitempty"`
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
	Pattern  string `json:"pattern,omitempty"`
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

// 内置快捷模式（v0.8）
var BuiltinPatterns = map[string]string{
	"int":   "^[0-9]+$",
	"float": "^[0-9]+(\\.[0-9]+)?$",
	"bool":  "^(true|false|1|0)$",
	"url":   "^https?://[^\\s/$?#]*\\.[^\\s]+$",
	"ip":    "^[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}$",
	"uuid":  "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
}

func resolvePattern(p string) string {
	if r, ok := BuiltinPatterns[p]; ok {
		return r
	}
	return p
}

func checkPattern(value, pattern string) bool {
	re := resolvePattern(pattern)
	// 剥离用户手写的锚点避免双重锚点
	re = strings.TrimPrefix(re, "^")
	re = strings.TrimSuffix(re, "$")
	// Go regex: 用 (?i) 内联标志实现忽略大小写
	if pattern == "bool" || pattern == "url" {
		re = "(?i)" + re
	}
	re = "^" + re + "$"
	matched, err := regexp.MatchString(re, value)
	if err != nil {
		return strings.Contains(value, pattern)
	}
	return matched
}

func main() {
	isCI := os.Getenv("GITHUB_ACTIONS") == "true"
	mode := "local"
	if isCI {
		mode = "ci"
	}

	fmt.Println("==============================================")
	fmt.Println("  Env Manager Lite - Go Validator v0.8")
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
	fmt.Printf("  Checking %d rule(s): %d required, %d match, %d default, %d pattern...\n\n",
		len(rules), typeCounts["required"], typeCounts["match"], typeCounts["default"], typeCounts["pattern"])

	results, errors := validate(rules)
	writeReport(rules, results, errors, mode)

	fmt.Println()
	if len(errors) > 0 {
		fmt.Printf("  ✗ Failed: %d issue(s).\n", len(errors))
		var missing, mismatched, patternFailed []string
		for _, e := range errors {
			switch e.Issue {
			case "missing":
				missing = append(missing, e.Var)
			case "mismatch":
				mismatched = append(mismatched, e.Var)
			case "pattern_fail":
				patternFailed = append(patternFailed, e.Var)
			}
		}
		if len(missing) > 0 {
			fmt.Printf("    Missing: %s\n", strings.Join(missing, ", "))
		}
		if len(mismatched) > 0 {
			fmt.Printf("    Mismatch: %s\n", strings.Join(mismatched, ", "))
		}
		if len(patternFailed) > 0 {
			fmt.Printf("    Pattern fail: %s\n", strings.Join(patternFailed, ", "))
		}
		os.Exit(1)
	}
	fmt.Println("  ✓ All checks passed!")
}

// loadRules 解析 .env-required + .env-required.d/*.conf + .env-required.yaml
func loadRules() map[string]Rule {
	root := findRoot()

	defaults := map[string]Rule{
		"DB_HOST": {Type: "required"},
		"API_KEY": {Type: "required"},
	}

	rules := map[string]Rule{}
	found := false

	// YAML 文件（简化解析）
	yamlPath := root + "/.env-required.yaml"
	if data, err := os.ReadFile(yamlPath); err == nil {
		for _, raw := range strings.Split(string(data), "\n") {
			line := strings.TrimSpace(raw)
			if line == "" || strings.HasPrefix(line, "#") {
				continue
			}
			// VAR: type
			if strings.Contains(line, ":") && !strings.HasPrefix(line, " ") {
				parts := strings.SplitN(line, ":", 2)
				varName := strings.TrimSpace(parts[0])
				val := strings.TrimSpace(parts[1])
				val = strings.Trim(val, `'"`)
				if varName != "" {
					rule := Rule{Type: "required"}
					switch val {
					case "match", "default", "pattern", "required":
						rule.Type = val
					}
					if rule.Type == "required" {
						rules[varName] = rule
						found = true
					}
				}
			}
		}
	}

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
		var names []string
		for _, entry := range dir {
			if entry.IsDir() {
				continue
			}
			if strings.HasSuffix(entry.Name(), ".conf") || strings.HasSuffix(entry.Name(), ".yaml") {
				names = append(names, entry.Name())
			}
		}
		sort.Strings(names)
		for _, name := range names {
			data, err := os.ReadFile(path.Join(rulesDir, name))
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
// 优先级: ~ > : > = > bare
func parseRuleLine(raw string) (string, Rule, bool) {
	line := strings.TrimSpace(raw)
	if line == "" || strings.HasPrefix(line, "#") {
		return "", Rule{}, false
	}
	// ~ 优先: pattern
	if strings.Contains(line, "~") {
		parts := strings.SplitN(line, "~", 2)
		varName := strings.TrimSpace(parts[0])
		pattern := ""
		if len(parts) > 1 {
			pattern = strings.TrimSpace(parts[1])
		}
		if varName != "" {
			return varName, Rule{Type: "pattern", Pattern: pattern}, true
		}
	}
	// : 且无 = → default
	if strings.Contains(line, ":") && !strings.Contains(line, "=") {
		parts := strings.SplitN(line, ":", 2)
		varName := strings.TrimSpace(parts[0])
		if varName == "" {
			return "", Rule{}, false
		}
		defaultVal := ""
		if len(parts) > 1 {
			defaultVal = strings.TrimSpace(parts[1])
		}
		return varName, Rule{Type: "default", Default: defaultVal}, true
	}
	// = → match
	if strings.Contains(line, "=") {
		parts := strings.SplitN(line, "=", 2)
		varName := strings.TrimSpace(parts[0])
		if varName == "" {
			return "", Rule{}, false
		}
		expected := ""
		if len(parts) > 1 {
			expected = strings.TrimSpace(parts[1])
		}
		return varName, Rule{Type: "match", Expected: expected}, true
	}
	// bare → required
	return line, Rule{Type: "required"}, true
}

func validate(rules map[string]Rule) ([]RuleResult, []RuleError) {
	var results []RuleResult
	var errors []RuleError

	keys := make([]string, 0, len(rules))
	for k := range rules {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	for _, varName := range keys {
		spec := rules[varName]
		current := os.Getenv(varName)

		switch spec.Type {
		case "default":
			if current == "" {
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

		case "pattern":
			if current == "" {
				errors = append(errors, RuleError{Var: varName, Issue: "missing"})
				results = append(results, RuleResult{Var: varName, Status: "missing"})
				fmt.Printf("  ✗ Missing: %s (pattern required: %s)\n", varName, spec.Pattern)
			} else if !checkPattern(current, spec.Pattern) {
				errors = append(errors, RuleError{Var: varName, Issue: "pattern_fail", Pattern: spec.Pattern, Got: current})
				results = append(results, RuleResult{Var: varName, Status: "pattern_fail"})
				fmt.Printf("  ✗ Pattern fail: %s (pattern '%s', got '%s')\n", varName, spec.Pattern, current)
			} else {
				results = append(results, RuleResult{Var: varName, Status: "ok", Value: current})
				fmt.Printf("  ✓ Pattern ok: %s matches '%s'\n", varName, spec.Pattern)
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
		Version:     "0.8.0",
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
