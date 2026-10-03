# 🛡️ Env Manager Lite

[![CI Status](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml/badge.svg)](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 轻量级跨语言环境变量验证工具。基于 GitHub Actions CI/CD，零依赖部署，确保应用上线前拥有所有必需的环境变量。

## ✨ 特性

- 🚀 **零依赖部署**：基于 GitHub Actions，无需额外服务器
- 🌐 **五语言支持**：Bash / Python / Node.js / Go / Rust 验证脚本
- ⚙️ **可配置规则**：通过 `.env-required` 文件定义必填变量，支持 `VAR=value` 精确匹配
- 🔒 **安全优先**：GitHub Secrets 管理敏感信息，验证输出自动脱敏
- 🐳 **Docker 容器化**：多语言运行时一体化镜像，按需选择验证模式
- 📝 **模板化**：提供 `.env.example`，方便团队协作

## 🛠️ 快速开始

### 1. 配置 Secrets

在 GitHub 仓库 **Settings > Secrets and variables > Actions** 中添加：
- `DB_HOST`: 数据库地址
- `API_KEY`: API 密钥

### 2. 定义验证规则

编辑项目根目录的 `.env-required`：

```text
# 必填变量（必须存在且非空）
DB_HOST
API_KEY

# 可选：精确匹配值（存在时校验）
DB_PORT=5432

# 带默认值（不存在时自动注入，存在则保留原值）
LOG_LEVEL:info
```

支持 `#` 注释和三种条目类型：
- `VAR` — 必填，缺失即失败
- `VAR=value` — 精确匹配，缺失或值不符均失败
- `VAR:default` — 带默认值，缺失时自动注入

### 3. 本地验证

```bash
# 设置变量
export DB_HOST=localhost
export API_KEY=***

# 运行任一验证器
python examples/validate.py
node examples/validate.js
bash examples/validate.sh
go run examples/validate.go

# 或使用 CLI 自动检测（依次执行所有可用验证器）
python scripts/env-validate.py
```

### 4. CI 自动运行

推送代码后，GitHub Actions 自动运行多语言验证。也可手动触发：
**Actions** → **Environment Variable Manager** → **Run workflow**

### 5. Docker 使用

```bash
# 构建
docker build -t env-validator .

# 运行 Python 验证（默认）
docker run --rm -e DB_HOST=localhost -e API_KEY=secret123 env-validator

# 运行 Go 验证
docker run --rm -e DB_HOST=localhost -e API_KEY=secret123 env-validator go

# 运行 Rust 验证
docker run --rm -e DB_HOST=localhost -e API_KEY=secret123 env-validator rust

# 查看帮助
docker run --rm env-validator help
```

## 📂 项目结构

```text
.
├── .github/workflows/
│   ├── env-manager.yml      # 核心 CI/CD 工作流（多语言验证 + Docker 构建推送）
│   ├── linter.yml          # Super-Linter 代码检查
│   └── release-please.yml  # 自动版本管理
├── examples/
│   ├── validate.py         # Python 验证器
│   ├── validate.js         # Node.js 验证器
│   ├── validate.sh         # Bash 验证器
│   ├── validate.go         # Go 验证器
│   └── validate.rs         # Rust 验证器
├── scripts/
│   └── env-validate.py    # CLI 入口（多语言自动检测）
├── .env-required           # 验证规则配置
├── .env.example            # 环境变量模板
├── Dockerfile              # 多语言运行时容器
├── docker-compose.yml      # 本地开发编排
├── entrypoint.sh           # 容器入口
└── README.md
```

## 🗺️ 路线图

- [x] 基础 CI/CD 工作流
- [x] 多语言支持 (Bash, Python, Node.js, Go, Rust)
- [x] Docker 容器化支持
- [x] 自定义变量规则文件 (`.env-required`)
- [x] 支持 `default` 默认值回退
- [x] JSON 报告输出（`ENV_VALIDATE_REPORT` 环境变量）
- [x] 集成 Slack/Discord 通知（可选 secrets）
- [x] SARIF 报告格式（GitHub Code Scanning 集成）
- [x] 多语言自动检测 CLI（`scripts/env-validate.py`）
- [ ] 规则文件继承/分层支持

## 🤝 贡献

欢迎 Issue 和 PR！见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 📄 许可证

MIT - 见 [LICENSE](LICENSE)
