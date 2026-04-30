# Claude Code 动态模型路由代理网关 — 需求与实施方案

## Context

Claude Code 通过 `/model opus`、`/model sonnet`、`/model haiku` 切换模型，本质上只改变了请求体中的 `model` 字段。所有请求都发往同一个 `ANTHROPIC_BASE_URL`，不支持为不同模型配不同端点。

**本项目核心**：构建一个本地代理网关，将 Claude Code 的 `ANTHROPIC_BASE_URL` 指向本地代理（如 `http://127.0.0.1:9876`），代理根据请求中的 `model` 字段动态路由到不同 LLM 厂商（各自独立 base\_url + api\_key），实现「切换模型 = 切换厂商」。同时支持 Anthropic 格式和 OpenAI 格式的上游厂商，代理负责双向格式转换。

## 技术栈

- **桌面框架**：Tauri 2.0（Rust 后端 + React 前端）
- **代理服务器**：axum（嵌入 Tauri 进程，非 sidecar）
- **HTTP 客户端**：reqwest（含 SSE 流式支持）
- **前端 UI**：React + TypeScript + shadcn/ui + Tailwind CSS
- **IPC 通信**：Tauri Commands + Channel API（流式推送）
- **本地数据库**：SQLite（rusqlite）— 日志持久化 + 统计分析

***

## 一、需求规格

### 1.1 核心功能：模型路由代理

```
Claude Code CLI
    │
    │  ANTHROPIC_BASE_URL=http://127.0.0.1:9876
    │  POST /v1/messages  { model: "claude-opus-4-6", ... }
    ▼
┌──────────────────────────────────────────────────────┐
│               代理网关 (Tauri App)                    │
│                                                      │
│  ① 读取请求体中的 model 字段                          │
│  ② 按关键词匹配 → 确定目标厂商                        │
│  ③ 若上游是 OpenAI 格式 → 转换请求体（Anthropic→OpenAI）│
│  ④ 注入该厂商的 api_key，改写 base_url                 │
│  ⑤ 转发请求 + 流式回传响应                            │
│  ⑥ 若上游是 OpenAI 格式 → 转换响应（OpenAI→Anthropic）  │
│                                                      │
│  路由规则（用户在 UI 配置）：                          │
│    model 包含 "opus"   → 厂商 A（OpenAI 格式）        │
│    model 包含 "sonnet" → 厂商 B（Anthropic 格式）      │
│    model 包含 "haiku"  → 厂商 C（OpenAI 格式）        │
└──────────────────────────────────────────────────────┘
    │        │        │
    ▼        ▼        ▼
┌──────┐ ┌──────┐ ┌──────┐
│厂商A  │ │厂商B  │ │厂商C  │
│OpenAI│ │Anthro│ │OpenAI│
│格式  │ │格式   │ │格式  │
└──────┘ └──────┘ └──────┘
```

### 1.2 厂商配置管理

每个厂商槽位支持以下字段：

| 字段       | 说明                                    | 示例                       |
| -------- | ------------------------------------- | ------------------------ |
| 厂商名称     | 显示用标签                                 | "OpenAI Opus"            |
| Base URL | API 端点根地址                             | `https://api.openai.com` |
| API Key  | 鉴权密钥                                  | `sk-xxx...`              |
| 协议格式     | `anthropic` 或 `openai`                | `openai`                 |
| 模型名映射    | 发送给上游的 model 字符串                      | `gpt-4o`（可留空，原样转发）       |
| 认证头格式    | `x-api-key` 或 `authorization: Bearer` | `Bearer`                 |
| 启用状态     | 是否启用该路由                               | true / false             |

持久化：JSON 文件（`~/.claude-dynamic-meteor/providers.json`），API Key 加密后存储。

### 1.3 模型路由规则

在请求体 `model` 字段中按**关键词匹配**确定目标厂商：

| Claude Code 模型名     | 匹配关键词    | 默认路由到  |
| ------------------- | -------- | ------ |
| `claude-opus-4-6`   | `opus`   | 厂商 A   |
| `claude-sonnet-4-6` | `sonnet` | 厂商 B   |
| `claude-haiku-4-5`  | `haiku`  | 厂商 C   |
| 自定义模型               | 用户自定义关键词 | 用户指定厂商 |

路由优先级：精确匹配 > 关键词匹配 > 第一个启用的默认厂商。

### 1.4 协议格式适配（核心难点）

代理必须支持两种上游格式，并在 Claude Code 和上游之间做双向转换。

#### 1.4.1 请求转换（Anthropic → OpenAI）

Claude Code 发送的是 Anthropic Messages API 格式：

```
POST /v1/messages
{
  "model": "claude-opus-4-6",
  "max_tokens": 8192,
  "system": "You are a helpful assistant",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "tools": [...],
  "stream": true
}
```

当上游是 OpenAI 格式时，需转换为：

```
POST /v1/chat/completions
{
  "model": "gpt-4o",                    // 使用厂商配置的模型名映射
  "max_tokens": 8192,
  "messages": [
    {"role": "system", "content": "You are a helpful assistant"},  // system 提升为 message
    {"role": "user", "content": "Hello"}
  ],
  "tools": [...],                        // 需转换 tool 格式（见下）
  "stream": true,
  "stream_options": {"include_usage": true}
}
```

**关键字段映射**：

