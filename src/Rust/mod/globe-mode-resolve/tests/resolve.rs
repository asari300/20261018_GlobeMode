use globe_mode_resolve::{Library, Reason, RefKind, Resolution};
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
    assert_eq!(get("P.M", "a.x"), "comp P.A.x");
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

/// S275: 成分参照の続き（`a.b.c`）。添字は無視、受け継いだ部品、組み込みの型の属性、部品の型の途中の失敗
#[test]
fn component_paths() {
    let lib = Library::from_sources([(
        "P.mo",
        "package P
  connector Pin Real v; flow Real i; end Pin;
  model Two Pin p; Pin n; end Two;
  model Sub extends Two; end Sub;
  type Volt = Real;
  model M
    Two r[2];
    Volt u;
    Sub s;
    Real a = r[1].p.v + u.start + s.n.i;
    Real b = r[1].p.nope;
    Real c = r[1].q.v;
    Real d = u.x.y;
  end M;
end P;",
    )]);
    assert!(lib.diagnostics.is_empty(), "{:?}", lib.diagnostics);
    let t = table(&lib);
    let get = |name: &str| t[&("P.M".to_string(), name.to_string())].as_str();
    assert_eq!(get("r.p.v"), "comp P.Pin.v");
    assert_eq!(get("u.start"), "builtin");
    assert_eq!(get("s.n.i"), "comp P.Pin.i");
    assert_eq!(get("r.p.nope"), "unresolved NotFound");
    assert_eq!(get("r.q.v"), "unresolved NotFound");
    // 属性の先は部品ではない（`resolve_head` は先頭の部品で止まる）
    assert_eq!(get("u.x.y"), "builtin");
    let r = lib
        .refs
        .iter()
        .find(|r| r.parts == ["r", "p", "v"])
        .unwrap();
    assert_eq!(describe(&lib, &lib.resolve_head(r)), "comp P.M.r");
}

/// S275: `replaceable` の既定の型・`constrainedby`・`redeclare package`（`extends` の修飾・部品の修飾・short class の修飾）
#[test]
fn redeclare_and_replaceable() {
    let lib = Library::from_sources([(
        "R.mo",
        "package R
  package Base constant Real k = 1; end Base;
  package Ext extends Base; constant Real e = 2; end Ext;
  package Other constant Real o = 3; end Other;
  model A
    replaceable package Medium = Ext;
    replaceable package Med2 = Other constrainedby Ext;
    Real d = Medium.e + Medium.k + Med2.e + Medium.o;
  end A;
  model B
    extends A(redeclare package Medium = Other);
    Real x = Medium.o;
  end B;
  model C
    A a(redeclare replaceable package Medium = Other constrainedby Base);
    Real y = a.Medium.o + a.Med2.e;
  end C;
  package Pk replaceable package Medium = Ext; end Pk;
  package P2 = Pk(redeclare package Medium = Other);
  model D
    Real z = P2.Medium.o;
  end D;
  model E
    replaceable Ext2 c constrainedby Base2;
    Real w = c.base + c.more;
  end E;
  model Base2 Real base; end Base2;
  model Ext2 extends Base2; Real more; end Ext2;
end R;",
    )]);
    assert!(lib.diagnostics.is_empty(), "{:?}", lib.diagnostics);
    let t = table(&lib);
    let get = |scope: &str, name: &str| t[&(format!("R.{scope}"), name.to_string())].as_str();
    // 既定の型（`Ext`）と、`constrainedby Ext` の中の `Ext` は受け継ぎでも見える。既定に無い名前は差し替わりうる
    assert_eq!(get("A", "Medium.e"), "comp R.Ext.e");
    assert_eq!(get("A", "Medium.k"), "comp R.Base.k");
    assert_eq!(get("A", "Med2.e"), "comp R.Ext.e");
    assert_eq!(get("A", "Medium.o"), "unresolved Redeclare");
    // extends の修飾・部品の修飾・short class の修飾の redeclare
    assert_eq!(get("B", "Medium.o"), "comp R.Other.o");
    assert_eq!(get("C", "a.Medium.o"), "comp R.Other.o");
    assert_eq!(get("C", "a.Med2.e"), "comp R.Ext.e");
    assert_eq!(get("D", "P2.Medium.o"), "comp R.Other.o");
    // 部品の `constrainedby`: 型は両方を試す
    assert_eq!(get("E", "c.base"), "comp R.Base2.base");
    assert_eq!(get("E", "c.more"), "comp R.Ext2.more");
}

