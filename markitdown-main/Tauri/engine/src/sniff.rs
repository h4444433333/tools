//! 简版格式嗅探：魔数 + 文本特征，替代 python 版的 magika ML 模型。
//! ponytail: 文本类判定是启发式的，误判时调度链会自然落到 Plain 兜底。

use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Docx,
    Html,
    Json,
    Csv,
    Plain,
    /// 能识别但引擎还没实现（pdf/zip/音频…）
    Unsupported(&'static str),
}

/// 返回 (按后缀猜, 按内容猜)。两者不一致时调用方都试一试。
pub fn guesses(path: &str, bytes: &[u8]) -> (Option<Format>, Option<Format>) {
    (from_ext(path), from_magic(bytes))
}

fn from_ext(path: &str) -> Option<Format> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "docx" => Format::Docx,
        "htm" | "html" => Format::Html,
        "json" => Format::Json,
        "csv" | "tsv" => Format::Csv,
        "txt" | "md" | "log" => Format::Plain,
        "pdf" => Format::Unsupported("pdf"),
        "zip" => Format::Unsupported("zip"),
        "xlsx" | "xls" | "pptx" | "epub" => Format::Unsupported("office-other"),
        "mp3" | "wav" => Format::Unsupported("audio"),
        _ => return None, // 没后缀/不认识，全交给内容猜
    })
}

fn from_magic(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"%PDF") {
        return Some(Format::Unsupported("pdf"));
    }
    if bytes.starts_with(b"PK\x03\x04") {
        // 在 zip 头部找条目名，认出 docx
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
        if head.contains("word/document") {
            return Some(Format::Docx);
        }
        return Some(Format::Unsupported("zip"));
    }
    if bytes.starts_with(b"\x1f\x8b") {
        return Some(Format::Unsupported("gz"));
    }
    // 文本类：BOM 说明链子接不住非法字节，先拒了交给 utf8_lossy 处理
    let text = match bytes.strip_prefix(&[0xef, 0xbb, 0xbf][..]) {
        Some(rest) => String::from_utf8_lossy(rest),
        None => match std::str::from_utf8(bytes) {
            Ok(s) => Cow::Borrowed(s),
            Err(_) => return Some(Format::Plain), // 非 utf8：留给转换器报错或兜底
        },
    };
    let t = text.trim_start();
    if t.starts_with('<') && (t[..t.len().min(512)].contains("<html") || t.starts_with("<!doctype html") || t.starts_with("<h1") || t.starts_with("<p>") || t.starts_with("<table") || t.starts_with("<div") || t.starts_with("<!DOCTYPE")) {
        return Some(Format::Html);
    }
    if t.starts_with('<') {
        return Some(Format::Plain); // 是 XML 但我们不当 html 硬转
    }
    if t.starts_with('[') && looks_like_json_object_array(t) {
        return Some(Format::Json);
    }
    if t.starts_with('{') || t.starts_with('[') {
        return Some(Format::Json);
    }
    let first_line = t.lines().next().unwrap_or("");
    if t.contains('\n') && first_line.contains(',') && !first_line.trim_end().ends_with(',')
    {
        return Some(Format::Csv);
    }
    Some(Format::Plain)
}

/// 数组第一个元素是不是对象（决定 json 转表格还是转列表）
pub fn looks_like_json_object_array(t: &str) -> bool {
    matches!(t.as_bytes().get(1), Some(b'{') | Some(b'\n') | Some(b' '))
}
