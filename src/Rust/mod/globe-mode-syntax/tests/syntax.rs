use globe_mode_syntax::{SyntaxKind, SyntaxKind::*, SyntaxNode, lex, parse};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn mo_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            mo_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
}

fn kinds(n: &SyntaxNode, k: SyntaxKind) -> usize {
    n.descendants().filter(|d| d.kind() == k).count()
}

fn roundtrip(text: &str) -> globe_mode_syntax::Parse {
    let toks: String = lex(text)
        .iter()
        .scan(0, |at, t| {
            let s = &text[*at..*at + t.len];
            *at += t.len;
            Some(s)
        })
        .collect();
    assert_eq!(toks, text, "lex");
    let p = parse(text);
    assert_eq!(p.syntax().to_string(), text, "tree");
    p
}

#[test]
fn lexer_kinds() {
    let k: Vec<_> = lex("model M // c\n  Real 'a b' = 1.5e-3 + .5; /* x */ \"s\\\"t\" a.*b <> := ")
        .iter()
        .map(|t| t.kind)
        .filter(|k| !k.is_trivia())
        .collect();
    assert_eq!(
        k,
        [
            KwModel, Ident, Ident, Ident, Eq, UReal, Plus, Dot, UInt, Semi, Str, Ident, DotStar,
            Ident, Ne, Assign
        ]
    );
    // 未終了の文字列・コメント・知らない文字・非 ASCII も全バイトが残る
    for s in [
        "\"abc",
        "/* abc",
        "'q",
        "a ¬ b",
        "日本語",
        "\"\\",
        "x\u{feff}y",
    ] {
        roundtrip(s);
    }
}

#[test]
fn structure_of_a_model() {
    let src = r#"within A.B;
encapsulated partial model M "desc"
  extends Base(x = 1) annotation(a = 1);
  import Modelica.Units.SI;
  import C = Modelica.Constants;
  import Modelica.Math.*;
  parameter Real k[2](each unit = "m") = {1, 2} "gain" annotation(Dialog(group = "g")), j = 3;
  Real x(start = 0) if k[1] > 0;
  replaceable model Inner = Foo constrainedby Bar;
  model Nested
    Real y;
  end Nested;
protected
  Real z = if x > 0 then 1 else 2;
equation
  der(x) = -k[1] * x;
  if x > 0 then
    z = 1;
  elseif x < -1 then
    z = 2;
  else
    z = 3;
  end if;
  for i in 1:2 loop
    connect(a[i], b[i]);
  end for;
  when x < 0 then
    reinit(x, 0);
  end when;
algorithm
  y := if x > 0 then 1 else 2;
  while y < 3 loop y := y + 1; end while;
end M;
type T = Real(unit = "m");
"#;
    let p = roundtrip(src);
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    let t = p.syntax();
    assert_eq!(kinds(&t, WithinClause), 1);
    assert_eq!(kinds(&t, ClassDef), 4);
    assert_eq!(kinds(&t, ShortBody), 2);
    assert_eq!(kinds(&t, ExtendsClause), 1);
    assert_eq!(kinds(&t, ImportClause), 3);
    assert_eq!(kinds(&t, ComponentClause), 4);
    assert_eq!(kinds(&t, Declaration), 5);
    assert_eq!(kinds(&t, Condition), 1);
    assert_eq!(kinds(&t, EquationSection), 1);
    assert_eq!(kinds(&t, AlgorithmSection), 1);
    // der / if(3 枝・中に 3 本) / for(中に 1) / when(中に 1)
    assert_eq!(kinds(&t, Equation), 1 + 1 + 3 + 1 + 1 + 1 + 1);
    assert_eq!(kinds(&t, Statement), 3);
}

#[test]
fn broken_inputs_keep_a_tree() {
    for s in [
        "model M Real x equation x = 1; end M;",
        "model M\n  Real x(\nequation\n  x = 1;\nend M;",
        "model M equation if x then y = 1; end M;",
        "model M equation x = (1; y = 2; end M;",
        "model M end;",
        "model",
        "end if; ) ] ; model",
        "model M model N end N; ",
        "x = 1;",
        "model M equation else y = 1; end if; end M;",
        "model M\n  Real x;\n  ???\n  Real y;\nend M;",
        "model M extends ; import ; Real ; end M;",
    ] {
        let p = roundtrip(s);
        assert!(!p.errors.is_empty(), "no error for {s:?}");
    }
    // 誤りの後ろの宣言は読めている
    let p = roundtrip("model M\n  Real x\nequation\n  x = 1;\nend M;");
    assert_eq!(kinds(&p.syntax(), EquationSection), 1);
    // 深い入れ子で溢れない
    let deep = "model M equation ".to_string() + &"if a then ".repeat(5000);
    roundtrip(&deep);
    roundtrip(&"model M ".repeat(5000));
}

#[test]
fn repo_fixtures() {
    let mut files = Vec::new();
    mo_files(&root().join("src/Modelica"), &mut files);
    assert!(files.len() > 50);
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let p = roundtrip(&text);
        assert!(
            p.errors.is_empty(),
            "{}: {:?}",
            f.display(),
            &p.errors[..p.errors.len().min(3)]
        );
        // 途中で切っても木は作れて往復する（簡易な fuzz）
        let step = (text.len() / 12).max(1);
        for cut in (0..text.len()).step_by(step) {
            if text.is_char_boundary(cut) {
                roundtrip(&text[..cut]);
            }
        }
    }
    // わざと誤ったファイル
    let bad = root().join("src/Rust/mod/globe-mode-frontend/tests/data");
    let mut files = Vec::new();
    mo_files(&bad, &mut files);
    let broken: Vec<_> = files
        .iter()
        .filter(|f| f.ends_with("SyntaxError.mo") || f.ends_with("Broken.mo"))
        .collect();
    assert!(!broken.is_empty());
    for f in broken {
        let p = roundtrip(&std::fs::read_to_string(f).unwrap());
        assert!(!p.errors.is_empty(), "{}", f.display());
        assert!(kinds(&p.syntax(), ClassDef) >= 1);
    }
}

/// MSL の全 `.mo`（`GLOBE_MSL` か `data/msl`。無ければ何もしない）。
/// `cargo test --release -p globe-mode-syntax -- --ignored --nocapture`
#[test]
#[ignore]
fn msl_all() {
    let dir = std::env::var_os("GLOBE_MSL").map_or_else(|| root().join("data/msl"), PathBuf::from);
    let mut files = Vec::new();
    mo_files(&dir, &mut files);
    if files.is_empty() {
        eprintln!("MSL が無いので省く: {}", dir.display());
        return;
    }
    let (mut ok, mut with_errors, mut bad_roundtrip, mut panics) = (0, 0, 0, 0);
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let r = std::panic::catch_unwind(|| {
            let p = parse(&text);
            (
                p.syntax().to_string() == text,
                p.errors
                    .first()
                    .map(|e| format!("{:?} {}", e.range, e.message)),
            )
        });
        match r {
            Err(_) => panics += 1,
            Ok((false, _)) => bad_roundtrip += 1,
            Ok((true, None)) => ok += 1,
            Ok((true, Some(e))) => {
                with_errors += 1;
                eprintln!("errors: {} {e}", f.display());
            }
        }
    }
    eprintln!(
        "files={} ok={ok} with_errors={with_errors} bad_roundtrip={bad_roundtrip} panics={panics}",
        files.len()
    );
    assert_eq!((bad_roundtrip, panics), (0, 0));
}
