//! 手書きの `include/globe_mode.h` の署名が `src/lib.rs` の `extern "C"` と一致するかを確かめる（S075）。

use std::collections::BTreeMap;

/// 空白を詰めた「戻り値 名前(引数の型,…)」の形にする。
type Sigs = BTreeMap<String, String>;

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Rust の型を C の型の書き方に直す。
fn c_type(rust: &str) -> String {
    match rust.trim() {
        "" => "void",
        "*const c_char" => "const char *",
        "*mut c_char" => "char *",
        "*mut GmWorkspace" => "GmWorkspace *",
        "c_int" => "int",
        "i64" => "int64_t",
        "u32" => "uint32_t",
        "usize" => "size_t",
        "*mut f64" => "double *",
        "*mut *mut f64" => "double **",
        "*mut usize" => "size_t *",
        other => panic!("表に無い型: {other}"),
    }
    .to_string()
}

fn rust_sigs(src: &str) -> Sigs {
    let mut out = Sigs::new();
    for chunk in src.split("extern \"C\" fn ").skip(1) {
        let name = &chunk[..chunk.find('(').unwrap()];
        let close = chunk.find(')').unwrap();
        let params: Vec<String> = chunk[name.len() + 1..close]
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .map(|p| c_type(p.split(':').nth(1).unwrap()))
            .collect();
        let rest = &chunk[close + 1..chunk.find('{').unwrap()];
        let ret = c_type(rest.trim().trim_start_matches("->"));
        out.insert(name.to_string(), format!("{ret} ({})", params.join(", ")));
    }
    out
}

fn header_sigs(h: &str) -> Sigs {
    let mut out = Sigs::new();
    for line in h.lines().map(str::trim) {
        if line.starts_with("/*") || line.starts_with('*') || !line.ends_with(");") {
            continue;
        }
        let open = line.find('(').unwrap();
        let head = &line[..open];
        let at = head.rfind([' ', '*']).unwrap() + 1;
        let ret = squash(&head[..at]);
        let params: Vec<String> = line[open + 1..line.len() - 2]
            .split(',')
            .filter(|p| !matches!(p.trim(), "" | "void"))
            .map(|p| {
                // 引数名（最後の語）を落とす
                let p = p.trim();
                let cut = p.rfind([' ', '*']).unwrap() + 1;
                squash(&p[..cut])
            })
            .collect();
        out.insert(
            head[at..].to_string(),
            format!("{ret} ({})", params.join(", ")),
        );
    }
    out
}

#[test]
fn header_matches_extern_c_signatures() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let rust = rust_sigs(&std::fs::read_to_string(format!("{dir}/src/lib.rs")).unwrap());
    let header =
        header_sigs(&std::fs::read_to_string(format!("{dir}/include/globe_mode.h")).unwrap());
    assert!(rust.len() >= 13, "{rust:?}");
    assert_eq!(rust, header);
}
