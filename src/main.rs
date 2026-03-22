use aqkanji2koe::AqKanji2Koe;
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let use_roman = args.iter().any(|a| a == "--roman" || a == "-r");
    let text_arg = args.iter().skip(1).find(|a| !a.starts_with('-')).cloned();

    let converter = AqKanji2Koe::new().unwrap_or_else(|e| {
        eprintln!("初期化エラー: {e}");
        std::process::exit(1);
    });

    let convert = |text: &str| {
        if use_roman {
            converter.convert_roman(text)
        } else {
            converter.convert(text)
        }
    };

    if let Some(text) = text_arg {
        match convert(&text) {
            Ok(result) => println!("{result}"),
            Err(e) => {
                eprintln!("変換エラー: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for line in stdin.lock().lines() {
            let text = line.unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            match convert(&text) {
                Ok(result) => {
                    writeln!(out, "{result}").ok();
                }
                Err(e) => {
                    eprintln!("変換エラー: {e}");
                }
            }
        }
    }
}