| Anthropic 字段                                     | OpenAI 字段                                      | 转换逻辑                                                                                                                      |
| ------------------------------------------------ | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `system`（顶层）                                     | `messages[0].role=system`                      | 提升为第一条 system message                                                                                                     |
| `messages[].content`（string 或 content\_block\[]） | `messages[].content`（string 或 parts\[]）        | content\_block 需展平                                                                                                        |
| `messages[].role=user/assistant`                 | `messages[].role=user/assistant`               | 一致，但 Anthropic 无 `system` role                                                                                            |
| `max_tokens`                                     | `max_tokens`                                   | 直接映射                                                                                                                      |
| `tools[].name/description/input_schema`          | `tools[].function.name/description/parameters` | 嵌套结构变化                                                                                                                    |
| `tool_choice`                                    | `tool_choice`                                  | 格式不同：`{"type":"auto"}` ↔ `{"type":"auto"}` / `{"type":"tool","name":"x"}` ↔ `{"type":"function","function":{"name":"x"}}` |
| `stop_sequences`                                 | `stop`                                         | 直接映射                                                                                                                      |
| `metadata`                                       | 无对应                                            | 丢弃                                                                                                                        |

**Anthropic content\_block → OpenAI content parts 映射**：

| Anthropic content\_block                                     | OpenAI content part                                                               |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `{"type":"text","text":"..."}`                               | `{"type":"text","text":"..."}`                                                    |
| `{"type":"image","source":{...}}`                            | `{"type":"image_url","image_url":{...}}`                                          |
| `{"type":"tool_use","id":"...","name":"...","input":{...}}`  | `{"type":"function_call","id":"...","function":{"name":"...","arguments":"..."}}` |
| `{"type":"tool_result","tool_use_id":"...","content":"..."}` | `{"role":"tool","tool_call_id":"...","content":"..."}`（需拆为独立 message）             |

#### 1.4.2 响应转换（OpenAI → Anthropic）

**非流式响应**：

OpenAI 返回：

```json
{
  "id": "chatcmpl-xxx",
  "choices": [{
    "message": {"role": "assistant", "content": "Hello!", "tool_calls": [...]},
    "finish_reason": "stop"
  }],
  "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
}
```

需转换为 Anthropic 格式：

```json
{
  "id": "msg-xxx",
  "type": "message",
  "role": "assistant",
  "content": [{"type":"text","text":"Hello!"}],
  "model": "claude-opus-4-6",
  "stop_reason": "end_turn",
  "usage": {"input_tokens": 10, "output_tokens": 5}
}
```

**流式响应（SSE）— 最复杂的部分**：

OpenAI SSE 格式：

```
data: {"id":"chatcmpl-xxx","choices":[{"delta":{"content":"Hello"},"index":0}]}
data: {"id":"chatcmpl-xxx","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_xxx","function":{"name":"fn","arguments":"{}"}}]},"index":0}]}
data: {"id":"chatcmpl-xxx","choices":[{"delta":{},"finish_reason":"stop"}]}
data: [DONE]
```

需实时转换为 Anthropic SSE 格式：

```
event: message_start
data: {"type":"message_start","message":{"id":"msg-xxx","type":"message","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}
```

**流式转换状态机**：

```
[收到第一个 OpenAI chunk]
  → 发送 message_start + content_block_start

[收到 content delta]
  → 发送 content_block_delta

[检测到 tool_calls delta]
  → 发送 content_block_start(type=tool_use) + content_block_delta(type=input_json_delta)

[收到 finish_reason=stop]
  → 发送 content_block_stop + message_delta(stop_reason=end_turn) + message_stop

[收到 finish_reason=tool_calls]
  → 发送 content_block_stop + message_delta(stop_reason=tool_use) + message_stop
```

#### 1.4.3 Anthropic 格式上游

对于 Anthropic 格式的上游（如 vLLM、NVIDIA NIM 等兼容服务），**不做任何转换**，直接透传请求和响应。

### 1.5 SSE 流式转发架构

```
Claude Code ──POST /v1/messages──► axum handler
                                       │
                                       ├─ 解析 model，匹配厂商
                                       ├─ 若 OpenAI 格式 → 转换请求体
                                       │
                                       ▼
                                  reqwest::post(upstream_url)
                                       │
                                       ▼
                                  bytes_stream() 逐块读取
                                       │
                                  ┌────┴────┐
                                  │         │
                             Anthropic    OpenAI
                             格式上游     格式上游
                                  │         │
                              直接透传   SSE 帧解析 + 格式转换
                                  │         │
                                  └────┬────┘
                                       │
                                  Body::from_stream()
                                       │
                                       ▼
                                  Claude Code 接收 SSE 响应
```

关键：

- Anthropic 格式上游：**零解析透传**，逐字节转发，延迟最小
- OpenAI 格式上游：需要**逐 SSE 帧解析→转换→重新编码**为 Anthropic SSE 格式
- 两种路径都使用 bounded mpsc channel 做背压控制

### 1.6 本地代理服务器生命周期

- **启动/停止**：用户通过 UI 按钮控制，或 Tauri 启动时自动启动
- **端口配置**：默认 `9876`，可自定义
- **路由端点**：
  - `POST /v1/messages` — 代理主入口
  - `GET /v1/models` — 返回已配置的可用模型列表（供 Claude Code 查询）
  - `GET /health` — 健康检查
- **仅监听 127.0.0.1**：不暴露到局域网
- **系统托盘**：最小化到托盘后台运行

### 1.7 Claude Code 对接方式

**核心原理**：Claude Code 读取 `ANTHROPIC_BASE_URL` 环境变量，所有 API 请求发往该地址。

网关提供「一键对接」功能：

1. 自动检测 `~/.claude/settings.json` 中是否已配置 `ANTHROPIC_BASE_URL`
2. 提供一键写入：将 `ANTHROPIC_BASE_URL=http://127.0.0.1:9876` 写入 settings
3. 提供一键还原：移除自定义配置，恢复默认
4. 启动时检测：若网关已启动但 Claude Code 未指向本地代理，UI 提示用户

