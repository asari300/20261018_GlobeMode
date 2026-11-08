//! 参照・リネームがワークスペースの根の下の開いていない `.mo` とライブラリ（`-L`）に届く（S230）。

use globe_mode_lsp::Server;
use serde_json::{Value, json};

fn uri(p: &std::path::Path) -> String {
    // Windows: `C:\x` -> `file:///C:/x`
    let p = p.display().to_string().replace('\\', "/");
    format!("file://{}{p}", if p.starts_with('/') { "" } else { "/" })
}

#[test]
fn references_and_rename_reach_unopened_files_and_libraries() {
    let dir = std::env::temp_dir().join(format!("gm-lsp-cross-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (root, lib) = (dir.join("ws"), dir.join("lib"));
    for d in [root.join("sub"), root.join(".hidden"), lib.clone()] {
        std::fs::create_dir_all(d).unwrap();
    }
    let defs = "model Base\n  Real x;\nend Base;\n";
    // 文字列の中の非 ASCII（UTF-8 で 2 バイト、UTF-16 で 1 単位）の後ろと CRLF
    let user = "model Use\r\n  String t = \"αα\"; Base b;\r\nend Use;\r\n";
    let write = |p: std::path::PathBuf, t: &str| std::fs::write(p, t).unwrap();
    write(root.join("Defs.mo"), defs);
    write(root.join("Use.mo"), user);
    write(
        root.join("sub/Plain.mo"),
        "model Plain\n  Real Base;\nend Plain;\n",
    ); // 部品の Base は別物
    write(
        root.join(".hidden/Hid.mo"),
        "model Hid\n  Base b;\nend Hid;\n",
    );
    write(lib.join("L.mo"), "model LibUser\n  Base z;\nend LibUser;\n");

    let mut s = Server::new(std::slice::from_ref(&lib)).unwrap();
    s.handle(json!({"id": 0, "method": "initialize", "params": {"rootUri": uri(&root)}}));
    let defs_uri = uri(&root.join("Defs.mo"));
    s.handle(
        json!({"method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": defs_uri, "languageId": "modelica", "version": 1, "text": defs}}}),
    );
    let ask = |s: &mut Server, m: &str| -> Vec<Value> {
        s.handle(
            json!({"id": 1, "method": m, "params": {"textDocument": {"uri": defs_uri},
            "position": {"line": 0, "character": 7}, "newName": "Core"}}),
        )
    };

    let r = ask(&mut s, "textDocument/references").remove(0);
    let mut hits: Vec<(String, u64, u64)> = r["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let f = l["uri"]
                .as_str()
                .unwrap()
                .rsplit('/')
                .next()
                .unwrap()
                .to_string();
            (
                f,
                l["range"]["start"]["line"].as_u64().unwrap(),
                l["range"]["start"]["character"].as_u64().unwrap(),
            )
        })
        .collect();
    hits.sort();
    // 宣言と end、Use.mo（CRLF の 2 行目、非 ASCII の後）、ライブラリの L.mo。.hidden と部品の Base は含まない
    let want: Vec<(String, u64, u64)> = [
        ("Defs.mo", 0, 6),
        ("Defs.mo", 2, 4),
        ("L.mo", 1, 2),
        ("Use.mo", 1, 19),
    ]
    .map(|(f, l, c)| (f.to_string(), l, c))
    .into();
    assert_eq!(hits, want, "{r}");

    // リネーム: changes は Defs.mo と Use.mo の 2 ファイル。ライブラリは含めず、知らせる
    let out = ask(&mut s, "textDocument/rename");
    let changes = out[0]["result"]["changes"].as_object().unwrap();
    let mut keys: Vec<_> = changes
        .keys()
        .map(|k| k.rsplit('/').next().unwrap())
        .collect();
    keys.sort();
    assert_eq!(keys, ["Defs.mo", "Use.mo"], "{}", out[0]);
    let use_edits = changes
        .iter()
        .find(|(k, _)| k.ends_with("Use.mo"))
        .unwrap()
        .1;
    assert_eq!(
        use_edits[0]["range"]["start"],
        json!({"line": 1, "character": 19})
    );
    assert_eq!(use_edits[0]["newText"], "Core");
    assert!(
        out[1]["params"]["message"]
            .as_str()
            .unwrap()
            .contains("ライブラリの 1 箇所"),
        "{out:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
