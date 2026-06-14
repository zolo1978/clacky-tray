<div align="center">

<img src="src-tauri/icons/icon.png" width="120" height="120" alt="Ever-Living">

# 🌿 Ever-Living

### 你的 AI 永续助理 · Always-on, Always-remembering

**Live Green · Live Well · Live Forever**

[![macOS](https://img.shields.io/badge/platform-macOS-8FCA97?style=flat-square&logo=apple&logoColor=white)](https://github.com/zolo1978/clacky-tray)
[![Tauri](https://img.shields.io/badge/Tauri-v2-8FCA97?style=flat-square&logo=tauri&logoColor=white)](https://v2.tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-8FCA97?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-8FCA97?style=flat-square)](LICENSE)

</div>

---

> **Ever-Living** 是一个 macOS 菜单栏原生应用，让你的 AI 助理 **持续在线、永不失忆**。  
> 后台常驻 Ruby sidecar 服务，前端 WebView 嵌入完整 Web UI —— 开机即用，多会话并行，跨会话记忆持久化。

## ✨ 核心特性

| | 特性 | 说明 |
|---|---|---|
| 🟢 | **永续驻留** | 菜单栏图标常驻，全局快捷键 `⌘⇧O` 一键唤起 |
| 🧠 | **持久记忆** | SOUL / USER / Memories 三层记忆模型，跨会话延续人格 |
| 💬 | **多会话并行** | coding、copywriting、research 同时进行，互不干扰 |
| ⏰ | **定时任务** | Cron 调度，自动执行重复性工作流 |
| 🔌 | **频道接入** | 飞书 / 企业微信 / Discord / Telegram IM 双向打通 |
| 🛠️ | **技能系统** | 200+ 内置技能，自定义 Skill 一键创建 |
| 🌐 | **MCP 协议** | 兼容 Claude Desktop / Cursor 标准 MCP 服务器 |
| 🎨 | **品牌定制** | White-label 支持，自定义 Logo / 配色 / 名称 |

## 🎨 设计语言

```
Accent    #8FCA97   自然浅绿 · 取自应用图标
Hover     #6db877   深绿交互态
Dark      #a3ddac   暗色主题柔和绿
Soft      #eaf6ec   10% 透明背景填充
```

- **字体**：SF Pro Display / Outfit — 清晰现代
- **风格**：自然、持久、有生命力
- **主题**：浅色 / 暗色自动跟随系统

## 🏗️ 技术架构

```
┌─────────────────────────────────────────┐
│         Ever-Living.app (Tauri v2)       │
│  ┌─────────────┐  ┌──────────────────┐  │
│  │  Rust Core  │  │   WebView (JS)   │  │
│  │  - 托盘菜单  │◄─┤  - Web UI 嵌入   │  │
│  │  - 全局热键  │  │  - 偏好设置       │  │
│  │  - 自启管理  │  │  - IPC 通信      │  │
│  └──────┬──────┘  └──────────────────┘  │
│         │ spawn / manage                 │
│  ┌──────▼──────────────────────────────┐ │
│  │   Ruby Sidecar (ever-living server) │ │
│  │   - HTTP + WebSocket :7070          │ │
│  │   - Agent 引擎 / 记忆 / 频道        │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**技术栈**

- 🦀 **Rust** + Tauri v2 — 原生壳层，托盘 / 热键 / 自启
- 💎 **Ruby** — Sidecar 服务端，Agent 引擎
- 🎨 **Vanilla TS + Vite** — 偏好设置窗口前端
- 🔧 **reqwest / tokio / libc** — 异步进程管理

## 🚀 快速开始

### 前置依赖

```bash
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Tauri CLI
cargo install tauri-cli --version "^2.0"

# 3. Node.js 18+ & pnpm
brew install node

# 4. Ruby sidecar (ever-living gem)
gem install ever-living
```

### 开发模式

```bash
git clone git@github.com:zolo1978/clacky-tray.git
cd clacky-tray

npm install        # 安装前端依赖
npm run tauri dev  # 启动开发服务器（热更新）
```

### 构建发布

```bash
npm run tauri build -- --bundles app
# 产物: src-tauri/target/release/bundle/macos/Ever-Living.app
```

部署到 `/Applications`：

```bash
cp -R src-tauri/target/release/bundle/macos/Ever-Living.app /Applications/
open /Applications/Ever-Living.app
```

## 📁 项目结构

```
clacky-tray/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # 入口
│   │   ├── lib.rs           # Tauri Builder + 托盘 + 命令
│   │   ├── sidecar.rs       # Ruby 进程管理 (start/stop/kill)
│   │   └── preferences.rs   # 偏好设置 JSON 持久化
│   ├── capabilities/        # Tauri 权限配置
│   ├── icons/               # 应用图标全尺寸
│   └── tauri.conf.json      # Tauri 配置
├── index.html               # 状态面板入口
├── preferences.html         # 偏好设置窗口
└── package.json
```

## ⌨️ 快捷键

| 快捷键 | 功能 |
|--------|------|
| `⌘⇧O` | 全局唤起 / 隐藏主窗口 |
| `⌘N` | 新建会话 |
| `Enter` | 发送消息 |
| `⇧Enter` | 换行 |

## 🔧 配置

偏好设置文件：`~/Library/Application Support/com.weifengchen.ever-living/preferences.json`

```json
{
  "port": 7070,
  "autostart": false,
  "shortcut": "CmdOrCtrl+Shift+O",
  "notifications_enabled": true,
  "locale": "zh-CN"
}
```

## 📜 更新日志

### v0.1.0 — 初始发布

- ✅ macOS 菜单栏原生应用
- ✅ Ruby sidecar 进程管理（启动 / 停止 / 残留清理）
- ✅ 多会话并行 + 跨会话记忆
- ✅ 全局快捷键 + 开机自启
- ✅ 浅绿品牌配色 `#8FCA97`
- ✅ 偏好设置窗口（端口 / 语言 / 快捷键 / 通知）

## 🤝 贡献

欢迎提 Issue 和 PR。

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/amazing`
3. 提交更改：`git commit -m 'Add amazing'`
4. 推送分支：`git push origin feature/amazing`
5. 提交 Pull Request

## 📄 License

MIT License — 详见 [LICENSE](LICENSE)

---

<div align="center">

**[🌿 Live Green · Live Well · Live Forever]**

Made with 🟢 by [zolo1978](https://github.com/zolo1978)

</div>
