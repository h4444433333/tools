// Tauri App 后端：把 mdcore 引擎暴露成前端可调用的命令。
// 真文件浏览 + 转换后存源文件同目录。

use serde::Serialize;

#[derive(Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    ext: String,
}

/// 引擎能处理的扩展名
const SUPPORTED: &[&str] = &["docx", "html", "htm", "csv", "json", "txt", "md"];

#[tauri::command]
fn convert(path: String) -> Result<String, String> {
    mdcore::convert_file(&path).map_err(|e| e.to_string())
}

/// 转换并存 .md 到源文件同目录，返回输出路径
#[tauri::command]
fn convert_save(path: String) -> Result<String, String> {
    let md = mdcore::convert_file(&path).map_err(|e| e.to_string())?;
    let src = std::path::Path::new(&path);
    let out = src.with_extension("md");
    std::fs::write(&out, &md).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}

/// 列目录（文件夹排前，文件按名称）
#[tauri::command]
fn list_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = std::fs::read_dir(&path).map_err(|e| format!("无法打开该文件夹: {}", e))?;
    let mut entries: Vec<FileEntry> = Vec::new();
    for item in dir.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let meta = match item.metadata() { Ok(m) => m, Err(_) => continue };
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        entries.push(FileEntry {
            name,
            path: item.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size: if meta.is_file() { meta.len() } else { 0 },
            ext,
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir)
        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

/// 默认浏览起始路径（~/Documents）
#[tauri::command]
fn default_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let doc = format!("{}/Documents", home);
    if std::path::Path::new(&doc).exists() { doc } else { home }
}

/// 判断扩展名是否支持（前端可用来灰显）
#[tauri::command]
fn is_supported(ext: String) -> bool {
    SUPPORTED.contains(&ext.as_str())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![convert, convert_save, list_dir, default_dir, is_supported])
        .run(tauri::generate_context!())
        .expect("启动 mdapp 失败");
}
