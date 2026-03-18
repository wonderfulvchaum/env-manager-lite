# 🛡️ Env Manager Lite

[![CI Status](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml/badge.svg)](https://github.com/wonderfulvchaum/env-manager-lite/actions/workflows/env-manager.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 一个轻量级的跨语言环境变量验证工具，专为 GitHub Actions CI/CD 流程设计。确保你的应用在部署前拥有所有必需的环境变量。

## ✨ 特性

- 🚀 **零依赖部署**：基于 GitHub Actions，无需额外服务器。
- 🐍 **多语言支持**：原生支持 Bash, Python, Node.js 验证脚本。
- 🔒 **安全优先**：利用 GitHub Secrets 管理敏感信息，永不泄露。
- 📝 **自动生成**：自动创建 `.env.example` 模板，方便团队协作。
- 🔄 **自动化流程**：每次 Push 或 PR 自动运行检查，防止配置错误上线。

## 🛠️ 快速开始

### 1. 配置 Secrets
在你的 GitHub 仓库中，进入 **Settings** > **Secrets and variables** > **Actions**，添加以下变量：
- `DB_HOST`: 你的数据库地址
- `API_KEY`: 你的 API 密钥

### 2. 添加工作流
将本仓库的 `.github/workflows/env-manager.yml` 复制到你的项目中，或参考本仓库配置。

### 3. 运行验证
当你推送代码时，GitHub Actions 会自动运行验证。你也可以手动触发：
1. 进入 **Actions** 标签页。
2. 选择 **Environment Variable Manager**。
3. 点击 **Run workflow**。

## 💻 多语言示例

本项目提供了三种语言的本地验证脚本，你可以直接在本地运行它们来测试环境变量配置。

### 🐚 Bash (Shell)
适用于大多数 Linux/Mac 环境。

```bash
# 设置变量
export DB_HOST=localhost
export API_KEY=secret123

# 运行验证
chmod +x examples/validate.sh
./examples/validate.sh
```

### 🐍 Python
适用于 Python 开发者，包含更详细的错误处理。

**前置要求**: Python 3.x

```bash
# 设置变量 (Linux/Mac)
export DB_HOST=localhost
export API_KEY=secret123

# 设置变量 (Windows PowerShell)
$env:DB_HOST="localhost"
$env:API_KEY="secret123"

# 运行验证
python examples/validate.py
```

**输出示例**:
```text
🐍 Python Environment Validator Started...
✅ Found: DB_HOST
✅ Found: API_KEY
🎉 Success: All required variables are present!
```

### 🟨 Node.js
适用于 JavaScript/TypeScript 项目。

**前置要求**: Node.js 14+

```bash
# 设置变量 (Linux/Mac)
export DB_HOST=localhost
export API_KEY=secret123

# 运行验证
node examples/validate.js
```

**输出示例**:
```text
🟨 Node.js Environment Validator Started...
✅ Found: DB_HOST
✅ Found: API_KEY
🎉 Success: All variables present!
```

## 📂 项目结构

```text
.
├── .github
│   └── workflows
│       └── env-manager.yml      # 核心 CI/CD 工作流
├── examples
│   ├── validate.sh              # Bash 验证脚本
│   ├── validate.py              # Python 验证脚本
│   └── validate.js              # Node.js 验证脚本
├── .env.example                 # 环境变量模板
├── CONTRIBUTING.md              # 贡献指南
└── README.md                    # 项目说明
```

## 🗺️ 路线图

- [x] 基础 CI/CD 工作流
- [x] 多语言支持 (Bash, Python, Node.js)
- [ ] Docker 容器化支持 (Coming Soon)
- [ ] 支持自定义变量规则文件
- [ ] 集成 Slack/Discord 通知

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何开始。

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

 
