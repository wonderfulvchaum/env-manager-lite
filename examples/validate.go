//go:build ignore
// +build ignore

// Env Manager Lite - Go Validator
package main

import (
	"fmt"
	"os"
)

func main() {
	required := []string{"DB_HOST", "API_KEY"}

	// 尝试从 .env-required 读取
	if data, err := os.ReadFile(".env-required"); err == nil {
		required = parseEnvRequired(string(data))
	}

	fmt.Println("==============================================")
	fmt.Println("  Env Manager Lite - Go Validator")
	fmt.Println("==============================================")
	fmt.Printf("  Checking %d variable(s)...\n\n", len(required))

	missing := []string{}
	for _, v := range required {
		val := os.Getenv(v)
		if val == "" {
			missing = append(missing, v)
			fmt.Printf("  ✗ Missing: %s\n", v)
		} else {
			display := val
			if len(val) > 8 {
				display = val[:4] + "..."
			}
			fmt.Printf("  ✓ Found: %s = %s\n", v, display)
		}
	}

	fmt.Println()
	if len(missing) > 0 {
		fmt.Printf("  ✗ Failed: %d variable(s) missing.\n", len(missing))
		for _, m := range missing {
			fmt.Printf("    - %s\n", m)
		}
		os.Exit(1)
	} else {
		fmt.Println("  ✓ All required variables present!")
	}
}

func parseEnvRequired(content string) []string {
	var vars []string
	for _, line := range splitLines(content) {
		line = trimSpace(line)
		if line == "" || hasPrefix(line, "#") {
			continue
		}
		// 支持 VAR=value 语法
		if idx := indexByte(line, '='); idx > 0 {
			vars = append(vars, trimSpace(line[:idx]))
		} else {
			vars = append(vars, line)
		}
	}
	if len(vars) == 0 {
		return []string{"DB_HOST", "API_KEY"}
	}
	return vars
}

func splitLines(s string) []string {
	var lines []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' || s[i] == '\r' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	lines = append(lines, s[start:])
	return lines
}

func trimSpace(s string) string {
	for len(s) > 0 && (s[0] == ' ' || s[0] == '\t') {
		s = s[1:]
	}
	for len(s) > 0 && (s[len(s)-1] == ' ' || s[len(s)-1] == '\t') {
		s = s[:len(s)-1]
	}
	return s
}

func hasPrefix(s, prefix string) bool {
	return len(s) >= len(prefix) && s[:len(prefix)] == prefix
}

func indexByte(s string, b byte) int {
	for i := 0; i < len(s); i++ {
		if s[i] == b {
			return i
		}
	}
	return -1
}
