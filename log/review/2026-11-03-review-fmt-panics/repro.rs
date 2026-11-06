use globe_mode_fmt::{format, Options};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("unicode") => {
            let error = format("!é", &Options::default()).unwrap_err();
            assert_eq!(error.offset, 0);
            println!("unicode: error at byte 0 (no panic)");
        }
        Some("tuple") => {
            let src = "function f\n  output Real a;\n  output Real b;\nalgorithm\n  a := 1;\n  b := 2;\nend f;\nmodel M\n  Real x;\nalgorithm\n  (, x) := f();\nend M;\n";
            let options = Options { width: 5, ..Options::default() };
            let out = format(src, &options).unwrap();
            assert_eq!(format(&out, &options).unwrap(), out);
            assert!(out.chars().filter(|c| !c.is_whitespace()).collect::<String>().contains("(,x):=f();"));
            println!("tuple: formatted and idempotent (no panic)");
        }
        _ => panic!("usage: repro unicode|tuple"),
    }
}