需要写入的配置项：

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:9876",
    "ANTHROPIC_API_KEY": "placeholder",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "true"
  }
}
```

注意：不需要设置 `ANTHROPIC_DEFAULT_OPUS_MODEL` 等——这些保持默认即可，因为模型名匹配在代理侧完成。

### 1.8 请求监控、日志与统计（SQLite）

所有请求日志、token 用量、成本数据持久化到本地 SQLite 数据库（`~/.claude-dynamic-meteor/meteor.db`），重启不丢失。

#### 1.8.1 数据表设计

```sql
-- 请求日志
CREATE TABLE request_logs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id    TEXT NOT NULL,           -- 唯一请求 ID
    timestamp     TEXT NOT NULL,           -- ISO 8601
    model         TEXT NOT NULL,           -- Claude Code 发送的 model 名
    provider_id   TEXT NOT NULL,           -- 路由到的厂商 ID
    provider_name TEXT NOT NULL,           -- 厂商显示名（冗余，方便查询）
    protocol      TEXT NOT NULL,           -- "anthropic" 或 "openai"
    upstream_url  TEXT NOT NULL,           -- 实际上游地址
    status_code   INTEGER,                -- HTTP 状态码
    latency_ms    INTEGER,                -- 端到端耗时（ms）
    input_tokens  INTEGER DEFAULT 0,       -- 输入 token 数
    output_tokens INTEGER DEFAULT 0,       -- 输出 token 数
    error_message TEXT,                   -- 错误信息（若有）
    is_streaming  BOOLEAN DEFAULT TRUE,   -- 是否流式
    created_at    TEXT DEFAULT (datetime('now'))
);

-- 每日统计汇总（定时聚合，加速 Dashboard 查询）
CREATE TABLE daily_stats (
    date          TEXT PRIMARY KEY,        -- YYYY-MM-DD
    total_requests INTEGER DEFAULT 0,
    total_errors   INTEGER DEFAULT 0,
    total_input_tokens  INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    avg_latency_ms REAL DEFAULT 0,
    -- 按厂商/模型的细粒度统计以 JSON 存储，避免过度拆表
    provider_breakdown TEXT DEFAULT '{}',  -- {"provider_a": {requests:10, tokens:500}, ...}
    model_breakdown    TEXT DEFAULT '{}'   -- {"opus": {requests:5, tokens:200}, ...}
);

-- 厂商配置（也持久化到 SQLite，替代 JSON 文件）
CREATE TABLE providers (
    id            TEXT PRIMARY KEY,        -- UUID
    name          TEXT NOT NULL,
    base_url      TEXT NOT NULL,
    api_key_enc   TEXT NOT NULL,           -- 加密存储
    protocol      TEXT NOT NULL DEFAULT 'anthropic',
    model_mapping TEXT,                    -- 发送给上游的 model 名
    auth_header   TEXT NOT NULL DEFAULT 'x-api-key',
    keyword       TEXT NOT NULL,           -- 路由关键词
    enabled       BOOLEAN DEFAULT TRUE,
    sort_order    INTEGER DEFAULT 0,
    created_at    TEXT DEFAULT (datetime('now')),
    updated_at    TEXT DEFAULT (datetime('now'))
);
```

#### 1.8.2 实时推送 + 持久化双通道

```
请求完成
  │
  ├─► INSERT INTO request_logs          ← 持久化到 SQLite
  │
  └─► Channel::send(LogEvent)           ← 实时推送到 React 前端
