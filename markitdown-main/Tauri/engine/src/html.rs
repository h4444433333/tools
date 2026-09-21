//! HTML → Markdown。ponytail: 不上解析器 crate，用状态机做「常见子集」：
//! 标题/段落/列表/链接/加粗斜体/表格/代码块；复杂嵌套和表单直接丢标签保文本。
//! 上限：不处理 <script> 里的伪标签、不处理块内行内混排的极端情况。够用再换 scraper。

pub fn html_to_md(html: &str) -> String {
    let mut out = String::new();
    let mut in_script = false;
    let mut in_pre = false;
    let mut list_stack: Vec<char> = Vec::new(); // 'u' | 'o'
    let mut in_td = false;
    let mut last_was_li = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut cur_row: Vec<String> = Vec::new();
    let mut cell = String::new();

    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // 读到标签结束
            let Some(end) = find_byte(bytes, i, b'>') else { break };
            let tag = String::from_utf8_lossy(&html.as_bytes()[i + 1..end]).to_lowercase();
            let closing = tag.starts_with('/');
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            let attrs = &tag; // 简化：整串里找 href
            if name != "li" {
                last_was_li = false;
            }

            match name.as_str() {
                "script" | "style" => in_script = !in_script && closing == false && in_script == false || (in_script && closing),
                "pre" => {
                    in_pre = !closing;
                    fenced(&mut out, in_pre, "```");
                }
                "code" if !in_pre => fenced(&mut out, !closing, "`"),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    let level = name[1..2].parse::<usize>().unwrap_or(1);
                    if closing {
                        out.push('\n');
                    } else {
                        newline_blank(&mut out);
                        out.push_str(&"#".repeat(level));
                        out.push(' ');
                    }
                }
                "p" | "div" | "section" | "article" | "header" | "footer" => {
                    if closing {
                        newline_blank(&mut out);
                    }
                }
                "br" => out.push('\n'),
                "hr" => {
                    newline_blank(&mut out);
                    out.push_str("---\n");
                }
                "ul" => {
                    if closing {
                        list_stack.pop();
                    } else {
                        list_stack.push('u');
                    }
                }
                "ol" => {
                    if closing {
                        list_stack.pop();
                    } else {
                        list_stack.push('o');
                    }
                }
                "li" => {
                    if closing {
                        out.push('\n');
                    } else {
                        // 列表项之间单行分隔；整表前才留空行
                        if last_was_li {
                            if !out.ends_with('\n') {
                                out.push('\n');
                            }
                        } else {
                            newline_blank(&mut out);
                        }
                        let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                        let marker = match list_stack.last() {
                            Some('o') => "1. ",
                            _ => "- ",
                        };
                        out.push_str(&indent);
                        out.push_str(marker);
                        last_was_li = true;
                    }
                }
                "a" => {
                    if closing {
                        out.push(']');
                    } else if let Some(pos) = attrs.find("href=\"") {
                        let rest = &attrs[pos + 6..];
                        let href = rest.split('"').next().unwrap_or("");
                        out.push('[');
                        out.push_str("](");
                        // 文本先占位，闭合时补——简化为「链接即文件名」不通用，
                        // 改为闭合时再追加：这里只记录
                        out.push_str("__LINKURL__");
                        out.push_str(href);
                        out.push(')');
                    }
                }
                "strong" | "b" => out.push_str("**"),
                "em" | "i" => out.push('*'),
                "table" => {
                    if closing {
                        // 渲染表格
                        if !table_rows.is_empty() {
                            let width = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                            for (ri, row) in table_rows.drain(..).enumerate() {
                                let mut row = row;
                                row.resize(width, String::new());
                                out.push_str("| ");
                                out.push_str(&row.join(" | "));
                                out.push_str(" |\n");
                                if ri == 0 {
                                    out.push('|');
                                    for _ in 0..width {
                                        out.push_str(" --- |");
                                    }
                                    out.push('\n');
                                }
                            }
                        }
                        newline_blank(&mut out);
                    } else {
                        table_rows.clear();
                    }
                }
                "tr" => {
                    if closing && !cur_row.is_empty() {
                        table_rows.push(std::mem::take(&mut cur_row));
                    }
                }
                "td" | "th" => {
                    if closing {
                        cur_row.push(cell.trim().replace('|', "\\|").replace('\n', " "));
                        cell.clear();
                        in_td = false;
                    } else {
                        in_td = true;
                    }
                }
                "title" => {
                    if !closing {
                        newline_blank(&mut out);
                        out.push_str("# ");
                    } else {
                        out.push('\n');
                        newline_blank(&mut out);
                    }
                }
                _ => {}
            }
            i = end + 1;
            continue;
        }
        if !in_script {
            let c = html[i..].chars().next().unwrap_or(' ');
            if in_td {
                cell.push(c);
            } else {
                out.push(c);
            }
            i += c.len_utf8();
            continue;
        }
        i += 1;
    }
    // 修正 <a> 简化产生的 __LINKURL__ 顺序：[文本](__LINKURL__url) → [文本](url)
    let fixed = out.replace("](__LINKURL__", "](");
    normalize(&fixed)
}

fn fenced(out: &mut String, open: bool, fence: &str) {
    if open {
        newline_blank(out);
        out.push_str(fence);
        out.push('\n');
    } else {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(fence);
        out.push('\n');
    }
}

fn newline_blank(out: &mut String) {
    if !out.is_empty() && !out.ends_with("\n\n") {
        if out.ends_with('\n') {
            out.push('\n');
        } else {
            out.push_str("\n\n");
        }
    }
}

fn find_byte(hay: &[u8], from: usize, needle: u8) -> Option<usize> {
    hay[from..].iter().position(|&b| b == needle).map(|p| p + from)
}

/// 全局归一化：与 python 版 _convert 尾部逻辑一致
pub fn normalize(s: &str) -> String {
    let mut out: String = String::new();
    let mut blank_run = 0;
    for line in s.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(line);
            out.push('\n');
        }
    }
    out.trim_start_matches('\n').trim_end_matches('\n').to_string()
}
