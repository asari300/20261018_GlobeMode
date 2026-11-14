use globe_mode_resolve::{Library, Reason, Resolution};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn describe(lib: &Library, r: &Resolution) -> String {
    match r {
        Resolution::Class(c) => format!("class {}", lib.qualified_name(*c)),
        Resolution::Component(c, i) => {
            format!(
                "comp {}.{}",
                lib.qualified_name(*c),
                lib.classes[*c].components[*i].name
            )
        }
        Resolution::Local => "local".into(),
        Resolution::Builtin => "builtin".into(),
        Resolution::Unresolved(why) => format!("unresolved {why:?}"),
    }
}

/// `(使われた場所のクラス, 名前)` -> 解決の結果
fn table(lib: &Library) -> BTreeMap<(String, String), String> {
    lib.refs
        .iter()
        .map(|r| {
            (
                (lib.qualified_name(r.scope), r.parts.join(".")),
                describe(lib, &lib.resolve(r)),
            )
        })
        .collect()
}

#[test]
fn small_library() {
    let lib = Library::from_sources([
        // ディレクトリ構造の package.mo
        (
            "P/package.mo",
            "within; package P constant Real k = 1; model A Real x; end A; end P;",
        ),
        // 単一ファイル（入れ子のクラスと import 4 種）
        (
            "P/Sub.mo",
            "within P; package Sub model C Real c; end C; model D Real d; end D; end Sub;",
        ),
        (
            "P/M.mo",
            "within P;
model M
  import P.Sub.C;
  import S = P.Sub;
  import P.Sub.*;
  import P.Sub.{D};
  import P.k;
  C c1; S.C c2; D d1; A a; Real y = k + a.x + time + sin(3);
  P.A a2;
  Missing m;
  Real z[2] = {i for i in 1:2};
equation
  for i in 1:2 loop a.x = i; end for;
end M;",
        ),
        // extends: 受け継いだ部品と入れ子のクラス。`extends` の名前は自分の受け継ぎを見ない
        (
            "P/N.mo",
            "within P;
model N
  extends A;
  Real w = x;
  model Inner Real q = x; end Inner;
  Inner i;
end N;",
        ),
        // encapsulated は外を見ない（最上位は見える）
        (
            "P/E.mo",
            "within P; encapsulated model E A a; P.A b; Real r = k; end E;",
        ),
        // 継承の輪
        (
            "P/Cyc.mo",
            "within P; package Cyc model X extends Y; end X; model Y extends X; end Y; model Z extends X; Real v = nothing; end Z; end Cyc;",
        ),
        // redeclare と replaceable
        (
            "P/R.mo",
            "within P; model R replaceable package Med = P.Sub; Med.C c; Med.Nope n; end R;",
        ),
    ]);
    assert!(lib.diagnostics.is_empty(), "{:?}", lib.diagnostics);
    let t = table(&lib);
    let get = |scope: &str, name: &str| t[&(scope.to_string(), name.to_string())].as_str();
    // 入れ子の外側・import（修飾・別名・`*`・個別）
    assert_eq!(get("P.M", "C"), "class P.Sub.C");
    assert_eq!(get("P.M", "S.C"), "class P.Sub.C");
    assert_eq!(get("P.M", "D"), "class P.Sub.D");
    assert_eq!(get("P.M", "A"), "class P.A");
    assert_eq!(get("P.M", "P.A"), "class P.A");
    assert_eq!(get("P.M", "k"), "comp P.k");
    assert_eq!(get("P.M", "a.x"), "comp P.M.a");
    assert_eq!(get("P.M", "time"), "builtin");
    assert_eq!(get("P.M", "sin"), "builtin");
    assert_eq!(get("P.M", "Missing"), "unresolved NotFound");
    assert_eq!(get("P.M", "i"), "local");
    // extends
    assert_eq!(get("P.N", "A"), "class P.A");
    assert_eq!(get("P.N", "x"), "comp P.A.x");
    assert_eq!(get("P.N.Inner", "x"), "comp P.A.x");
    // encapsulated
    assert_eq!(get("P.E", "A"), "unresolved NotFound");
    assert_eq!(get("P.E", "P.A"), "class P.A");
    // 循環は未解決として出る（落ちない）
    assert_eq!(get("P.Cyc.Z", "nothing"), "unresolved Cycle");
    // replaceable なクラスの中の名前は型が変わりうる
    assert_eq!(get("P.R", "Med.C"), "class P.Sub.C");
    assert_eq!(get("P.R", "Med.Nope"), "unresolved Redeclare");
    assert_eq!(
        lib.class_by_name("P.Sub.C")
            .map(|c| lib.qualified_name(c))
            .as_deref(),
        Some("P.Sub.C")
    );
}

