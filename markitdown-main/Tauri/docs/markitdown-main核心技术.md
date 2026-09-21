# markitdown 核心技术详解

> 配套文档：《markitdown-main项目讲解.md》。本篇只讲技术实现，每条都对应到具体源码位置。

---

## 1. 注册式责任链 + 优先级调度（全局骨架）

**代码位置**：`packages/markitdown/src/markitdown/_markitdown.py`

- 核心数据结构只有一张表：`self._converters: List[ConverterRegistration]`，每项 = `(转换器实例, 优先级)`。
- 注册即头插：`register_converter()` 用 `list.insert(0, ...)`，**后注册的排最前**。
- 每次转换前重新稳定排序：`sorted(self._converters, key=lambda x: x.priority)`（排序放在每次调用时做，因为优先级可能在两次调用之间变化）。稳定排序保证同优先级时"后注册者先试"。
- 优先级只有两档常量：
  - `PRIORITY_SPECIFIC_FILE_FORMAT = 0.0` —— 专格式转换器（PDF/DOCX/XLSX…）
  - `PRIORITY_GENERIC_FILE_FORMAT = 10.0` —— 兜底转换器（PlainText/Html/Zip），注册时显式传 10，垫底。
- **覆盖机制**：注册表头插 + 稳定排序，天然实现"后来者居上"。插件或 Azure 云端转换器只要最后注册（优先级 0），就排在所有本地同名转换器前面，先被尝试；云端失败抛异常后，调度循环自动回落到本地转换器 —— 这就是"云优先、本地兜底"的完整实现，没有任何 if/else 硬编码。

## 2. 转换器双方法契约：accepts() / convert()

**代码位置**：`_base_converter.py`

- 所有转换器继承 `DocumentConverter`，只要求实现两个方法，且**方法签名刻意完全一致**（同样的 `file_stream, stream_info, **kwargs`）——保证"只要能 accepts 就一定能 convert"。
- 唯一硬性红线：**accepts() 不许改变二进制流的读取位置**。如果为了判断必须多读几个字节（如 Outlook .msg 要 peek 魔数），必须自己 `seek(cur_pos)` 复位。
- `DocumentConverterResult` 极简：`markdown` 字符串 + 可选 `title`，`__str__` 直接返回 markdown。转换器之间零耦合。

## 3. StreamInfo 多猜测机制（格式识别的鲁棒性来源）

**代码位置**：`_markitdown.py` 的 `_get_stream_info_guesses()`（L741-840）

`StreamInfo`（`_stream_info.py`，frozen dataclass）是文件的"身份证"：`mimetype / extension / charset / filename / local_path / url`，支持 `copy_and_update()` 不可变合并。

识别管线（多信号融合）：

```
基础猜测（路径/响应头给什么用什么）
   │  只有扩展名没 mimetype → mimetypes.guess_type() 补
   │  只有 mimetype 没扩展名 → mimetypes.guess_all_extensions() 补
   ▼
magika 内容识别（ML 模型，读文件头字节判真实类型）
   │  是文本 → charset-normalizer 采样探测编码
   ▼
相容性判定：magika 结果 vs 基础猜测 的 mimetype/extension/charset 逐项比对
   ├─ 相容   → 只放 1 个合并后的猜测
   └─ 不相容 → 放 2 个猜测（后缀说的 + 内容说的），调度循环都试
```

- 编码采样有个细节（`_read_charset_sample()` L73-98）：读 64KiB 样本时，若末尾正好切断了一个多字节 UTF-8 字符，用增量解码器**补读最多 3 字节**把字符拼完整，避免误判编码。
- 每个猜测都用 `finally: file_stream.seek(cur_pos)` 保证探测完复位。
- 调度层对 `stream_info_guesses + [StreamInfo()]`（全空的终极猜测）做双层循环 —— 即使文件既没后缀又识别不出，仍给通用转换器最后一次机会。

## 4. 调度循环的防御性设计

**代码位置**：`_convert()`（L606-699）

- 进入前记录 `cur_pos = file_stream.tell()`，两处**断言**流位置未变：猜测迭代之间、每次 accepts() 之后。违规转换器（尤其第三方插件）会被断言当场抓住，而不是造成难以排查的静默错位。
- 失败隔离：`convert()` 抛任何异常 → 打包成 `FailedConversionAttempt(converter, exc_info)` 继续下一个转换器；成功前绝不返回半成品。
- 异常分层（`_exceptions.py`）：`StepInternalException` → `FileConversionException`（有转换器认领但全部失败，`__str__` 汇总每个尝试的堆栈）→ `UnsupportedFormatException`（无人认领）。另有 `MissingDependencyException`（可选依赖没装，直接提示装哪个 extra）。
- 成功后的唯一全局后处理：逐行 `rstrip()` + 把 ≥3 个连续换行压成 2 个（`re.sub(r"\n{3,}", "\n\n", ...)`），归一化 whitespace 由调度层统一做，转换器不用管。

## 5. 全局参数透传与嵌套转换

**代码位置**：`_convert()` L630-649

