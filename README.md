<div align="center">

#  Tools

**A collection of practical, phone-first utilities — zero tech knowledge required.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-engine-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://tauri.app/)

[English](#english) · [中文](#中文)

</div>

---

<a name="english"></a>

## English

### What is this?

A monorepo of **handy tools rebuilt for mobile phones**. Every tool here was originally a desktop/CLI utility (mostly Python), rewritten in **Rust** and packaged as a **cross-platform app** (Android / iOS / Mac / Windows) via [Tauri v2](https://tauri.app). No terminal. No pip install. Just tap and go.

### Design Principles

| Principle | Meaning |
|-----------|---------|
| **Phone-first** | If it can't run on a phone, it doesn't ship. |
| **Zero-config** | No API keys, no env vars, no setup. Open and use. |
| **Offline** | Core features work without internet. |
| **Self-contained** | All interaction inside our own app — no sharing to other apps required. |
| **Beautiful** | Intuitive UI tailored to each tool's purpose. |

### Tools

| Tool | Description | Platform | Status |
|------|-------------|----------|--------|
| [**Textify**](#textify) | Convert any document (Word, HTML, CSV, JSON…) to clean Markdown text — right on your phone | Android · iOS · Mac | 🟢 Alpha |

---

<a name="textify"></a>

### Textify

> Turn files into text you can actually use — copy to AI, paste anywhere, export as Markdown.

**Based on:** [Microsoft markitdown](https://github.com/microsoft/markitdown) (MIT License) — an open-source tool that converts Office documents, PDFs, images, and more to Markdown. We rewrote its core in Rust and wrapped it as a phone-first mobile app.

**Downloads:**

| Platform | File | Size |
|----------|------|------|
|  Android (APK) | [textify-android-debug.apk](https://github.com/h4444433333/tools/releases/latest/download/textify-android-debug.apk) | 443 MB |
| 💻 Mac (Apple Silicon) | [textify-mac-universal.zip](https://github.com/h4444433333/tools/releases/latest/download/textify-mac-universal.zip) | 6.6 MB |
| 🍎 iPhone (iOS) | Coming soon — requires Apple Developer signing | — |

> Android: enable "Install from unknown sources" in Settings, then tap the APK.
> Mac: unzip, drag to Applications. If Gatekeeper blocks it, right-click → Open.

**Features:**
- 📂 Built-in file browser — navigate your phone's storage, no external app needed
- 🔄 One-tap conversion: `.docx` / `.html` / `.csv` / `.json` / `.txt` → Markdown
- 💾 Auto-saves `.md` next to the original file
- 📋 Copy to clipboard (paste directly into ChatGPT, Claude, etc.)
- 📷 Coming soon: photo OCR, PDF, URL fetch, audio transcription

**Tech stack:**
- **Engine**: [mdcore](markitdown-main/Tauri/engine/) — pure Rust, zero Python, compiles to native ARM
- **App shell**: [Tauri v2](markitdown-main/Tauri/app/) — one codebase → Android APK + iOS IPA + Mac .app
- **Based on**: [Microsoft markitdown](markitdown-main/) (original Python project, kept as reference)

**快速上手：**
```bash
# 克隆整个仓库
git clone https://github.com/h4444433333/tools.git && cd tools

# 只拉取某一个工具（稀疏检出，省流量）
git clone --filter=blob:none --sparse https://github.com/h4444433333/tools.git
cd tools
git sparse-checkout set markitdown-main/Tauri   # 只要这一个工具的代码

# 编译引擎
cd markitdown-main/Tauri/engine && cargo build --release

# 跑 Mac 版
cd ../app/src-tauri && cargo build && open target/debug/mdapp

# 出安卓 APK
cd ../.. && npx tauri android build --apk

# 出 iPhone 版（需苹果开发者账号）
cd ../.. && npx tauri ios build
```

---

<a name="中文"></a>

## 中文

### 这是什么？

一个**手机优先的实用工具箱**。每个工具原本都是桌面/命令行的（多数是 Python），我们用 **Rust** 重写核心引擎，用 **Tauri v2** 打包成跨平台 App（安卓 / iPhone / Mac / Windows）。不用命令行、不用装 Python，打开就能用。

### 设计理念

| 原则 | 含义 |
|------|------|
| **手机优先** | 不能在手机上跑的功能不发版 |
| **零配置** | 不填密钥、不设环境变量，打开即用 |
| **离线可用** | 核心功能不联网也能跑 |
| **自闭环** | 所有操作在自己 App 内完成，不依赖其他 App |
| **好看好用** | 根据工具特点设计直观界面 |

### 工具列表

| 工具 | 简介 | 平台 | 状态 |
|------|------|------|------|
| [**万物转文本 (Textify)**](#textify-1) | 手机上把 Word、网页、表格等文件一键转成干净的 Markdown 文本 | 安卓 · iPhone · Mac | 🟢 Alpha |

---

<a name="textify-1"></a>

### 万物转文本 (Textify)

> 把任何文件变成你能用的文字——复制给 AI、随处粘贴、导出 Markdown。

**基于：** [Microsoft markitdown](https://github.com/microsoft/markitdown)（MIT 开源协议）——微软出品的文档转 Markdown 工具。我们用 Rust 重写了它的核心引擎，并包装成手机优先的移动端 App。

**下载安装：**

| 平台 | 文件 | 大小 |
|------|------|------|
| 📱 安卓 (APK) | [textify-android-debug.apk](https://github.com/h4444433333/tools/releases/latest/download/textify-android-debug.apk) | 443 MB |
| 💻 Mac (苹果芯片) | [textify-mac-universal.zip](https://github.com/h4444433333/tools/releases/latest/download/textify-mac-universal.zip) | 6.6 MB |
| 🍎 iPhone (iOS) | 待上架 —— 需要苹果开发者签名 | — |

> 安卓：设置里开启“允许安装未知来源应用”，然后点 APK 安装。
> Mac：解压后拖到“应用程序”文件夹。如果提示无法打开，右键→打开。

**功能：**
- 📂 App 内自带文件浏览器——直接翻手机存储，不用跳到别的 App
- 🔄 一键转换：`.docx` / `.html` / `.csv` / `.json` / `.txt` → Markdown
- 💾 自动存 `.md` 到源文件旁边，不用找输出路径
- 📋 复制到剪贴板（打开 ChatGPT / 通义千问 直接粘贴）
- 📷 即将支持：拍照识别、PDF、网址抓取、语音转文字

**技术栈：**
- **引擎**：[mdcore](markitdown-main/Tauri/engine/) —— 纯 Rust 编写，零 Python 依赖，编译为原生 ARM 指令
- **App 壳**：[Tauri v2](markitdown-main/Tauri/app/) —— 一套代码出安卓 APK + iPhone IPA + Mac .app
- **参考原项目**：[Microsoft markitdown](markitdown-main/)（Python 版，保留作为架构参考）

**快速上手：**
```bash
# 克隆整个仓库
git clone https://github.com/h4444433333/tools.git && cd tools

# 只拉取某一个工具（稀疏检出，省流量）
git clone --filter=blob:none --sparse https://github.com/h4444433333/tools.git
cd tools
git sparse-checkout set markitdown-main/Tauri   # 只要这一个工具的代码

# 编译引擎
cd markitdown-main/Tauri/engine && cargo build --release

# 跑 Mac 版
cd ../app/src-tauri && cargo build && open target/debug/mdapp

# 出安卓 APK
cd ../.. && npx tauri android build --apk

# 出 iPhone 版（需苹果开发者账号）
cd ../.. && npx tauri ios build
```

---

## 📁 项目结构

```
tools/
├── Agents.md                  # 工程规范（所有工具共用的军规）
├── README.md                  # 本文件
└── markitdown-main/           # 工具 ①：万物转文本
    ├── Tauri/                 # ← 改造代码全在这里
    │   ├── engine/            #    Rust 转换引擎 (mdcore crate)
    │   ├── app/               #    Tauri v2 App 壳 (前端 + 后端)
    │   ├── docs/              #    设计文档
    │   └── specs/             #    SDD 规格
    ├── packages/              #    原始 Python 代码（参考用）
    └── ...
```

## 🤝 Contributing / 贡献

PRs welcome! Each tool lives in its own folder under the repo root. Follow [Agents.md](Agents.md) for conventions.

欢迎贡献！每个工具放在独立文件夹，改造代码统一在工具目录下的 `Tauri/` 子文件夹。详见 [Agents.md](Agents.md)。

## 📄 License

[MIT](LICENSE) © [h4444433333](https://github.com/h4444433333)
