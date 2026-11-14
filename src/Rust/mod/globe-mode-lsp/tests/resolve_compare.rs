//! S272: 自前の名前解決（`globe-mode-resolve`）と、いまの LSP の定義ジャンプ（`textDocument/definition`）を MSL の型名で突き合わせる。
//! `GLOBE_MSL=<data/msl> cargo test -p globe-mode-lsp --test resolve_compare -- --ignored --nocapture`

use globe_mode_lsp::Server;
use globe_mode_resolve::{Library, RefKind, Resolution};
use serde_json::json;
use std::path::{Path, PathBuf};

fn uri(p: &Path) -> String {
    format!("file://{}", p.display().to_string().replace('\\', "/"))
}

/// バイト位置 → (行, UTF-16 の列)
fn pos(text: &str, off: usize) -> (usize, usize) {
    let before = &text[..off];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, text[start..off].encode_utf16().count())
}

#[test]
#[ignore]
fn definition_agrees_on_msl_type_names() {
    let root = std::env::var_os("GLOBE_MSL").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl"),
        PathBuf::from,
    );
    if !root.is_dir() {
        eprintln!("MSL が無いので省く");
        return;
    }
    let lib = Library::from_dir(&root).unwrap();
    // 型名（Type）で、自前が型に解決したものを、ファイルごとに 1 つおきに最大 3 つ（ファイルは 40 本おき）
    let mut by_file: std::collections::BTreeMap<u32, Vec<usize>> = Default::default();
    for (i, r) in lib.refs.iter().enumerate() {
        if r.kind == RefKind::Type && matches!(lib.resolve(r), Resolution::Class(_)) {
            by_file.entry(r.file).or_default().push(i);
        }
    }
    let mut s = Server::new(std::slice::from_ref(&root)).unwrap();
    s.handle(json!({"id": 0, "method": "initialize", "params": {"rootUri": uri(&root)}}));
    let (mut total, mut agree, mut missing) = (0, 0, 0);
    let mut diffs = Vec::new();
    for (k, (file, ids)) in by_file.iter().enumerate() {
        if k % 40 != 0 {
            continue;
        }
        let path = Path::new(&lib.files[*file as usize]);
        let text = std::fs::read_to_string(path).unwrap();
        let u = uri(path);
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": u, "languageId": "modelica", "version": 1, "text": text}}}),
        );
        for &i in ids.iter().step_by(ids.len().div_ceil(3).max(1)).take(3) {
            let r = &lib.refs[i];
            let Resolution::Class(c) = lib.resolve(r) else {
                continue;
            };
            // 名前の最後の字の位置（点つきの名前の全体が引かれる）
            let (line, ch) = pos(&text, usize::from(r.range.end()) - 1);
            let out = s.handle(
                json!({"id": 1, "method": "textDocument/definition", "params": {
                "textDocument": {"uri": u}, "position": {"line": line, "character": ch}}}),
            );
            total += 1;
            let res = &out[0]["result"];
            if res.is_null() {
                missing += 1;
                diffs.push(format!(
                    "LSP は null: {} {} in {}",
                    r.parts.join("."),
                    line + 1,
                    path.display()
                ));
                continue;
            }
            let cl = &lib.classes[c];
            let cpath = Path::new(&lib.files[cl.file as usize]);
            let ctext = std::fs::read_to_string(cpath).unwrap();
            let (cl_line, cl_ch) = pos(&ctext, usize::from(cl.name_range.start()));
            let want = json!({"line": cl_line, "character": cl_ch});
            // LSP の URI は空白を %20 にする
            let got_uri = res["uri"].as_str().unwrap_or_default().replace("%20", " ");
            if got_uri.ends_with(&cpath.display().to_string()) && res["range"]["start"] == want {
                agree += 1;
            } else {
                diffs.push(format!(
                    "{} in {}:{}: 自前 {} @{want} / LSP {} @{}",
                    r.parts.join("."),
                    path.display(),
                    line + 1,
                    lib.qualified_name(c),
                    got_uri,
                    res["range"]["start"]
                ));
            }
        }
    }
    eprintln!(
        "突き合わせ {total} 箇所: 一致 {agree}、LSP が null {missing}、不一致 {}",
        total - agree - missing
    );
    for d in &diffs {
        eprintln!("  {d}");
    }
}
