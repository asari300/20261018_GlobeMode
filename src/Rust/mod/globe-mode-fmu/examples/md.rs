//! `modelDescription.xml` を標準出力に書く（S231。`fmu-export.yml` が FMU に詰めるために使う）。
//!
//! 使い方: cargo run -p globe-mode-fmu --example md -- <model.mo> <完全修飾名> <2|3> <me|cs|both>

use globe_mode_fmu::{Kind, model_description_kind};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [path, model, version, kind] = &a[..] else {
        eprintln!("使い方: md <model.mo> <完全修飾名> <2|3> <me|cs|both>");
        std::process::exit(2);
    };
    let kind = match kind.as_str() {
        "me" => Kind::Me,
        "cs" => Kind::Cs,
        "both" => Kind::Both,
        k => {
            eprintln!("kind は me・cs・both: {k}");
            std::process::exit(2);
        }
    };
    let text = std::fs::read_to_string(path).expect("model.mo を読めない");
    match model_description_kind(model, &text, version == "3", kind) {
        Ok(xml) => print!("{xml}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
