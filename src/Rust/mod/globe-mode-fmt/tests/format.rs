//! 整形器の検査: ユーザーの Before/After、冪等、トークン不変、壊れた入力。

use std::path::{Path, PathBuf};

use globe_mode_fmt::{LintOptions, Options, format, lint};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .unwrap()
}

fn mo_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            mo_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
}

#[test]
fn invalid_ascii_before_unicode_returns_error() {
    let error = format("!é", &Options::default()).unwrap_err();
    assert_eq!(error.offset, 0);
}

#[test]
fn wrapping_omitted_tuple_output_preserves_model() {
    let src = "function f\n  output Real a;\n  output Real b;\nalgorithm\n  a := 1;\n  b := 2;\nend f;\nmodel M\n  Real x;\nalgorithm\n  (, x) := f();\nend M;\n";
    let options = Options {
        width: 5,
        ..Options::default()
    };
    let out = format(src, &options).unwrap();
    assert_eq!(format(&out, &options).unwrap(), out);
    assert!(
        out.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .contains("(,x):=f();")
    );
}

#[test]
fn crlf_multiline_block_comment_and_string_format() {
    // S192 (issue #19): 内部の CRLF が \r\r\n にならない
    let o = Options {
        insert_origin: false,
        format_html: false,
        ..Options::default()
    };
    let src = "model M\r\n  /* first\r\n  second */\r\n  Real x \"a\r\nb\";\r\nend M;\r\n";
    let out = format(src, &o).unwrap();
    assert!(out.contains("/* first\r\n  second */") && out.contains("\"a\r\nb\""));
    assert!(!out.contains("\r\r"));
    assert_eq!(format(&out, &o).unwrap(), out);
}

#[test]
fn redeclare_model_extends_with_modification_formats() {
    // S192 (issue #20、S167 で修正済み): `redeclare model extends M(x = 2)`
    let src = "package Base\n  replaceable model M\n    parameter Real x = 1;\n  end M;\nend Base;\n\npackage Derived\n  extends Base;\n\n  redeclare model extends M(x = 2)\n  end M;\nend Derived;\n";
    let out = format(src, &Options::default()).unwrap();
    assert!(out.contains("redeclare model extends M(x = 2)"));
    assert_eq!(format(&out, &Options::default()).unwrap(), out);
}

#[test]
fn mismatched_block_end_is_rejected() {
    // S192 (issue #21): 開いた if を end for で閉じる入力は断る。end if なら通る
    let bad = "model M\n  Real x;\nequation\n  if true then\n    x = 1;\n  end for;\nend M;\n";
    let e = format(bad, &Options::default()).unwrap_err();
    assert!(e.message.contains("合わない"), "{e}");
    assert!(format(&bad.replace("end for;", "end if;"), &Options::default()).is_ok());
    let bad =
        "model M\n  Real x;\nequation\n  for i in 1:2 loop\n    x = 1;\n  end while;\nend M;\n";
    assert!(format(bad, &Options::default()).is_err());
}

#[test]
fn user_before_after() {
    let before = include_str!("data/objects_before.mo");
    let after = include_str!("data/objects_after.mo");
    assert_eq!(format(before, &Options::default()).unwrap(), after);
    assert_eq!(format(after, &Options::default()).unwrap(), after, "冪等");
}

#[test]
fn no_origin_insertion_when_off() {
    // 例の Rectangle は origin = {0, 0} を書いているので、外してから補わないことを見る
    let before = include_str!("data/objects_before.mo").replace("origin = {0, 0}, ", "");
    assert!(!before.contains("origin = {0, 0}"));
    let out = format(
        &before,
        &Options {
            insert_origin: false,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!out.contains("origin = {0, 0}"));
    assert!(out.contains("origin = {0, 40}"));
}

#[test]
fn validation_library_is_idempotent() {
    let mut files = Vec::new();
    mo_files(&repo().join("src/Modelica"), &mut files);
    assert!(files.len() > 50, "{}", files.len());
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let once =
            format(&src, &Options::default()).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let twice = format(&once, &Options::default()).unwrap();
        assert_eq!(once, twice, "{} が冪等でない", f.display());
        assert!(once.ends_with('\n') && !once.ends_with("\n\n"));
        assert!(
            once.lines().all(|l| l == l.trim_end()),
            "{} に行末の空白",
            f.display()
        );
    }
}

#[test]
fn broken_input_is_left_alone() {
    let src = std::fs::read_to_string(
        repo().join("src/Rust/mod/globe-mode-frontend/tests/data/SyntaxError.mo"),
    )
    .unwrap();
    assert!(format(&src, &Options::default()).is_err());
    assert!(format("model M\n  x = (1;\nend M;\n", &Options::default()).is_err());
}

