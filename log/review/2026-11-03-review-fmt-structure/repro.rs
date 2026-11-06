extern crate globe_mode_fmt;
use globe_mode_fmt::{Options, format};

fn main() {
    let options = Options::default();
    let control = "package Base\n  replaceable model M\n    parameter Real x = 1;\n  end M;\nend Base;\npackage Derived\n  extends Base;\n  redeclare model extends M\n  end M;\nend Derived;\n";
    let modified = "package Base\n  replaceable model M\n    parameter Real x = 1;\n  end M;\nend Base;\npackage Derived\n  extends Base;\n  redeclare model extends M(x = 2)\n  end M;\nend Derived;\n";
    let invalid = "model M\n  Real x;\nequation\n  if true then\n    x = 1;\n  end for;\nend M;\n";
    let valid = invalid.replace("end for;", "end if;");
    for (label, input) in [
        ("class-extends-control", control),
        ("class-extends-modification", modified),
        ("block-control", &valid),
        ("block-mismatched-end", invalid),
    ] {
        let result = format(input, &options);
        println!("{label}: {result:?}");
        match label {
            "class-extends-control" | "block-control" => assert!(result.is_ok()),
            "class-extends-modification" => assert!(result.is_err()),
            "block-mismatched-end" => assert!(result.is_ok()),
            _ => unreachable!(),
        }
    }
}