```

- **Channel 推送**：每个请求完成后，通过 Tauri Channel 实时推送摘要到前端，前端即时更新日志列表和统计数字
- **SQLite 写入**：异步写入，不阻塞代理转发。使用 `tokio::task::spawn_blocking` 将 SQLite 写入移到独立线程
- **历史查询**：前端通过 Tauri Command 调用 `get_logs(filter)` 从 SQLite 查询，支持按时间范围、厂商、模型、状态码筛选

#### 1.8.3 统计 Dashboard 指标

| 指标          | 数据来源                             | 展示形式       |
| ----------- | -------------------------------- | ---------- |
| 今日请求总数      | `daily_stats`                    | 数字卡片       |
| 今日错误率       | `daily_stats`                    | 数字卡片 + 趋势线 |
| 今日 Token 用量 | `daily_stats`（input + output）    | 数字卡片       |
| 平均延迟        | `daily_stats`                    | 数字卡片       |
| 按厂商请求分布     | `daily_stats.provider_breakdown` | 饼图/条形图     |
| 按模型请求分布     | `daily_stats.model_breakdown`    | 饼图/条形图     |
| 7 天趋势       | `daily_stats` 近 7 行              | 折线图        |
| 实时活跃请求      | Channel 推送                       | 状态指示灯      |

#### 1.8.4 数据管理

- **自动清理**：保留最近 90 天日志，更早的自动删除（Tauri 启动时执行）
- **手动导出**：支持导出为 JSON/CSV
- **手动清除**：提供清除全部日志按钮（需确认）
- **数据库迁移**：使用 `rusqlite` 内建的 `user_version` 做版本管理

### 1.9 错误处理

| 场景       | 处理策略                                                                                              |
| -------- | ------------------------------------------------------------------------------------------------- |
| 上游不可达    | 返回 502 + Anthropic 格式错误，UI 告警                                                                     |
| 上游超时     | 返回 504（默认 120s，可配置）                                                                               |
| 路由未匹配    | 返回 400 + `{"error":{"type":"invalid_request","message":"No provider configured for model: xxx"}}` |
| 上游返回错误   | Anthropic 格式：透传；OpenAI 格式：转换为 Anthropic 错误格式                                                      |
| 客户端断开    | 主动关闭上游连接                                                                                          |
| 格式转换失败   | 返回 500 + 错误详情，记录转换日志                                                                              |
| 代理服务器未启动 | Claude Code 侧直接报连接失败                                                                              |

### 1.10 安全性

- API Key 加密存储，不在日志中明文输出
- 代理仅监听 `127.0.0.1`
- 关闭 Claude Code 非必要遥测（`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=true`）

### 1.11 跨平台兼容性（macOS / Windows / Linux）

应用必须在三大桌面平台上一致运行。以下是各平台差异点及处理策略：

| 功能                      | macOS                                                  | Windows                                     | Linux                              | 处理策略                                                       |
| ----------------------- | ------------------------------------------------------ | ------------------------------------------- | ---------------------------------- | ---------------------------------------------------------- |
| API Key 安全存储            | Keychain (`security-framework`)                        | Credential Manager (`keyring` crate)        | Secret Service API / 文件加密          | 统一使用 `keyring` crate，它封装了三平台原生密钥存储                         |
| 系统托盘                    | `tauri tray-icon` feature                              | `tauri tray-icon` feature                   | `tauri tray-icon` feature          | Tauri 2 统一 API，无需额外处理                                      |
| 开机自启                    | LaunchAgent plist                                      | Registry Run key                            | XDG autostart `.desktop`           | 使用 `tauri-plugin-autostart`                                |
| WebView 引擎              | WKWebView（系统自带）                                        | WebView2（需引导安装）                             | WebKitGTK（需系统安装）                   | Windows 上 Tauri 安装器自动引导 WebView2 Bootstrapper；Linux 文档注明依赖 |
| 数据目录                    | `~/Library/Application Support/claude-dynamic-meteor/` | `%APPDATA%/claude-dynamic-meteor/`          | `~/.config/claude-dynamic-meteor/` | 使用 `dirs` crate 的 `data_dir()` 统一获取                        |
| SQLite 路径               | 同数据目录下 `meteor.db`                                     | 同数据目录下 `meteor.db`                          | 同数据目录下 `meteor.db`                 | `rusqlite` bundled 模式确保三平台一致                               |
| Claude Code settings 路径 | `~/.claude/settings.json`                        | `%USERPROFILE%\.claude\settings.json` | `~/.claude/settings.json`    | 使用 `dirs::home_dir()` + `.claude` 拼接                       |

**关键 Rust 依赖**：

```toml
keyring = "3"                       # 三平台统一 Keychain/Credential Manager/Secret Service
dirs = "5"                          # 跨平台标准目录路径
tauri-plugin-autostart = "2"        # 三平台开机自启
```

**Linux 额外说明**：

- WebKitGTK 需用户系统安装（`libwebkit2gtk-4.1-dev`），在 README 中注明
- Secret Service API 需要 GNOME Keyring 或 KDE Wallet 运行，若无则降级为文件加密存储

***

## 二、UI 布局与页面设计

> 设计实现时使用 `web-design-guidelines` skill 进行合规审查

### 2.0 整体布局架构

```
┌─────────────────────────────────────────────────────────┐
│ ◉ Claude Dynamic Meteor              ─  □  ×  (标题栏)  │
├────────┬────────────────────────────────────────────────┤
│        │  Header: 代理状态 ● Running  │ Port: 9876     │
│        ├────────────────────────────────────────────────┤
│  侧边栏 │                                                │
│        │                                                │
│ 📊 仪表盘│           主内容区                             │
│ 🔀 厂商  │          （根据侧边栏选择切换）                  │
│ 📋 日志  │                                                │
│ 🔗 对接  │                                                │
│ ⚙ 设置  │                                                │
│        │                                                │
│        │                                                │
│        ├────────────────────────────────────────────────┤
│        │  底部状态栏: 今日请求 42 │ Token 15.2k │ 延迟 45ms│
└────────┴────────────────────────────────────────────────┘
```

**布局特征**：

- 左侧固定侧边栏（56px 折叠 / 200px 展开），图标 + 文字
- 顶部 Header 显示代理实时状态（绿色/红色/灰色指示灯）
- 底部状态栏显示全局统计摘要
- 主内容区自适应，最小宽度 800px

### 2.1 仪表盘页面（Dashboard）

```
┌────────────────────────────────────────────────┐
│  仪表盘                                         │
├──────┬──────┬──────┬──────┬──────────────────────┤
│请求数 │错误率 │Token │平均  │                      │
│  142 │ 2.1% │15.2k │ 45ms│                      │
│ ↑12% │ ↓0.5%│↑8%  │ ↓3ms│                      │
├──────┴──────┴──────┴──────┴──────────────────────┤
│                                                  │
│  7天请求趋势（折线图）                              │
│  ┌──────────────────────────────────────────┐    │
│  │    ╱╲     ╱╲                              │    │
│  │   ╱  ╲   ╱  ╲   ╱╲                       │    │
│  │  ╱    ╲ ╱    ╲ ╱  ╲                      │    │
│  │ ╱      ╲      ╲╱    ╲                    │    │
│  │╱                           ╲              │    │
│  └──────────────────────────────────────────┘    │
│  Mon  Tue  Wed  Thu  Fri  Sat  Sun              │
│                                                  │
├──────────────────────┬───────────────────────────┤
│  厂商请求分布（环形图） │  模型请求分布（条形图）      │
│  ┌──────────────┐    │  ┌──────────────────────┐│
│  │    ╭──╮      │    │  │ ████████  Opus  58   ││
│  │   │ A  │     │    │  │ ██████    Sonnet 42  ││
│  │    ╰──╯      │    │  │ ███       Haiku  22  ││
│  └──────────────┘    │  └──────────────────────┘│
├──────────────────────┴───────────────────────────┤
│  实时活跃请求                                     │
│  ● opus → OpenAI (gpt-4o)  正在流式传输... 1.2s   │
│  ● sonnet → Anthropic      正在流式传输... 0.8s   │
└──────────────────────────────────────────────────┘
```

**组件拆分**：

- `StatsCards`：4 个数字卡片（请求数、错误率、Token 用量、平均延迟），每个带同比变化箭头
- `TrendChart`：7 天折线图（recharts 或 visx，显示请求量 + 延迟双轴）
- `DistributionChart`：左侧环形图（按厂商），右侧水平条形图（按模型）
- `ActiveRequests`：当前正在进行的请求列表（Channel 实时推送），显示模型、目标厂商、已耗时

### 2.2 厂商管理页面（Providers）

```
┌────────────────────────────────────────────────┐
│  厂商管理                        [+ 添加厂商]    │
├────────────────────────────────────────────────┤
│                                                │
│  ┌────────────────────────────────────────┐    │
│  │ 🟢 OpenAI Opus                   编辑 删除│    │
│  │ Base: https://api.openai.com           │    │
│  │ 格式: OpenAI  │  映射: gpt-4o           │    │
│  │ 关键词: opus  │  认证: Bearer           │    │
│  │ 今日: 58 请求  │  平均延迟: 52ms         │    │
│  └────────────────────────────────────────┘    │
│                                                │
│  ┌────────────────────────────────────────┐    │
│  │ 🟢 Anthropic Sonnet              编辑 删除│    │
│  │ Base: https://api.anthropic.com        │    │
│  │ 格式: Anthropic │  映射: (直通)          │    │
│  │ 关键词: sonnet │  认证: X-Api-Key       │    │
│  │ 今日: 42 请求  │  平均延迟: 38ms         │    │
│  └────────────────────────────────────────┘    │
│                                                │
│  ┌────────────────────────────────────────┐    │
│  │ 🔴 DeepSeek Haiku                 编辑 删除│    │
│  │ Base: https://api.deepseek.com         │    │
│  │ 格式: OpenAI  │  映射: deepseek-chat    │    │
│  │ 关键词: haiku  │  认证: Bearer           │    │
│  │ 已禁用                                   │    │
│  └────────────────────────────────────────┘    │
│                                                │
└────────────────────────────────────────────────┘
```

**厂商配置弹窗（Dialog）** — 点击「添加厂商」或「编辑」：

```
┌──────────────────────────────────────────────┐
│  编辑厂商                                     │
├──────────────────────────────────────────────┤
│                                              │
│  厂商名称   [OpenAI Opus                   ] │
│                                              │
│  Base URL   [https://api.openai.com        ] │
│                                              │
│  API Key    [sk-••••••••••••••••]    [测试]   │
│                                              │
│  协议格式   ◉ Anthropic  ○ OpenAI            │
│                                              │
│  模型名映射 [gpt-4o          ] (留空=原样转发) │
│                                              │
│  认证头格式 ◉ X-Api-Key  ○ Bearer Token      │
│                                              │
│  路由关键词  [opus                          ] │
│                                              │
│  ☑ 启用                                     │
│                                              │
├──────────────────────────────────────────────┤
│             [取消]  [保存]                     │
└──────────────────────────────────────────────┘
```

**组件拆分**：

- `ProviderList`：厂商卡片列表，支持拖拽排序（调整优先级）
- `ProviderCard`：单厂商卡片，显示核心信息 + 今日统计 + 状态开关
- `ProviderForm`：shadcn Dialog + Form，含字段验证
- 「测试连接」按钮：向 `base_url/v1/models`（或 `/v1/messages` 发送一个极短请求）验证连通性和 API Key 有效性

### 2.3 请求日志页面（Logs）

```
┌────────────────────────────────────────────────┐
│  请求日志                                       │
├────────────────────────────────────────────────┤
│  筛选:                                          │
│  厂商[全部 ▾] 模型[全部 ▾] 状态[全部 ▾] 日期[今天▾]│
│                              [导出 CSV] [清除]   │
├────┬─────────┬──────┬──────┬──────┬─────┬──────┤
│时间│ Model   │ 厂商  │ 格式 │状态码│耗时 │Token │
├────┼─────────┼──────┼──────┼──────┼─────┼──────┤
│14:2│opus-4-6 │OpenAI│openai│ 200  │52ms │1.2k │
│14:1│sonnet-6 │Anthro│anthro│ 200  │38ms │0.8k │
│14:0│haiku-5  │DeepSk│openai│ 502  │---  │  0  │
│13:5│opus-4-6 │OpenAI│openai│ 200  │61ms │2.1k │
└────┴─────────┴──────┴──────┴──────┴─────┴──────┘
  < 1  2  3  4  5  ...  12 >     共 284 条

点击行展开详情：
┌────────────────────────────────────────────────┐
│ ▼ 14:23 opus-4-6 → OpenAI (gpt-4o)            │
│   请求体: {"model":"claude-opus-4-6","messages":│
│            [{"role":"user","content":"..."}]}   │
│   转换后: {"model":"gpt-4o","messages":[...]}   │
│   响应: 200 OK, 52ms, 输出 1200 tokens          │
│   SSE 事件数: 48                                │
└────────────────────────────────────────────────┘
```

**组件拆分**：

- `RequestLog`：shadcn DataTable，支持筛选、排序、分页
- `LogDetail`：行展开面板，显示请求体/响应体/转换详情
- `StreamViewer`：在 LogDetail 中，可回放该请求的 SSE 事件流
- 筛选器：shadcn Select 组件，数据从 SQLite 查询

### 2.4 Claude Code 对接页面（Integration）

```
┌────────────────────────────────────────────────┐
│  Claude Code 对接                               │
├────────────────────────────────────────────────┤
│                                                │
│  连接状态                                       │
│  ┌────────────────────────────────────────┐    │
│  │  🟢 代理已启动 (127.0.0.1:9876)        │    │
│  │  🟡 Claude Code 未指向本地代理          │    │
│  │                                        │    │
│  │  当前 ANTHROPIC_BASE_URL:              │    │
│  │  https://api.anthropic.com (默认)       │    │
│  │                                        │    │
│  │  [一键对接 Claude Code]                 │    │
│  └────────────────────────────────────────┘    │
│                                                │
│  对接配置详情                                    │
│  ┌────────────────────────────────────────┐    │
│  │  将写入 ~/.claude/settings.json:  │    │
│  │                                        │    │
│  │  {                                     │    │
│  │    "env": {                            │    │
│  │      "ANTHROPIC_BASE_URL":            │    │
│  │        "http://127.0.0.1:9876",       │    │
│  │      "ANTHROPIC_API_KEY":             │    │
│  │        "placeholder",                 │    │
│  │      "CLAUDE_CODE_DISABLE_...":       │    │
│  │        "true"                         │    │
│  │    }                                   │    │
│  │  }                                     │    │
│  │                                        │    │
│  │  [还原默认配置]                          │    │
│  └────────────────────────────────────────┘    │
│                                                │
│  使用说明                                       │
│  ┌────────────────────────────────────────┐    │
│  │  1. 点击「一键对接」写入配置             │    │
│  │  2. 重启 Claude Code（或在会话中执行     │    │
│  │     /model 重新连接）                   │    │
│  │  3. 在 Claude Code 中执行：            │    │
│  │     /model opus   → 路由到 OpenAI      │    │
│  │     /model sonnet → 路由到 Anthropic   │    │
│  │     /model haiku  → 路由到 DeepSeek    │    │
│  └────────────────────────────────────────┘    │
│                                                │
└────────────────────────────────────────────────┘
```

**组件拆分**：

- `ClaudeIntegration`：检测当前连接状态 + 一键操作 + 使用说明
- 自动检测逻辑：读取 `~/.claude/settings.json`，检查 `ANTHROPIC_BASE_URL` 是否指向本地代理端口

### 2.5 设置页面（Settings）

```
┌────────────────────────────────────────────────┐
│  设置                                          │
├────────────────────────────────────────────────┤
│                                                │
│  代理配置                                       │
│  端口号     [9876                             ] │
│  启动时自动开启代理  ☑                           │
│  请求超时(秒)     [120                          ] │
│                                                │
│  系统集成                                       │
│  开机自启         ☐                             │
│  最小化到系统托盘   ☑                            │
│  关闭窗口时       ◉ 最小化到托盘  ○ 退出应用      │
│                                                │
│  数据管理                                       │
│  日志保留天数     [90                           ] │
│  [导出所有日志 (JSON)]  [导出所有日志 (CSV)]       │
│  [清除所有日志]                                  │
│                                                │
│  安全                                           │
│  API Key 存储方式  ◉ 系统密钥存储  ○ 加密文件     │
│                                                │
└────────────────────────────────────────────────┘
```

***

## 三、实施方案

### 3.1 项目结构

```
claude-dynamic-meteor/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   └── src/
│       ├── main.rs                  # Tauri 入口
│       ├── lib.rs                   # Tauri 模块注册 + setup
│       ├── proxy/
│       │   ├── mod.rs
│       │   ├── server.rs            # axum server 启停、路由注册
│       │   ├── handler.rs           # POST /v1/messages 处理入口
│       │   ├── router.rs            # model → provider 路由匹配
│       │   └── stream.rs            # SSE 流式转发（Anthropic 透传 + OpenAI 转换）
│       ├── adapter/
│       │   ├── mod.rs               # 适配器 trait 定义
│       │   ├── anthropic.rs         # Anthropic 直通适配器（无转换）
│       │   ├── openai.rs            # OpenAI ↔ Anthropic 格式转换
│       │   ├── request.rs           # Anthropic 请求 → OpenAI 请求
│       │   └── response.rs          # OpenAI 响应 → Anthropic 响应（含 SSE 流式状态机）
│       ├── config/
│       │   ├── mod.rs
│       │   ├── provider.rs          # Provider 数据结构
│       │   └── store.rs             # 厂商配置持久化（SQLite providers 表）
│       ├── claude/
│       │   ├── mod.rs
│       │   └── settings.rs          # Claude Code settings.json 读写
│       ├── commands/
│       │   ├── mod.rs               # Tauri Commands 注册
│       │   ├── server_cmd.rs        # start_proxy / stop_proxy / proxy_status
│       │   ├── provider_cmd.rs      # CRUD 厂商配置
│       │   ├── log_cmd.rs           # 日志查询（从 SQLite 读取）
│       │   ├── stats_cmd.rs         # 统计数据查询
│       │   └── claude_cmd.rs        # inject_claude_config / revert_claude_config
│       ├── db/
│       │   ├── mod.rs               # SQLite 初始化 + 迁移
│       │   ├── logs.rs              # request_logs 表 CRUD
│       │   ├── stats.rs             # daily_stats 聚合 + 查询
│       │   └── migration.rs         # 数据库 schema 版本管理
│       └── monitor/
│           └── mod.rs               # 实时 Channel 推送 + SQLite 写入协调
├── src/
│   ├── App.tsx                      # 主布局（侧边栏 + 内容区）
│   ├── main.tsx                     # React 入口
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx          # 侧边导航
│   │   │   └── Header.tsx           # 顶栏（代理状态指示灯 + 连接状态）
│   │   ├── dashboard/
│   │   │   ├── DashboardPage.tsx    # 仪表盘主页
│   │   │   ├── StatsCards.tsx       # 统计数字卡片组
│   │   │   ├── TrendChart.tsx       # 7 天趋势折线图
│   │   │   ├── DistributionChart.tsx # 按厂商/模型分布图
│   │   │   └── ActiveRequests.tsx   # 实时活跃请求列表
│   │   ├── server/
│   │   │   └── ServerControl.tsx    # 代理启停 + 端口配置
│   │   ├── providers/
│   │   │   ├── ProviderList.tsx     # 厂商列表
│   │   │   ├── ProviderCard.tsx     # 厂商卡片（状态、快速切换）
│   │   │   └── ProviderForm.tsx     # 厂商配置表单（shadcn Dialog）
│   │   ├── logs/
│   │   │   ├── RequestLog.tsx       # 请求日志表格（可筛选、分页）
│   │   │   ├── LogDetail.tsx        # 单条日志详情（展开查看请求/响应体）
│   │   │   └── StreamViewer.tsx     # 实时流式预览
│   │   ├── claude/
│   │   │   └── ClaudeIntegration.tsx # Claude Code 对接状态 + 一键配置
│   │   └── ui/                      # shadcn/ui 组件
│   │       ├── button.tsx
│   │       ├── card.tsx
│   │       ├── dialog.tsx
│   │       ├── input.tsx
│   │       ├── select.tsx
│   │       ├── switch.tsx
│   │       ├── table.tsx
│   │       ├── toast.tsx
│   │       └── badge.tsx
│   ├── hooks/
│   │   ├── useProxyChannel.ts       # Tauri Channel 监听（实时日志 + 活跃请求）
│   │   ├── useProviders.ts          # 厂商配置 CRUD
│   │   ├── useProxyServer.ts        # 代理启停控制
│   │   └── useStats.ts             # 统计数据查询
│   ├── lib/
│   │   ├── tauri.ts                 # invoke / Channel 工具封装
│   │   └── types.ts                 # TypeScript 类型定义
│   └── styles/
│       └── globals.css              # Tailwind + shadcn 样式
├── components.json                   # shadcn/ui 配置
├── package.json
├── tailwind.config.js
├── tsconfig.json
└── vite.config.ts
```

### 3.2 核心 Rust 类型

```rust
/// 厂商配置
struct Provider {
    id: String,                       // UUID
    name: String,                     // 显示名
    base_url: String,                 // 上游 API 地址
    api_key: String,                  // 加密存储，运行时解密
    protocol: Protocol,               // Anthropic 或 OpenAI
    model_mapping: Option<String>,    // 模型名映射，如 "gpt-4o"
    auth_header: AuthHeader,          // X-Api-Key 或 Bearer
    keyword: String,                  // 路由关键词，如 "opus"
    enabled: bool,
}

enum Protocol { Anthropic, OpenAI }
enum AuthHeader { ApiKey, Bearer }

/// 适配器 trait
#[async_trait]
trait LlmAdapter: Send + Sync {
    /// 将 Anthropic 请求转换为上游格式并转发
    async fn forward_request(
        &self,
        request: AnthropicRequest,
        provider: &Provider,
        client: &reqwest::Client,
    ) -> Result<ProxyResponse>;

    /// 返回该适配器支持的协议
    fn protocol(&self) -> Protocol;
}

/// 代理响应（统一内部表示）
enum ProxyResponse {
    /// Anthropic 格式，直接透传字节流
    AnthropicStream(Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>),
    /// OpenAI 格式，需要逐帧转换
    OpenAIStream(Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>),
}
```

### 3.3 SSE 流式转换状态机（OpenAI → Anthropic）

这是最复杂的组件。核心是一个有状态的事件转换器：

```rust
struct OpenAIToAnthropicConverter {
    message_id: String,               // 生成的 msg-xxx ID
    model: String,                    // 保留原始 model 名
    content_block_index: u32,         // 当前 content block 索引
    in_tool_call: bool,               // 是否正在处理 tool_calls
    input_tokens: u32,                // 累计 input tokens
    output_tokens: u32,               // 累计 output tokens
    buffer: String,                   // SSE 行缓冲（处理跨 chunk 的 SSE 帧）
}

impl OpenAIToAnthropicConverter {
    /// 输入一个 OpenAI SSE chunk（原始字节），输出零或多个 Anthropic SSE 帧
    fn convert_chunk(&mut self, chunk: &[u8]) -> Vec<AnthropicSseFrame> {
        // 1. 缓冲 + 按行拆分 SSE 事件
        // 2. 解析 data: {...} 为 OpenAI ChatChunk
        // 3. 根据状态机生成 Anthropic SSE 帧
        //    - 首个 chunk → message_start + content_block_start
        //    - delta.content → content_block_delta(text_delta)
        //    - delta.tool_calls → content_block_start(tool_use) + input_json_delta
        //    - finish_reason → content_block_stop + message_delta + message_stop
    }
}
```

### 3.4 依赖清单

**Cargo.toml 关键依赖**：

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
axum = "0.7"
reqwest = { version = "0.12", features = ["stream", "json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
tokio-stream = "0.1"
futures = "0.3"
uuid = { version = "1", features = ["v4"] }
async-trait = "0.1"
rusqlite = { version = "0.31", features = ["bundled"] }  # SQLite（bundled 确保跨平台一致）
keyring = "3"                       # 三平台统一 Keychain/Credential Manager/Secret Service
dirs = "5"                          # 跨平台标准目录路径
aes-gcm = "0.10"                    # API Key 加密（keyring 不可用时的降级方案）
base64 = "0.22"
```

**package.json 关键依赖**：

```json
{
  "@tauri-apps/api": "^2",
  "@tauri-apps/plugin-shell": "^2",
  "react": "^18",
  "react-dom": "^18",
  "tailwindcss": "^3",
  "lucide-react": "^0.400",
  "class-variance-authority": "^0.7",
  "clsx": "^2",
  "tailwind-merge": "^2",
  "recharts": "^2.12"
}
```

***

## 四、分阶段实施计划

### 阶段 1：项目骨架 + Anthropic 直通 + SQLite 基础

- [x] Tauri 2 项目初始化（`tauri init`）
- [x] shadcn/ui 配置（Vite + React + Tailwind）
- [x] Rust 端 SQLite 初始化（schema 建表、迁移机制）
- [x] Provider 数据结构 + SQLite 持久化（providers 表）
- [x] axum server 启停（Tauri Command 控制）
- [x] `POST /v1/messages` Anthropic 格式直通（零转换透传）
- [x] model 关键词路由匹配（含单元测试）
- [x] 请求日志写入 SQLite（request\_logs 表）
- [ ] React 前端：代理启停 + 厂商配置 CRUD（shadcn 表单）— **进行中**

**阶段 1 完成度：88% (8/9)**

### 阶段 2：OpenAI 格式适配

- [x] 请求转换：Anthropic Messages → OpenAI Chat Completions（\~800 行代码）
- [x] 非流式响应转换：OpenAI → Anthropic
- [x] 流式 SSE 状态机：OpenAI chunks → Anthropic SSE 事件（含多 tool\_call 处理）
- [x] Provider 配置增加 `protocol` 字段（anthropic / openai）
- [ ] UI 厂商配置弹窗增加协议格式选择 — **待实现**

**阶段 2 完成度：80% (4/5)**

### 阶段 3：监控 + 仪表盘 + Claude Code 对接

- [x] 请求日志实时推送（Tauri Channel → React）— 架构已实现
- [x] 日志页面（shadcn DataTable，筛选/分页/展开详情）
- [x] daily\_stats 聚合计算 + 仪表盘统计卡片（读时聚合）
- [x] 7 天趋势图 + 厂商/模型分布图（recharts）
- [x] Claude Code settings.json 检测 + 一键对接/还原
- [x] `GET /v1/models` 端点 + `GET /health` 健康检查
- [x] 错误处理 + UI toast 告警（sonner）

**阶段 3 完成度：100%** ✅

### 阶段 4：工具调用 + 跨平台 + 体验优化

- [x] tool\_use / function\_call 格式映射（请求 + 响应 + 流式）— 已在 OpenAI 适配器中实现
- [x] keyring crate 集成（三平台安全存储 API Key）
- [x] 系统托盘 + 后台运行 + 开机自启
- [x] 日志导出（JSON/CSV）— 命令已实现
- [x] 设置页面完善

**阶段 4 完成度：100%** ✅

***

## 当前项目整体进度：**96%**

### 已完成核心功能：

- ✅ Tauri 2 + React + SQLite 架构搭建
- ✅ axum 代理服务器（端口冲突自动递增）
- ✅ Anthropic 格式直通（旁路 usage 提取）
- ✅ OpenAI 格式双向转换（请求/响应/SSE 流式状态机，\~800 行）
- ✅ 模型路由匹配（精确/关键词/降级，含单元测试）
- ✅ SQLite 持久化（providers/request\_logs/daily\_stats）
- ✅ Tauri Commands（server/provider/log/stats/claude/settings/autostart）
- ✅ Claude Code 一键对接/还原
- ✅ 应用设置持久化
- ✅ shadcn/ui 组件库（12+ 组件）
- ✅ React Hooks（6 个自定义 hooks）
- ✅ 完整的前端页面（Dashboard/Providers/Logs/Integration/Settings）
- ✅ recharts 图表（趋势图 + 分布图）
- ✅ Toast 通知（sonner）
- ✅ keyring 安全存储（三平台支持 + AES-256-GCM 降级）
- ✅ 健康检查端点（/health + /v1/models）
- ✅ 系统托盘（最小化到托盘后台运行）
- ✅ 开机自启（tauri-plugin-autostart 集成）
- ✅ 代码优化（移除 dead\_code 警告）
- ✅ 统一深色主题（移除主题切换功能）

### 待完成任务（可选）：

1. **打包分发**：macOS .dmg / Windows .msi / Linux .AppImage（已生成 debug 版本）
2. **生产构建**：运行 `npm run tauri build` 生成 release 版本

***

## 五、验证方案

1. **单元测试**：
   - `router::match_provider()` — 各种模型名匹配场景
   - `openai::convert_request()` — Anthropic 请求 → OpenAI 请求
   - `openai::convert_sse_chunk()` — OpenAI SSE → Anthropic SSE 逐帧验证
   - `db::logs` — SQLite 写入 + 查询 + 聚合
2. **集成测试**（Rust 端 mock server）：
   - 启动 mock Anthropic 上游 → 代理直通 → 验证响应完整
   - 启动 mock OpenAI 上游 → 代理转换 → 验证 Anthropic 格式输出正确
   - 验证日志正确写入 SQLite
3. **端到端测试**：
   - 配置 3 个厂商（2 个 OpenAI 格式 mock + 1 个 Anthropic 格式 mock）
   - `ANTHROPIC_BASE_URL=http://127.0.0.1:9876`
   - Claude Code 执行 `/model opus` → 验证路由到 OpenAI 格式厂商 A，响应被正确转换
   - `/model sonnet` → Anthropic 格式厂商 B，直通验证
   - `/model haiku` → OpenAI 格式厂商 C，转换验证
   - 验证仪表盘统计数据与实际请求一致
4. **流式延迟测试**：
   - Anthropic 直通路径：<10ms 额外延迟
   - OpenAI 转换路径：<50ms 额外延迟（帧解析+重构开销）
5. **跨平台测试**：
   - macOS：Keychain 存储 API Key、系统托盘、开机自启
   - Windows：Credential Manager 存储 API Key、WebView2 兼容
   - Linux：Secret Service 存储 API Key、WebKitGTK 渲染一致性
6. **真实厂商测试**：
   - 对接 OpenAI API（gpt-4o）验证流式转换
   - 对接 DeepSeek API 验证 OpenAI 兼容格式
   - 对接 vLLM 本地部署验证 Anthropic 兼容格式

