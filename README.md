# Env Manager Lite 🔐

一个基于 GitHub Actions 的轻量级环境变量管理演示项目。旨在帮助开源开发者理解如何安全地处理敏感配置信息，避免硬编码密码。

## 🚀 功能特点

- **安全存储**: 利用 GitHub Secrets 存储敏感数据。
- **自动验证**: 通过 CI/CD 流程自动检查必要的环境变量是否配置。
- **模板生成**: 自动生成 `.env.example` 文件，方便团队成员协作。
- **零依赖**: 无需安装额外软件，直接在 GitHub 网页端运行。

## 🛠️ 如何使用

### 1. 配置 Secrets
前往本仓库的 **Settings** -> **Secrets and variables** -> **Actions**，添加以下 New repository secrets：
- `DB_HOST`: 你的数据库地址
- `API_KEY`: 你的 API 密钥

### 2. 运行工作流
- 进入 **Actions** 标签页。
- 选择 **Environment Variable Manager**。
- 点击 **Run workflow** 按钮。

### 3. 查看结果
工作流将验证你的 Secret 是否存在，并生成标准的 `.env.example` 文件结构。

## 🤝 为什么需要 1Password?

虽然 GitHub Secrets 适合 CI/CD，但在本地开发环境中，开发者仍然需要安全地共享和同步这些变量。本项目推荐配合 **1Password** 使用：
- 在团队间安全共享 `.env` 文件内容。 
- 自动填充本地开发环境。
- 审计谁在何时访问了哪些密钥。

## 📜 License
MIT License
