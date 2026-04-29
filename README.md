# Claude Dynamic Meteor

本地代理网关，让 Claude Code 通过 `/model opus|sonnet|haiku` 动态路由到不同 LLM 厂商。

## 🎯 核心功能

- **动态模型路由**：根据模型名关键词自动路由到不同厂商
- **双协议支持**：同时支持 Anthropic 和 OpenAI 格式的上游厂商
- **格式自动转换**：OpenAI ↔ Anthropic 请求/响应双向转换（含 SSE 流式）
- **安全存储**：API Key 使用系统 Keyring（macOS Keychain / Windows Credential Manager / Linux Secret Service）+ AES-256-GCM 降级
- **请求监控**：实时日志、统计分析、7 天趋势图
- **智能集成**：启动代理时自动配置 Claude Code，关闭时自动还原
- **系统托盘**：后台运行，支持开机自启

## 🚀 快速开始

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
pnpm tauri dev
```

### 构建

```bash
pnpm build
pnpm tauri build
```

## 📖 使用说明

### 1. 启动应用

运行应用后，在右上角点击「启动代理」按钮：
- 代理服务器默认监听 `http://127.0.0.1:9876`
- 同时自动配置 Claude Code 使用此代理

### 2. 配置厂商

在「厂商」页面添加 LLM 厂商：

- **厂商名称**：显示用标签（如 "OpenAI Opus"）
- **Base URL**：API 端点（如 `https://api.openai.com`）
- **API Key**：鉴权密钥（自动加密存储）
- **协议格式**：`anthropic` 或 `openai`
- **模型名映射**：发送给上游的 model 字符串（可留空原样转发）
- **认证头格式**：`x-api-key` 或 `bearer`
- **路由关键词**：模型名匹配关键词（如 `opus`, `sonnet`, `haiku`）

### 3. 使用 Claude Code

在 Claude Code 中执行：

```bash
/model opus    # 路由到关键词包含 "opus" 的厂商
/model sonnet  # 路由到关键词包含 "sonnet" 的厂商
/model haiku   # 路由到关键词包含 "haiku" 的厂商
```

### 4. 关闭代理

点击右上角「停止代理」按钮：
- 停止代理服务器
- 自动还原 Claude Code 的原始配置

## 🏗️ 技术架构

### 后端（Rust + Tauri 2.0）

- **代理服务器**：axum（嵌入 Tauri 进程）
- **HTTP 客户端**：reqwest（SSE 流式支持）
- **数据库**：SQLite（rusqlite bundled）
- **安全存储**：keyring + aes-gcm
- **系统托盘**：tauri tray-icon
- **开机自启**：tauri-plugin-autostart
- **格式转换**：
  - Anthropic 格式：零解析透传（旁路提取 usage）
  - OpenAI 格式：请求/响应双向转换 + SSE 流式状态机

### 前端（React + TypeScript）

- **框架**：React 18 + Vite
- **UI 组件**：shadcn/ui + Tailwind CSS
- **路由**：react-router-dom (HashRouter)
- **图表**：recharts
- **通知**：sonner
- **图标**：lucide-react

### 路由优先级

1. **精确匹配**：`model === keyword`
2. **边界关键词匹配**：`(^|-)keyword(-|$)` 正则
3. **降级**：第一个启用的默认厂商

## 📊 功能特性

### 已实现（100%）

- ✅ Tauri 2 + React + SQLite 架构
- ✅ axum 代理服务器（端口冲突自动递增）
- ✅ Anthropic 格式直通（旁路 usage 提取）
- ✅ OpenAI 格式双向转换
- ✅ 模型路由匹配（含单元测试）
- ✅ SQLite 持久化（providers/request_logs/daily_stats）
- ✅ 完整的前端页面（Dashboard/Providers/Logs/Settings）
- ✅ recharts 图表（趋势图 + 分布图）
- ✅ Toast 通知
- ✅ keyring 安全存储（三平台支持）
- ✅ 健康检查端点（/health + /v1/models）
- ✅ Claude Code 自动配置/还原（与代理开关联动）
- ✅ 日志导出（JSON/CSV）
- ✅ 系统托盘
- ✅ 开机自启
- ✅ 现代化 UI 设计（shadcn/ui）

## 🔒 安全性

- API Key 优先使用系统 Keyring 存储
- 降级使用 AES-256-GCM 加密文件存储
- 代理仅监听 `127.0.0.1`（不暴露到局域网）
- 关闭 Claude Code 非必要遥测

## 📝 开发日志

详见 [PRD-PLAN.md](./PRD-PLAN.md)

## 📄 许可证

MIT License - see the [LICENSE](./LICENSE) file for details.