#[test]
fn spacing_and_blank_lines() {
    let src = "within A.B;\nmodel M \"d\"\n  parameter Real k=-1 \"g\";\n  Real x(start=1,fixed=true);\n  Real y[3];\nequation\n  der(x)=-k*x+y[1]^2; // c\n  for i in 1:3 loop\n    y[i]=if x>0 then i else -i;\n  end for;\n\n\n  annotation (experiment(StopTime=1));\nend M;";
    let want = "within A.B;\n\nmodel M \"d\"\n\n  parameter Real k = -1 \"g\";\n  Real x(start = 1, fixed = true);\n  Real y[3];\n\nequation\n\n  der(x) = -k * x + y[1]^2; // c\n  for i in 1:3 loop\n    y[i] = if x > 0 then i else -i;\n  end for;\n\nannotation(experiment(StopTime = 1));\n\nend M;\n";
    assert_eq!(format(src, &Options::default()).unwrap(), want);
    assert_eq!(format(want, &Options::default()).unwrap(), want);
    // S061: equation / algorithm の前後の空行は切れる。切ると元からあった空行だけが残る
    let off = Options {
        blank_around_sections: false,
        ..Options::default()
    };
    let want_off = want.replace("\n\nequation\n\n", "\nequation\n");
    assert_eq!(format(src, &off).unwrap(), want_off);
    let alg = "model M\n  Real x;\nalgorithm\n  x := 1;\ninitial equation\n  x = 0;\nend M;\n";
    assert_eq!(
        format(alg, &Options::default()).unwrap(),
        "model M\n\n  Real x;\n\nalgorithm\n\n  x := 1;\n\ninitial equation\n\n  x = 0;\n\nend M;\n"
    );
}

#[test]
fn annotation_and_documentation_order() {
    let before = include_str!("data/annotation_order_before.mo");
    let after = include_str!("data/annotation_order_after.mo");
    assert_eq!(format(before, &Options::default()).unwrap(), after);
    assert_eq!(format(after, &Options::default()).unwrap(), after, "冪等");
    // lint: 並びがずれていれば annotation-order（annotation と Documentation で 1 件ずつ）、整形後は 0 件
    let rules = |src: &str| -> Vec<(usize, &'static str)> {
        lint(src, &LintOptions::default())
            .into_iter()
            .filter(|m| m.rule == "annotation-order")
            .map(|m| (m.line, m.rule))
            .collect()
    };
    assert_eq!(rules(before).len(), 2, "{:?}", rules(before));
    assert!(rules(after).is_empty());
}

