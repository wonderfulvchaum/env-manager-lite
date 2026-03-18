# 🛡️ Env Manager Lite

[![CI Status](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml/badge.svg)](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)](https://github.com/wonderfulvchaum/env-manager-lite/releases)

> 一个轻量级的跨语言环境变量验证工具，专为 GitHub Actions 和本地开发设计。确保你的应用在部署前拥有所有必需的环境变量。

## ✨ 特性

- 🚀 **零依赖**：无需安装额外软件，原生支持 Bash。
- 🐍 **多语言支持**：提供 Python 和 Node.js 验证脚本示例。
- 🔄 **CI/CD 集成**：自动在 GitHub Actions 中运行验证，防止配置错误导致部署失败。
- 📝 **自动生成**：自动创建 `.env.example` 模板供团队成员参考。
- 🛡️ **安全优先**：基于 GitHub Secrets 管理敏感信息，不硬编码。

## 🚀 快速开始

### 1. 配置 GitHub Secrets
在你的仓库设置中 (`Settings` > `Secrets and variables` > `Actions`) 添加以下变量：
- `DB_HOST`: 你的数据库地址
- `API_KEY`: 你的 API 密钥

### 2. 本地验证 (可选)
如果你想在提交代码前在本地检查环境变量，可以使用我们提供的脚本。

#### 🐚 Bash (Linux/Mac)
```bash
chmod +x examples/validate.sh
./examples/validate.sh
