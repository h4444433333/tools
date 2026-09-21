# markitdown 项目总体讲解

> 一句话概括：**这是一个"万物转 Markdown"的转换工具** —— 你把 PDF、Word、Excel、PPT、图片、音频、网页、EPUB、ZIP 甚至 YouTube 链接丢给它，它输出干净的 Markdown 文本。它是微软开源的项目，主要服务于大模型（LLM）和文本分析场景。

---

## 一、这个工具是干什么用的？

### 白话场景
你手上有一份 PDF 合同、一个 Word 报告、一张截图、一段录音，想把这些内容变成纯文本喂给 AI（比如让 GPT 帮你总结）。以前每个格式都要找一个专门的解析库，写法各不相同。markitdown 把这件事统一了：**一个入口，什么格式进，Markdown 出**。

### 为什么输出是 Markdown？
- 大模型天然"读懂" Markdown（训练语料里大量 Markdown），标题、列表、表格、链接都能保留结构；
- Markdown 接近纯文本，省 token（省钱）；
- 比 HTML 干净，比纯文本多了结构信息。

### 支持转换的格式（内置）
PDF、PowerPoint、Word、Excel（含老版 .xls）、图片（EXIF 信息 + OCR）、音频（语音转文字）、HTML、CSV/JSON/XML 等文本格式、ZIP（递归拆开逐个转）、YouTube 链接（抓字幕）、EPUB 电子书、Outlook 邮件、Jupyter Notebook、Wikipedia/RSS/Bing 搜索结果页等特定网址。

> 注意：它的输出是"给机器读的"，结构保留得不错，但不是为"给人看的完美排版"设计的。

---

## 二、工程结构（Monorepo，4 个子包）

```
markitdown-main/
└── packages/
    ├── markitdown/                  # ★ 核心库（绝大部分代码在这里）
    │   ├── src/markitdown/
    │   │   ├── _markitdown.py       # 主类 MarkItDown：调度中枢（852 行）
    │   │   ├── _base_converter.py   # 所有转换器的抽象基类 + 结果类
    │   │   ├── _stream_info.py      # 文件"身份证"：mimetype/扩展名/编码/URL 等元信息
    │   │   ├── _exceptions.py       # 统一异常体系
    │   │   ├── __main__.py          # 命令行入口（markitdown xxx.pdf -o out.md）
    │   │   ├── converters/          # ★ 20+ 个具体格式转换器，一个文件一种格式
    │   │   └── converter_utils/     # 图片、Excel 内嵌图片、docx 等公共工具函数
    │   └── tests/                   # 大量按格式分类的测试（docx 公式、pptx 图片、pdf 表格…）
    ├── markitdown-mcp/              # MCP 服务器：把转换能力暴露给 AI Agent 调用
    ├── markitdown-ocr/              # 插件：用大模型视觉能力给 PDF/Word/PPT/Excel 里的图片做 OCR
    └── markitdown-sample-plugin/    # 官方插件示例（教第三方怎么写插件）
```

### 关键技术栈（Python ≥ 3.10）
| 用途 | 依赖库 |
|---|---|
| 文件类型识别（看内容猜格式） | **magika**（Google 的 ML 文件分类模型） |
| 字符编码探测 | charset-normalizer |
| HTML → Markdown | beautifulsoup4 + markdownify |
| PDF 解析 | pdfplumber / pdfminer.six |
| Office 文档 | python-docx、python-pptx、openpyxl、xlrd、pandas |
| Outlook 邮件 | olefile |
| 音频转写 | pydub + SpeechRecognition |
| YouTube 字幕 | youtube-transcript-api |
| 云端高质量解析（可选） | Azure Document Intelligence / Azure Content Understanding |
| 网络请求 | requests |

依赖是**按需安装**的（`pip install 'markitdown[pdf,docx]'` 只装用到的），核心只强制要求少量基础库 —— 这让轻量部署成为可能，也是我们后续改造时值得借鉴的设计。

---

## 三、核心技术架构：三层"责任链"设计

整个项目最核心的设计模式是 **注册式责任链（Chain of Responsibility）+ 优先级调度**：

```
                     ┌──────────────────────────────┐
 用户输入             │        MarkItDown 主类        │
 (文件/URL/流)  ───▶ │  1. 收集 StreamInfo "猜测"     │
                     │  2. 按优先级遍历转换器          │
                     │  3. accepts() → convert()     │
                     └──────────────┬───────────────┘
                                    │ 逐个询问
        ┌───────────┬───────────┬───┴─────┬───────────┐
        ▼           ▼           ▼         ▼           ▼
   PdfConverter  DocxConverter  ...   HtmlConverter  PlainTextConverter
   (优先级 0，    (优先级 0)          (优先级 10，兜底)  (优先级 10，兜底)
    最具体的排前面)
```

三个关键机制：

1. **统一转换器接口**（`_base_converter.py`）：每个转换器只需实现两个方法：
   - `accepts(file_stream, stream_info)` —— "这文件我能处理吗？"（快速判断，不许改变流位置）
   - `convert(...)` —— 真正干活，返回 `DocumentConverterResult`（Markdown 文本 + 可选标题）。

2. **优先级注册表**（`register_converter`）：
   - 专格式转换器（PDF/DOCX 等）默认优先级 0，**先被尝试**；
   - 通用转换器（纯文本/HTML/ZIP）优先级 10，**垫底兜底**；
   - 同优先级下，**后注册的排前面**（稳定排序），所以 Azure 云端转换器如果在注册表最顶部注册，就会覆盖本地同名转换器 —— 这就是"挂了云服务就用云、没挂就用本地"的实现方式。

