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

    // 同名の部品だけを持つファイルは、解決できない候補なので rename は断る。消すと通る
    let r = ask(&mut s, "textDocument/rename").remove(0);
    assert_eq!(r["error"]["code"], -32803, "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("Plain.mo"),
        "{r}"
    );
    std::fs::remove_file(root.join("sub/Plain.mo")).unwrap();

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

    // import の別名を通る参照でも `D.Base` は `Elsewhere.Pkg.Base`（ここにはどこにも無い）で、この `Base` とは別。解決できない候補は黙って取りこぼさず、ファイルを挙げて断る（S235 以降、解決できる分は下の試験で `changes` に入る）
    write(
        root.join("Imp.mo"),
        "model Imp\n  import D = Elsewhere.Pkg;\n  D.Base b;\nend Imp;\n",
    );
    let r = ask(&mut s, "textDocument/rename").remove(0);
    assert_eq!(r["error"]["code"], -32803, "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("Imp.mo"),
        "{r}"
    );
    // 参照の一覧は引けた分だけ返す
    let r = ask(&mut s, "textDocument/references").remove(0);
    assert_eq!(r["result"].as_array().unwrap().len(), 4, "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- S235: 継承した部品と import の別名 ----

/// 根 `root` に `files` を書いて、`opened` の文書を開いたサーバ。
fn server_with(tag: &str, files: &[(&str, &str)], opened: &[&str]) -> (Server, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("gm-lsp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (f, t) in files {
        std::fs::write(dir.join(f), t).unwrap();
    }
    let mut s = Server::new(&[]).unwrap();
    s.handle(json!({"id": 0, "method": "initialize", "params": {"rootUri": uri(&dir)}}));
    for f in opened {
        let text = files.iter().find(|(n, _)| n == f).unwrap().1;
        s.handle(
            json!({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri(&dir.join(f)), "languageId": "modelica", "version": 1, "text": text}}}),
        );
    }
    (s, dir)
}

fn call(
    s: &mut Server,
    dir: &std::path::Path,
    m: &str,
    f: &str,
    line: u32,
    ch: u32,
    new: &str,
) -> Value {
    s.handle(
        json!({"id": 1, "method": m, "params": {"textDocument": {"uri": uri(&dir.join(f))},
        "position": {"line": line, "character": ch}, "newName": new}}),
    )
    .remove(0)
}

/// `changes` を (ファイル名, 行, 列) の並びにする（並べ替え済み）。
fn edits(r: &Value) -> Vec<(String, u64, u64)> {
    let mut v: Vec<(String, u64, u64)> = r["result"]["changes"]
        .as_object()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .flat_map(|(k, es)| {
            let f = k.rsplit('/').next().unwrap().to_string();
            es.as_array().unwrap().iter().map(move |e| {
                (
                    f.clone(),
                    e["range"]["start"]["line"].as_u64().unwrap(),
                    e["range"]["start"]["character"].as_u64().unwrap(),
                )
            })
        })
        .collect();
    v.sort();
    v
}

const INHERIT: &[(&str, &str)] = &[
    ("A.mo", "model A\n  Real x;\n  Real w;\nend A;\n"),
    (
        "B.mo",
        "model B\n  extends A;\nequation\n  der(x) = 1;\nend B;\n",
    ),
    (
        "C.mo",
        "model C\n  extends B;\n  B b;\n  Real y = b.x;\nend C;\n",
    ),
    (
        "Other.mo",
        "model Other\n  Real x;\nequation\n  der(x) = 1;\nend Other;\n",
    ),
];

fn inherit_want() -> Vec<(String, u64, u64)> {
    // 定義元の宣言、B の中、C の `b.x`。無関係な Other の x は含まない
    [("A.mo", 1, 7), ("B.mo", 3, 6), ("C.mo", 3, 13)]
        .map(|(f, l, c)| (f.to_string(), l, c))
        .into()
}

#[test]
fn inherited_component_rename_and_references_from_the_definition() {
    let (mut s, dir) = server_with("inh-def", INHERIT, &["A.mo"]);
    let r = call(&mut s, &dir, "textDocument/rename", "A.mo", 1, 7, "z");
    assert_eq!(edits(&r), inherit_want(), "{r}");
    let r = call(&mut s, &dir, "textDocument/references", "A.mo", 1, 7, "");
    let mut got: Vec<_> = r["result"]
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
    got.sort();
    assert_eq!(got, inherit_want(), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn inherited_component_rename_from_the_inheritor_reaches_the_definition() {
    let (mut s, dir) = server_with("inh-sub", INHERIT, &["A.mo", "B.mo"]);
    // B の中の `der(x)` の x から: 定義元 A.x と、B を継承する C、`b.x` に届く
    let p = call(&mut s, &dir, "textDocument/prepareRename", "B.mo", 3, 6, "");
    assert_eq!(
        p["result"]["start"],
        json!({"line": 3, "character": 6}),
        "{p}"
    );
    let r = call(&mut s, &dir, "textDocument/rename", "B.mo", 3, 6, "z");
    assert_eq!(edits(&r), inherit_want(), "{r}");
    // 同じ階層に既にある名前は断る
    let r = call(&mut s, &dir, "textDocument/rename", "B.mo", 3, 6, "w");
    assert_eq!(r["error"]["code"], -32803, "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

const IMPORTS: &[(&str, &str)] = &[
    (
        "Defs.mo",
        "within Elsewhere.Pkg;\nmodel Base\n  Real x;\nend Base;\n",
    ),
    (
        "Imp.mo",
        "model Imp\n  import D = Elsewhere.Pkg;\n  D.Base b;\nend Imp;\n",
    ),
    (
        "Wild.mo",
        "model Wild\n  import Elsewhere.Pkg.*;\n  Base b;\nend Wild;\n",
    ),
];

#[test]
fn class_rename_follows_an_import_alias_and_a_wildcard_import() {
    let (mut s, dir) = server_with("imp", IMPORTS, &["Defs.mo"]);
    let r = call(&mut s, &dir, "textDocument/rename", "Defs.mo", 1, 7, "Core");
    // `D.Base` は Base の部分（列 4）だけ。ワイルドカードは `Base b;`（列 2）
    let want: Vec<(String, u64, u64)> = [
        ("Defs.mo", 1, 6),
        ("Defs.mo", 3, 4),
        ("Imp.mo", 2, 4),
        ("Wild.mo", 2, 2),
    ]
    .map(|(f, l, c)| (f.to_string(), l, c))
    .into();
    assert_eq!(edits(&r), want, "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn import_alias_rename_stays_in_its_document() {
    let (mut s, dir) = server_with("alias", IMPORTS, &["Imp.mo"]);
    // `import D = …` の D と、`D.Base` の頭 D。Base や Elsewhere は変えない。ほかのファイルは含まない
    for (line, ch) in [(1, 9), (2, 2)] {
        let r = call(&mut s, &dir, "textDocument/rename", "Imp.mo", line, ch, "E");
        let want: Vec<(String, u64, u64)> = [("Imp.mo", 1, 9), ("Imp.mo", 2, 2)]
            .map(|(f, l, c)| (f.to_string(), l, c))
            .into();
        assert_eq!(edits(&r), want, "{r}");
    }
    // 同じクラスにある名前とは衝突させない
    let r = call(&mut s, &dir, "textDocument/rename", "Imp.mo", 2, 2, "b");
    assert_eq!(r["error"]["code"], -32803, "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}
