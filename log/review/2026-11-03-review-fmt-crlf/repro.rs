// Run from the repository root; see review_en.md for rustc commands.
use globe_mode_fmt::{format, Options};

fn main() {
    let options = Options { insert_origin: false, format_html: false, ..Options::default() };
    let block_crlf = "model M\r\n  /* first\r\n  second */\r\n  Real x;\r\nend M;\r\n";
    let block_lf = block_crlf.replace("\r\n", "\n");
    let line_crlf = "model M\r\n  // first\r\n  // second\r\n  Real x;\r\nend M;\r\n";
    for (name, input) in [("block_crlf", block_crlf), ("block_lf", block_lf.as_str()), ("line_crlf", line_crlf)] {
        let result = format(input, &options);
        println!("{name}: {result:?}");
        match name {
            "block_crlf" => assert!(result.is_err()),
            _ => assert!(result.is_ok()),
        }
    }
}