#[test]
fn indent_four_and_long_line_wraps() {
    let src = "model M\n  Modelica.Blocks.Math.Gain gain(k = 2) annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));\nend M;\n";
    let out = format(
        src,
        &Options {
            indent: 4,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "model M\n\n    Modelica.Blocks.Math.Gain gain(k = 2) annotation(\n        Placement(transformation(extent = {{-10, -10}, {10, 10}}))\n    );\n\nend M;\n"
    );
}

#[test]
fn lint_rules() {
    let src = "model m\n  Real x;\nend m;\nmodel Ok \"d\"\n  Real s = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\nend Ok;\n";
    let msgs = lint(src, &LintOptions::default());
    let rules: Vec<_> = msgs.iter().map(|m| (m.rule, m.line)).collect();
    assert_eq!(
        rules,
        [
            ("class-name-pascal-case", 1),
            ("missing-description", 1),
            ("line-too-long", 5)
        ]
    );
    let only = lint(
        src,
        &LintOptions {
            disabled: vec!["line-too-long".into(), "missing-description".into()],
            ..LintOptions::default()
        },
    );
    assert_eq!(only.len(), 1);
}

/// S207: lint が本文の長さの 2 乗で遅くなっていた（診断ごとの行・桁の数え直し、長い行ごとの全行の探索）。
/// 絶対時間ではなく、本文を 2 倍にしたときの時間の比（3 回の最小値）が 3 未満であることを緩く確かめる（2 乗なら 4）。
#[test]
fn lint_time_grows_linearly() {
    let body = |n: usize| {
        let mut s = String::from("package P \"p\"\n");
        for i in 0..n {
            s += &format!(
                "  model m{i}\n    Real x; // {}\n  end m{i};\n",
                "c".repeat(90)
            );
        }
        s + "end P;\n"
    };
    let time = |src: &str| {
        (0..3)
            .map(|_| {
                let t = std::time::Instant::now();
                std::hint::black_box(lint(src, &LintOptions::default()));
                t.elapsed().as_secs_f64()
            })
            .fold(f64::MAX, f64::min)
    };
    let (small, large) = (body(4000), body(8000));
    let n = lint(&large, &LintOptions::default()).len();
    assert_eq!(n, 3 * 8000, "診断の数（クラス名・説明・行の長さ）");
    let ratio = time(&large) / time(&small);
    assert!(ratio < 3.0, "本文を 2 倍にして時間が {ratio:.2} 倍");
}

#[test]
fn documentation_html_is_indented_and_can_be_turned_off() {
    let src = "model M \"m\"\n  annotation(Documentation(info = \"<html><head></head><body><h1>The Infomation</h1><ul>List<li>One</li><li>Two</li><li>Tree.js</li></ul></body></html>\"));\nend M;\n";
    let want = "model M \"m\"\n\nannotation(\n  Documentation(\n    info = \"\n      <html><head></head><body>\n        <h1>The Infomation</h1>\n        <ul>List\n          <li>One</li>\n          <li>Two</li>\n          <li>Tree.js</li>\n        </ul>\n      </body></html>\n    \"\n  )\n);\n\nend M;\n";
    assert_eq!(format(src, &Options::default()).unwrap(), want);
    assert_eq!(format(want, &Options::default()).unwrap(), want, "冪等");
    let crlf = src.replace('\n', "\r\n");
    assert_eq!(
        format(&crlf, &Options::default()).unwrap(),
        want.replace('\n', "\r\n")
    );
    let off = Options {
        format_html: false,
        ..Options::default()
    };
    assert!(
        format(src, &off)
            .unwrap()
            .contains("info = \"<html><head></head><body><h1>")
    );
    // lint は HTML を見ない（整形の前後で line-too-long 以外は同じ）
    let rules = |s: &str| -> Vec<&str> {
        lint(s, &LintOptions::default())
            .into_iter()
            .map(|m| m.rule)
            .filter(|r| *r != "line-too-long")
            .collect()
    };
    assert_eq!(rules(src), rules(want));
}

// S211（#24）: 図形プリミティブの名前でも、annotation の外の呼び出しは並べ替えず origin も足さない
#[test]
fn primitive_names_outside_annotation_are_left_alone() {
    let src = "model M\n  extends Pkg.Line(R = 1, L = 2);\n  model N = Lines.Line(R = 1, L = 2);\n  Real x = Text(b = 2, a = 1);\n  Real y = Documentation(revisions = 1, info = 2);\nend M;\n";
    let out = format(src, &Options::default()).unwrap();
    assert!(!out.contains("origin"), "{out}");
    assert!(out.contains("Line(R = 1, L = 2)"), "{out}");
    assert!(out.contains("Text(b = 2, a = 1)"), "{out}");
    assert!(
        out.contains("Documentation(revisions = 1, info = 2)"),
        "{out}"
    );
    // annotation の中は今までどおり
    let out = format(
        "model M\n  annotation(Icon(graphics = {Line(color = {0, 0, 0}, points = {{0, 0}, {1, 1}})}));\nend M;\n",
        &Options::default(),
    )
    .unwrap();
    assert!(
        out.contains("origin = {0, 0}") && out.contains("points"),
        "{out}"
    );
}

// S211（#28）: 深い入れ子はスタックを溢れさせず誤りを返す（100,000 段は 8 MB のスタックでも落ちる）
#[test]
fn deep_nesting_is_an_error_not_a_stack_overflow() {
    for n in [100_000, 257] {
        let src = format!(
            "model A Real x={}1{}; end A;\n",
            "(".repeat(n),
            ")".repeat(n)
        );
        let e = format(&src, &Options::default()).unwrap_err();
        assert!(e.message.contains("深すぎる"), "{e}");
        assert!(lint(&src, &LintOptions::default()).len() < 100);
    }
    let src = format!(
        "model A Real x={}1{}; end A;\n",
        "(".repeat(256),
        ")".repeat(256)
    );
    assert!(format(&src, &Options::default()).is_ok());
}

// S211（#32）: UTF-8 の BOM は読み飛ばし、出力には残す
#[test]
fn bom_is_accepted_and_kept() {
    let src = "\u{feff}model M\n  Real x(start=1);\nend M;\n";
    let out = format(src, &Options::default()).unwrap();
    assert_eq!(out, "\u{feff}model M\n\n  Real x(start = 1);\n\nend M;\n");
    assert_eq!(format(&out, &Options::default()).unwrap(), out);
    let l = lint(
        "\u{feff}model m\n  Real x;\nend m;\n",
        &LintOptions::default(),
    );
    assert!(
        l.iter()
            .any(|m| m.rule == "class-name-pascal-case" && m.line == 1 && m.column == 7),
        "{l:?}"
    );
}