- 构造函数上的 `llm_client / llm_model / llm_prompt / exiftool_path / style_map` 存为实例字段，每次调用前注入 kwargs（调用方显式传的优先）。转换器按需取用 —— 不传 LLM 的调用完全无感，没有任何强制网络开销。
- `_kwargs["_parent_converters"] = self._converters`：把整张注册表塞给转换器，**ZIP 转换器据此实现递归** —— 解开包后对每个成员文件重新走一遍完整的 guess×accept×convert 流程（`convert_stream` 语义），因此嵌套 zip、zip 内混合格式天然支持。`OutlookMsgConverter` 同样用它转换邮件附件。

## 6. 插件体系（不改源码的扩展点）

**代码位置**：`_load_plugins()`（L113-130）、`enable_plugins()`

- 基于 Python entry_points，分组名固定 `markitdown.plugin`；插件只需实现 `register_converters(markitdown_instance, **kwargs)`。
- **懒加载 + 只加载一次**（模块级 `_plugins` 三态：None=未加载）；单个插件加载/注册失败只 `warn` 并跳过，**绝不让一个坏插件拖垮整个转换**。
- 插件可自选优先级插在任意位置（如 9 = 排在兜底组之前、专格式组之后），`markitdown --use-plugins` 才启用，默认关闭。

## 7. URL 形状路由（内容级转换器）

**代码位置**：`converters/_wikipedia_converter.py`、`_youtube_converter.py`、`_bing_serp_converter.py`、`_rss_converter.py`

- 这类转换器的 `accepts()` 匹配的是 **URL 模式**（如 `en.wikipedia.org/wiki/`），而不是文件类型。同一个 `https://` 输入，维基链接走"只提正文"的专用解析，普通网页落到优先级 10 的 HtmlConverter。
- 实现"专站专办"但零中心路由代码 —— 全部靠责任链自然分流。
- `convert_uri()` 支持 http/https/file/data 四种 scheme；`file:` 经 `file_uri_to_path()`（`_uri_utils.py`）安全转本地路径，`data:` 经 `parse_data_uri()` 内联解码。

## 8. HTTP 响应头的完整利用

**代码位置**：`convert_response()`（L540-604）

从响应头榨取一切识别线索：`content-type`（拆 mimetype + charset）、`content-disposition`（RFC 2231 折叠编码的文件名解析，`_get_content_disposition_filename()`）、URL path 兜底提取文件名/扩展名。响应体分块读入 `BytesIO` 后再进统一管线 —— 网络层和转换层彻底解耦。

## 9. 安全模型：宽入口 + 窄入口分层

- `convert()` 是"什么都接"的便利入口（路径/URL/响应/流自动分发）；官方安全建议是按需只用 `convert_local()` / `convert_stream()` / `convert_response()`，在源头收敛权限面。
- 进程权限即 I/O 权限（README 明示）：不内置沙箱，靠调用方选对入口 + 清洗输入。
- HTTP 请求带 `Accept: text/markdown, text/html;q=0.9...`（L160-164）—— 支持"Markdown for agents"的服务端直接回 Markdown，省一次转换。
- 流不可 seek 时自动整体缓冲进内存（`convert_stream()` L436-444），保证任何转换器都能复位 —— 契约由框架兜底而非转嫁插件作者。

## 10. 重量级能力的可选化模式（改造 Rust 时的依赖映射参考）

| 能力 | 实现方式 | 依赖强度 |
|---|---|---|
| PDF 文本/表格 | pdfplumber/pdfminer 逐页抽取，表格转 md 表 | extra: `[pdf]` |
| Office 三件套 | zip 解包 + XML 解析（python-docx/pptx/openpyxl） | 各自独立 extra |
| 图片描述/OCR | 调外部多模态 LLM（llm_client），本地零 ML 依赖 | 运行时可选 |
| 音频转写 | pydub 解码 + SpeechRecognition 上云识别 | extra: `[audio-transcription]` |
| 文件类型识别 | magika（ONNX 小模型，CPU 推理） | **核心强制依赖** |
| 扫描件高保真 | Azure DocIntel / Content Understanding，云端注册即覆盖本地 | 纯 API 客户端 |

→ Rust 改写映射：zip+xml 系（docx/pptx/xlsx/epub/ipynb）最易（`zip` + `quick-xml`）；PDF 用 `pdfium`/`lopdf`；magika 模型可经 `ort`（ONNX Runtime）原样跑；markdownify 逻辑需手写（`html2text` 类 crate）；LLM 类能力天然是 HTTP 调用，改造零障碍。

## 11. MCP 子包：最小封装的范本

**代码位置**：`packages/markitdown-mcp/src/markitdown_mcp/__main__.py`（仅 139 行）

- 一个 `@mcp.tool() convert_to_markdown(uri)` 即全部核心；`MCPServer` 同时支持 stdio 与 Streamable HTTP（`uvicorn` + Starlette 挂载）。
- 错误处理哲学值得抄：每种异常折叠成**固定文案的 `ToolError`**（如 OSError 只回 `os.strerror`，**不回显已解析的路径**）—— 工具调用方不需要、也不应看到内部细节。Tauri 版的前端错误文案层应沿用同一原则。
