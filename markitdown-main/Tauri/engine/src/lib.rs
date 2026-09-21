//! mdcore：markitdown 的极简 Rust 引擎。
//! 架构抄原版作业：责任链（按猜测 × 按优先级挨个试）+ 全局归一化，
//! 但砍掉 StreamInfo/magika/插件层——一个静态 match 就够当前格式数用。

mod converters;
mod html;
mod sniff;

pub use sniff::Format;

use converters::{csv_to_md, docx_to_md, json_to_md};
use html::{html_to_md, normalize};
use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// 没有任何转换器认领这个文件
    Unsupported,
    /// 有转换器认领但转换失败（原因已说人话）
    Conversion(String),
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // 面向普通用户的文案，与前端错误层共用
            Error::Unsupported => write!(f, "这个文件暂时不支持，目前认识：Word、网页、CSV、JSON、纯文本"),
            Error::Conversion(s) => write!(f, "文件读取出错：{s}"),
            Error::Io(e) => write!(f, "文件打不开：{e}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// 转换本地文件，返回 Markdown。
pub fn convert_file(path: &str) -> Result<String, Error> {
    let bytes = std::fs::read(path)?;
    convert_bytes(path, &bytes)
}

/// 责任链主循环：后缀猜测、内容猜测去重后挨个试，全败才报错。
pub fn convert_bytes(path: &str, bytes: &[u8]) -> Result<String, Error> {
    let (by_ext, by_magic) = sniff::guesses(path, bytes);
    let mut tried: Vec<Format> = Vec::new();
    for guess in [by_ext, by_magic].into_iter().flatten() {
        if tried.contains(&guess) {
            continue;
        }
        tried.push(guess);
        match dispatch(guess, path, bytes) {
            Attempt::Ok(md) => return Ok(normalize(&md)),
            Attempt::Failed(why) => {
                // 认领了但转坏了：记录首个原因，继续让下一个猜测兜底
                if matches!(guess, Format::Docx | Format::Html | Format::Json | Format::Csv) {
                    return Err(Error::Conversion(why));
                }
            }
            Attempt::Skip => {}
        }
    }
    Err(Error::Unsupported)
}

enum Attempt {
    Ok(String),
    Failed(String),
    Skip,
}

fn dispatch(fmt_: Format, _path: &str, bytes: &[u8]) -> Attempt {
    let text = |lossy: bool| -> Result<String, Attempt> {
        match std::str::from_utf8(bytes) {
            Ok(s) => Ok(s.to_string()),
            Err(_) if lossy => Ok(String::from_utf8_lossy(bytes).into_owned()),
            Err(_) => Err(Attempt::Failed("文件编码不是 UTF-8".into())),
        }
    };
    match fmt_ {
        Format::Docx => match docx_to_md(bytes) {
            Ok(md) => Attempt::Ok(md),
            Err(e) => Attempt::Failed(e),
        },
        Format::Html => match text(true) {
            Ok(t) => Attempt::Ok(html_to_md(&t)),
            Err(e) => e,
        },
        Format::Json => match text(false) {
            Ok(t) => match json_to_md(&t) {
                Ok(md) => Attempt::Ok(md),
                Err(e) => Attempt::Failed(e),
            },
            Err(e) => e,
        },
        Format::Csv => match text(true) {
            Ok(t) => match csv_to_md(&t) {
                Ok(md) => Attempt::Ok(md),
                Err(e) => Attempt::Failed(e),
            },
            Err(e) => e,
        },
        Format::Plain => match text(true) {
            Ok(t) => Attempt::Ok(t),
            Err(e) => e,
        },
        Format::Unsupported(_) => Attempt::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, content: &[u8]) -> String {
        let p = format!("/tmp/mdcore_test_{name}");
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn plain_txt() {
        let p = tmp("a.txt", b"hello \nworld\n\n\n\nend\n");
        let md = convert_file(&p).unwrap();
        assert_eq!(md, "hello\nworld\n\nend"); // 归一化：去行尾空格+压空行
    }

    #[test]
    fn csv_becomes_table() {
        let p = tmp("a.csv", b"name,age\n\"a,b\",3\n");
        let md = convert_file(&p).unwrap();
        assert!(md.contains("| name | age |"), "{md:?}");
        assert!(md.contains("| a,b | 3 |"), "{md:?}");
    }

    #[test]
    fn json_array_becomes_table() {
        let p = tmp("a.json", br#"[{"n":"x","v":1},{"n":"y","v":2}]"#);
        let md = convert_file(&p).unwrap();
        assert!(md.contains("| n | v |"));
        assert!(md.contains("| x | 1 |"));
    }

    #[test]
    fn html_headings_and_list() {
        let p = tmp("a.html", b"<html><body><h2>T</h2><ul><li>i1</li><li>i2</li></ul></body></html>");
        let md = convert_file(&p).unwrap();
        assert!(md.contains("## T"));
        assert!(md.contains("- i1\n- i2"));
    }

    #[test]
    fn renamed_png_still_rejected_not_garbled() {
        // 内容嗅探认出 zip 家族之外的二进制 → 明确报不支持，而不是吐乱码
        let p = tmp("fake.docx", b"\x89PNG\r\n\x1a\x00rest-is-binary\x00\xff");
        // 后缀猜 docx：docx 解析失败应报错，而不是静默兜底成乱码
        let err = convert_file(&p).unwrap_err();
        assert!(matches!(err, Error::Conversion(_) | Error::Unsupported), "{err:?}");
    }

    #[test]
    fn docx_fixture() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/sample.docx");
        let md = convert_file(p).unwrap();
        assert!(md.contains("# 标题一"), "{md}");
        assert!(md.contains("正文段落内容。"), "{md}");
    }

    #[test]
    fn pdf_says_unsupported_in_chinese() {
        let p = tmp("a.pdf", b"%PDF-1.4\nnothing else");
        let err = convert_file(&p).unwrap_err();
        assert!(err.to_string().contains("不支持"), "{err}");
    }
}