#[test]
fn enum_and_builtin_shadow() {
    let lib = Library::from_sources([(
        "a.mo",
        "package Q type E = enumeration(a, b \"x\", c); model T E e = E.b; Real Real2; end T; end Q;",
    )]);
    let t = table(&lib);
    assert_eq!(t[&("Q.T".into(), "E.b".into())], "comp Q.E.b");
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn msl_dir() -> PathBuf {
    std::env::var_os("GLOBE_MSL").map_or_else(|| root().join("data/msl"), PathBuf::from)
}

fn count(lib: &Library) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for r in &lib.refs {
        let res = lib.resolve(r);
        let k = match res {
            Resolution::Unresolved(w) => format!("unresolved {w:?}"),
            Resolution::Class(_) => "class".into(),
            Resolution::Component(..) => "component".into(),
            Resolution::Local => "local".into(),
            Resolution::Builtin => "builtin".into(),
        };
        *m.entry(format!("{:?} {k}", r.kind)).or_default() += 1;
    }
    m
}

/// `src/Modelica`（MSL が無ければ Modelica・ModelicaServices への参照だけは数えない）で、redeclare 以外の未解決は 0
#[test]
fn validation_has_no_unresolved() {
    let msl = msl_dir();
    let have_msl = msl.is_dir();
    let mut dirs = vec![root().join("src/Modelica")];
    if have_msl {
        dirs.push(msl);
    }
    let dirs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    let lib = Library::from_dirs(&dirs).unwrap();
    let mut bad = Vec::new();
    for r in &lib.refs {
        if let Resolution::Unresolved(why) = lib.resolve(r) {
            let msl_ref = matches!(
                r.parts[0].as_str(),
                "Modelica" | "ModelicaServices" | "Complex"
            );
            if why != Reason::Redeclare && (have_msl || !msl_ref) {
                bad.push(format!(
                    "{}:{:?} {} {:?}",
                    lib.files[r.file as usize],
                    r.range,
                    r.parts.join("."),
                    why
                ));
            }
        }
    }
    // MSL が無いと、MSL から受け継いだ名前（Inherited）は確かめられない
    if !have_msl {
        bad.retain(|b| !b.ends_with("Inherited"));
    }
    assert!(
        bad.is_empty(),
        "{} unresolved:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// MSL 4.1.0 の全 `.mo` の解決率。`GLOBE_MSL=... cargo test --release -p globe-mode-resolve -- --ignored --nocapture`
#[test]
#[ignore]
fn msl_stats() {
    let dir = msl_dir();
    if !dir.is_dir() {
        eprintln!("MSL が無いので省く: {}", dir.display());
        return;
    }
    let t0 = std::time::Instant::now();
    let lib = Library::from_dir(&dir).unwrap();
    let t1 = t0.elapsed();
    let counts = count(&lib);
    eprintln!(
        "files={} classes={} refs={} diagnostics={} build={t1:?} resolve={:?}",
        lib.files.len(),
        lib.classes.len(),
        lib.refs.len(),
        lib.diagnostics.len(),
        t0.elapsed() - t1
    );
    for (k, v) in &counts {
        eprintln!("{k}: {v}");
    }
    let mut shown: BTreeMap<String, usize> = BTreeMap::new();
    for r in &lib.refs {
        if let Resolution::Unresolved(why) = lib.resolve(r) {
            let key = format!("{why:?} {:?} {}", r.kind, r.parts.join("."));
            *shown.entry(key).or_default() += 1;
        }
    }
    if let Ok(w) = std::env::var("GLOBE_WHY") {
        for r in lib.refs.iter().filter(|r| r.parts.join(".") == w).take(8) {
            eprintln!(
                "  {:?} in {} {:?}",
                lib.resolve(r),
                lib.qualified_name(r.scope),
                lib.files[r.file as usize]
            );
        }
    }
    let mut top: Vec<_> = shown.into_iter().collect();
    top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (k, n) in top.iter().take(60) {
        eprintln!("{n:6} {k}");
    }
}