/// S341・S345 の試験が使う、差し替えた型の例（`class extends` の `ThermodynamicState` と `extends B(redeclare T x)`）
const REDECLARED: &str = "package S
  package PM
    replaceable record TS end TS;
    replaceable partial model BP TS state; end BP;
    replaceable partial function f input TS s; end f;
  end PM;
  package Simple
    extends PM;
    redeclare record extends TS Real T; end TS;
    redeclare model extends BP Real x = state.T + state.nope; end BP;
    redeclare function extends f Real y = s.T; end f;
  end Simple;
  model Port Real p; end Port;
  model PortX extends Port; Real hot; end PortX;
  model Base replaceable Port port; end Base;
  model Machine
    extends Base(redeclare PortX port);
    Real a = port.hot + port.p + port.cold;
  end Machine;
  model Other
    extends Base;
    Real b = port.hot;
  end Other;
end S;";

/// S341: 既定の型で引けず `Redeclare` になる成分参照を、差し替えた後の型で引く。
/// `extends B(redeclare T x)`（部品の再宣言）と、`class extends` で差し替えた型の名前を受け継いだクラスから見直す形
#[test]
fn redeclared_component_types() {
    let lib = Library::from_sources([("S.mo", REDECLARED)]);
    assert!(lib.diagnostics.is_empty(), "{:?}", lib.diagnostics);
    let t = table(&lib);
    let get = |scope: &str, name: &str| t[&(format!("S.{scope}"), name.to_string())].as_str();
    // `ThermodynamicState` を `class extends` で差し替えた `Simple` の `BaseProperties`・関数から見ると `T` は引ける
    assert_eq!(get("Simple.BP", "state.T"), "comp S.Simple.TS.T");
    assert_eq!(get("Simple.f", "s.T"), "comp S.Simple.TS.T");
    // 差し替えた型にも無い名前は Redeclare のまま
    assert_eq!(get("Simple.BP", "state.nope"), "unresolved Redeclare");
    // `extends Base(redeclare PortX port)` で差し替えた部品の型。既定の型の名前も引ける
    assert_eq!(get("Machine", "port.hot"), "comp S.PortX.hot");
    assert_eq!(get("Machine", "port.p"), "comp S.Port.p");
    assert_eq!(get("Machine", "port.cold"), "unresolved Redeclare");
    // 差し替えていない `Other` では `hot` は引けない
    assert_eq!(get("Other", "port.hot"), "unresolved NotFound");
}

/// S345: 補完の `members_of` も、`scope` から見て差し替わった後の型の要素を出す（S341 と同じ規則）
#[test]
fn members_of_follows_redeclared_types() {
    let lib = Library::from_sources([("S.mo", REDECLARED)]);
    let at = |scope: &str, parts: &[&str]| {
        let c = lib.class_by_name(scope).unwrap();
        let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        names(&lib.members_of(c, &parts).unwrap()).join(",")
    };
    // `class extends` で差し替えた `TS`（既定の `PM.TS` は空）
    assert_eq!(at("S.Simple.BP", &["state"]), "T");
    // `extends Base(redeclare PortX port)`。差し替えていない `Other` は既定の `Port` のまま
    assert_eq!(at("S.Machine", &["port"]), "hot,p");
    assert_eq!(at("S.Other", &["port"]), "p");
}

/// S351（#132）: 差し替えていない（`redeclare` でも `replaceable` でもない）同名の別の型は、補完の候補に出さない
#[test]
fn members_of_ignores_unrelated_same_name_types() {
    let lib = Library::from_sources([(
        "S.mo",
        "package Lib
  connector Pin Real v; flow Real i; end Pin;
  partial model OnePort Pin p; end OnePort;
  record T Real a; end T;
  model Base T t; end Base;
end Lib;
package MyLib
  connector Pin Real x; end Pin;
  record T Real z; end T;
  model R extends Lib.OnePort; end R;
  model D
    extends Lib.Base;
    record T Real y; end T;
  end D;
end MyLib;",
    )]);
    let at = |scope: &str, parts: &[&str]| {
        let c = lib.class_by_name(scope).unwrap();
        let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        names(&lib.members_of(c, &parts).unwrap()).join(",")
    };
    assert_eq!(at("MyLib.R", &["p"]), "i,v");
    assert_eq!(at("Lib.OnePort", &["p"]), "i,v");
    // 継ぐ側の外側にも、局所にも同名の `T` があるが、差し替えてはいない
    assert_eq!(at("MyLib.D", &["t"]), "a");
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

/// `.history/` のような隠しフォルダの `.mo` は読まない（S358。同名のクラスが二重にならない。fmt・LSP・frontend の `collect_mo_files` と同じ規則）
#[test]
fn from_dirs_skips_hidden_dirs() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("resolve_hidden_dirs");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".history")).unwrap();
    std::fs::write(dir.join("A.mo"), "model A Real x; end A;").unwrap();
    std::fs::write(dir.join(".history/A.mo"), "model A Real x; end A;").unwrap();
    let lib = Library::from_dir(&dir).unwrap();
    assert_eq!(lib.files.len(), 1, "{:?}", lib.files);
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
    // 成分参照の続き（`a.b.c`）: S275 の前は先頭で止めていた（`resolve_head`）。いまは続きまで（`resolve`）
    let (mut multi, mut head_ok, mut full_ok) = (0, 0, 0);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    let mut attrs: BTreeMap<String, usize> = BTreeMap::new();
    for r in lib
        .refs
        .iter()
        .filter(|r| r.kind == RefKind::Expr && r.parts.len() > 1)
    {
        multi += 1;
        head_ok += usize::from(!matches!(lib.resolve_head(r), Resolution::Unresolved(_)));
        match lib.resolve(r) {
            Resolution::Unresolved(w) => *why.entry(format!("{w:?}")).or_default() += 1,
            Resolution::Builtin => {
                full_ok += 1;
                // 組み込みの型の属性（`x.start`）のはず。ここに属性以外が出たら取りこぼし
                *attrs
                    .entry(r.parts.last().cloned().unwrap_or_default())
                    .or_default() += 1;
            }
            _ => full_ok += 1,
        }
    }
    eprintln!("組み込みに解決した続きの最後の名前: {attrs:?}");
    eprintln!(
        "成分参照の続き（parts>1）: {multi} 件、先頭だけ解決 {head_ok}、続きまで解決 {full_ok}、未解決の内訳 {why:?}"
    );
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