3. **插件体系**：第三方包通过 Python entry_points（分组 `markitdown.plugin`）注册，实现 `register_converters()` 往注册表里插自己的转换器。**不用改源码就能扩展新格式**（如 markitdown-ocr 就是这样给 4 种文档转换器"打补丁"加 OCR 的）。

---

## 四、一次转换的完整流程（最重要）

以 `md.convert("report.pdf")` 为例，逐步走一遍代码：

```
① 入口分发 convert()
   判断 source 类型：本地路径 / http(s) URL / file: URI / data: URI /
   requests.Response / 二进制流 → 分别路由到 convert_local / convert_uri /
   convert_response / convert_stream
   （权限最小化设计：官方建议只用你需要的窄入口）

② 组建 StreamInfo "猜测列表"  _get_stream_info_guesses()
   - 从路径拿扩展名、文件名 → 生成基础猜测
   - 用 mimetypes 在"扩展名 ↔ mimetype"间互相补全
   - 读文件头，让 magika 从二进制内容识别真实类型（防改名骗人）
   - 是文本的话，再用 charset-normalizer 采样 64KB 探测字符编码
   - 若 magika 结果与文件后缀矛盾 → 两套猜测都放进列表，挨个试

③ 调度循环 _convert()
   对每一个 stream_info 猜测 × 按优先级排序的每个转换器：
   a. 断言流位置没被上一个转换器弄乱（核心防御性约定）
   b. converter.accepts() 说"能" → 调 converter.convert()
   c. 转换抛异常 → 记入 failed_attempts，继续问下一个转换器
      （一个格式多个转换器可互相兜底，比如 docx 解析失败还能试试通用转换器）
   d. 成功 → 后处理：去行尾空格、把 3 个以上连续换行压成 2 个 → 返回

④ 全部失败
   有尝试记录 → 抛 FileConversionException（带每个转换器的原始堆栈）
   无人认领   → 抛 UnsupportedFormatException（"这格式不支持"）
```

**嵌套转换**：ZIP 转换器拿到压缩包后，会把里面每个文件**递归交回主调度**（通过 `_parent_converters` 传递注册表），所以一个 zip 里混着 pdf+docx+html 也能全部转出来拼成一份 Markdown。这是架构上很漂亮的自相似设计。

**特殊转换器直接接管 URL**：Wikipedia / YouTube / Bing SERP 转换器的 `accepts()` 是按 **URL 形状**匹配的 —— 同一个"网页"输入，维基链接走维基专用解析（只提正文），普通网页走 HTML 通用解析。

---

## 五、周边能力一览

| 能力 | 入口 | 说明 |
|---|---|---|
| 命令行 | `markitdown file.pdf -o out.md`，支持管道 | 286 行，薄封装 |
| Python API | `MarkItDown().convert(...)` | 主用法 |
| Docker | 根目录 `Dockerfile` | 管道式一次性转换 |
| MCP 服务器 | `packages/markitdown-mcp`（139 行） | 暴露 `convert_to_markdown(uri)` 工具，让 Claude 等 AI Agent 直接调用；支持 stdio 和 Streamable HTTP 两种传输 |
| LLM 图片描述 | `llm_client` / `llm_model` 参数 | PPT/图片里的图像可发给多模态大模型生成描述文字 |
| Azure 云端解析 | `docintel_endpoint` / `cu_endpoint` | 扫描件、复杂表格、音视频用云端服务，质量更高；CU 还支持自定义分析器 + YAML front matter 字段抽取 |
| OCR 插件 | `markitdown-ocr` | 复用 llm_client 模式，不引入新 ML 依赖 |

**安全设计**（README 专门强调）：它以进程权限做 I/O，会访问进程能访问的一切资源 —— 官方明确要求：不信任的输入必须先清洗，服务端场景要用最窄的 `convert_local()` / `convert_stream()` 等入口，限制路径、URL scheme 和内网地址。

---

## 六、优点与局限（为改造做铺垫）

**优点**
- 架构极简清晰：一个基类 + 一张优先级注册表，扩展新格式成本极低；
- 容错性好：多猜测 × 多转换器互相兜底，改名文件、后缀骗人都能处理；
- 依赖按需安装，核心很轻；
- 测试覆盖细致（按格式、按特性拆分的 40+ 测试文件）。

**局限 / 我们的改造切入点**
- **纯 Python 实现**，手机上跑不了 → 按 Agents.md 规则，核心路径需改写为 Rust（PDF 有 pdfium/lopdf 生态，Office 文档本质是 zip+xml 可用 zip+quick-xml 解析，magika 是 ONNX 模型可用 candle/ort 跑，均无需回退 C++；个别卡住的依赖再按规则降级 C++）；
- 无图形界面（上游明确拒绝做 UI，官方定位就是库）→ 界面正是我们 Tauri 封装要补足的核心；
- 网络类转换器（YouTube/Wikipedia/Azure）依赖外网与云账号 → 手机独立使用场景下应作为可选能力；
- 输出面向 LLM 而非普通人阅读 → 界面上要做"预览/复制/导出"这类傻瓜化包装。

---

## 七、README 中明确的项目边界（重要）

上游微软声明：本仓库**只收库和 CLI**，明确拒绝 Web 前端、桌面端、移动端应用（Electron/PyQt/Flutter 等"out of scope"），希望衍生应用作为独立项目依赖 PyPI 包。—— 即：**我们做 Tauri 手机壳完全符合上游意愿，且官方插件机制（#markitdown-plugin）就是给这种扩展留的口子。**
