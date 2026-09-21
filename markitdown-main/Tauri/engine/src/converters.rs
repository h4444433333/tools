//! CSV / JSON / DOCX 转换器。

/// CSV → Markdown 表格。首行当表头。
pub fn csv_to_md(text: &str) -> Result<String, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false) // records() 要拿到全部行，首行自己当表头
        .flexible(true) // 烂数据不炸，缺列补空
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut it = rdr.records();
    let header = it
        .next()
        .ok_or_else(|| "csv 没有数据行".to_string())?
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    let cells: Vec<String> = header.iter().map(|c| esc(c)).collect();
    out.push_str("| ");
    out.push_str(&cells.join(" | "));
    out.push_str(" |\n|");
    for _ in 0..cells.len() {
        out.push_str(" --- |");
    }
    out.push('\n');
    for rec in it {
        let rec = rec.map_err(|e| e.to_string())?;
        out.push_str("| ");
        let cells: Vec<String> = rec.iter().map(|c| esc(c)).collect();
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
    }
    Ok(out)
}

fn esc(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// JSON → Markdown：对象数组转表格，其余转缩进列表。
pub fn json_to_md(text: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(text.trim()).map_err(|e| e.to_string())?;
    let mut out = String::new();
    render_json(&v, 0, &mut out);
    Ok(out)
}

fn render_json(v: &serde_json::Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map {
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        out.push_str(&format!("{pad}- **{k}**\n"));
                        render_json(val, depth + 1, out);
                    }
                    _ => {
                        out.push_str(&format!("{pad}- **{k}**: {}\n", fmt_scalar(val)))
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            // 全是对称对象 → 表格（python 版对 jsonl 也这么干）
            if !arr.is_empty()
                && arr.iter().all(|x| x.is_object())
            {
                let cols: Vec<String> = arr[0]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect();
                out.push_str("| ");
                out.push_str(&cols.iter().map(|c| esc(c)).collect::<Vec<_>>().join(" | "));
                out.push_str(" |\n|");
                for _ in &cols {
                    out.push_str(" --- |");
                }
                out.push('\n');
                for row in arr {
                    let cells: Vec<String> = cols
                        .iter()
                        .map(|c| esc(&fmt_scalar(row.get(c).unwrap_or(&serde_json::Value::Null))))
                        .collect();
                    out.push_str("| ");
                    out.push_str(&cells.join(" | "));
                    out.push_str(" |\n");
                }
            } else {
                for item in arr {
                    match item {
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                            out.push_str(&format!("{pad}-\n"));
                            render_json(item, depth + 1, out);
                        }
                        _ => out.push_str(&format!("{pad}- {}\n", fmt_scalar(item))),
                    }
                }
            }
        }
        _ => out.push_str(&format!("{pad}{}\n", fmt_scalar(v))),
    }
}

fn fmt_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// DOCX → Markdown：解 zip 拿 word/document.xml，段落按样式转标题/正文。
/// ponytail: 只提 w:t 文本 + Heading 样式；表格/图片/批注不做，等接需求再加。
pub fn docx_to_md(bytes: &[u8]) -> Result<String, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    let mut xml = String::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.name() == "word/document.xml" {
            std::io::Read::read_to_string(&mut f, &mut xml).map_err(|e| e.to_string())?;
            break;
        }
    }
    if xml.is_empty() {
        return Err("不是有效的 docx（缺 word/document.xml）".into());
    }
    let mut out = String::new();
    // 按段落切：每个 </w:p> 结尾是一段
    for para in xml.split("</w:p>") {
        let style = para
            .find("w:pStyle w:val=\"Heading")
            .map(|_| ()); // 有 Heading 样式
        let level = para
            .split_once("w:pStyle w:val=\"Heading")
            .and_then(|(_, r)| r.chars().next())
            .and_then(|c| c.to_digit(10))
            .unwrap_or(0);
        let mut text = String::new();
        let mut rest = para;
        while let Some(a) = rest.find("<w:t") {
            let Some(b) = rest[a..].find('>') else { break };
            let start = a + b + 1;
            let end = rest[start..].find("</w:t>").map(|e| start + e).unwrap_or(rest.len());
            text.push_str(&rest[start..end]);
            rest = &rest[end..];
        }
        let text = decode_xml(&text);
        if text.trim().is_empty() {
            continue;
        }
        if level > 0 {
            out.push_str(&"#".repeat(level as usize));
            out.push(' ');
        }
        out.push_str(text.trim());
        out.push('\n');
        if style.is_some() || level == 0 {
            out.push('\n');
        }
    }
    Ok(out)
}

fn decode_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