fn names(v: &[globe_mode_resolve::Member]) -> Vec<&str> {
    let mut n: Vec<&str> = v.iter().map(|m| m.name.as_str()).collect();
    n.sort_unstable();
    n
}

/// S279: 書きかけの文書を一時的に足して `a.` の直下を引く。終わったらライブラリは元のまま
#[test]
fn members_of_a_broken_document_and_the_library_is_restored() {
    let mut lib = Library::from_sources([(
        "A.mo",
        "package A\n  connector Pin\n    Real v;\n    flow Real i;\n  end Pin;\n  model Base\n    Pin p;\n  protected\n    Real h;\n  end Base;\n  model R\n    extends Base;\n    Pin n;\n    model Inner\n    end Inner;\n  end R;\nend A;\n",
    )]);
    let (nc, nf) = (lib.classes.len(), lib.files.len());
    let text = "within A;\nmodel M\n  R r;\nequation\n  r.p.v = r.\n  Real = ;\n"; // 末尾の `end M;` も無い
    let at = text.find("r.\n").unwrap() as u32 + 2;
    let got = lib.with_source("M.mo", text, |l, f| {
        let c = l.class_at(f, at)?;
        assert_eq!(l.qualified_name(c), "A.M");
        let r = l.members_of(c, &["r".into()])?;
        let p = l.members_of(c, &["r".into(), "p".into()])?;
        Some((names(&r).join(","), names(&p).join(",")))
    });
    assert_eq!(got, Some(("Inner,n,p".into(), "i,v".into())));
    assert_eq!((lib.classes.len(), lib.files.len()), (nc, nf));
    assert_eq!(lib.class_by_name("A.M"), None);
}

/// 計測用（`#[ignore]`）: MSL でライブラリを作る時間・メモリと、`Resistor` の部品のメンバー
/// `GLOBE_MSL=<main>/data/msl cargo test -p globe-mode-resolve --release --test resolve -- --ignored --nocapture msl_members`
#[test]
#[ignore = "MSL が要る"]
fn msl_members_and_memory() {
    let rss = || {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS"))?
                    .split_whitespace()
                    .nth(1)?
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0)
    };
    let (r0, t0) = (rss(), std::time::Instant::now());
    let msl = msl_dir();
    let mut lib = Library::from_dir(&msl).unwrap();
    let (built, r1) = (t0.elapsed(), rss());
    lib.refs = Vec::new();
    lib.refs.shrink_to_fit();
    let r2 = rss();
    let text = "model M\n  Modelica.Electrical.Analog.Basic.Resistor r;\nequation\n  r.\n";
    let t1 = std::time::Instant::now();
    let m = lib.with_source("M.mo", text, |l, f| {
        let c = l.class_at(f, text.len() as u32 - 1)?; // `r.` の直後
        l.members_of(c, &["r".into()])
    });
    eprintln!(
        "build {built:?}, RSS {r0} -> {r1} -> {r2} (refs を捨てた後) kB; with_source+members {:?}",
        t1.elapsed()
    );
    let m = m.expect("Resistor のメンバー");
    let n = names(&m);
    eprintln!("{n:?}");
    for want in ["R", "p", "n", "v", "i", "T"] {
        assert!(n.contains(&want), "{want}");
    }
}
