//! 命令行入口：mdcore <文件> [-o 输出.md]。Tauri 后端未来直接调用同一 convert_file。
use mdcore::convert_file;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input = None;
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                output = args.get(i + 1).cloned();
                i += 1;
            }
            a => {
                input = Some(a.to_string());
            }
        }
        i += 1;
    }
    let Some(path) = input else {
        eprintln!("用法: mdcore <文件> [-o 输出.md]");
        std::process::exit(2);
    };
    match convert_file(&path) {
        Ok(md) => match output {
            Some(o) => {
                if let Err(e) = std::fs::write(&o, md) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            None => print!("{md}"),
        },
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
