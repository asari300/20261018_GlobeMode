//! `Documentation(…)` の文字列の中の HTML を、改行とインデントで整える（S062）。
//!
//! 変えるのは空白だけ。ブロックの要素は 1 行ずつ、中身は 1 段下げる。中身が文字と
//! インラインの要素だけなら 1 行に書く（`<h1>The Information</h1>`）。`<html><head>…</head><body>` は
//! 1 行にまとめ、body の中身を 1 段下げる。`<pre>` などの空白に意味のある要素や、
//! 組み立て直して空白以外が変わるものは整えない（`None`）。

/// 空白に意味がある要素。含んでいれば整えない
const VERBATIM: &[&str] = &["<pre", "<textarea", "<script", "<style"];
/// 中身を閉じない要素
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "wbr",
];
/// 文の中に置く要素（行を分けない）
const INLINE: &[&str] = &[
    "a", "abbr", "b", "br", "cite", "code", "em", "font", "i", "img", "kbd", "q", "s", "small",
    "span", "strong", "sub", "sup", "tt", "u", "var",
];

/// 入れ子の深さの上限。これを超えたら整えない
const MAX_DEPTH: usize = 256;

#[derive(Debug)]
enum H<'a> {
    Text(&'a str),
    /// コメント・`<!DOCTYPE>`・閉じの合わないタグ
    Raw(&'a str),
    El {
        name: String,
        open: &'a str,
        children: Vec<H<'a>>,
        close: &'a str,
    },
}

/// 文字列の字句（`"` を含む）の中身を整えた行（段の深さ, 行）。HTML でなければ None。
pub fn pretty(token: &str) -> Option<Vec<(usize, String)>> {
    let body = token.strip_prefix('"')?.strip_suffix('"')?;
    let lower = body.to_ascii_lowercase();
    if !body.contains('<') || VERBATIM.iter().any(|v| lower.contains(v)) {
        return None;
    }
    let tree = parse(body)?;
    let mut out = Vec::new();
    block(&tree, 0, &mut out);
    let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let joined: String = out.iter().map(|(_, l)| l.as_str()).collect();
    (squash(&joined) == squash(body) && !out.is_empty()).then_some(out)
}

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches(['<', '/'])
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn parse(s: &str) -> Option<Vec<H<'_>>> {
    // 開いている要素: (名前, 開きタグ, 子)
    let mut stack: Vec<(String, &str, Vec<H<'_>>)> = vec![(String::new(), "", Vec::new())];
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with('<') {
            let end = i + s[i..].find('>')? + 1;
            let tag = &s[i..end];
            let name = tag_name(tag);
            if tag.starts_with("<!") || tag.starts_with("<?") || name.is_empty() {
                stack.last_mut()?.2.push(H::Raw(tag));
            } else if tag.starts_with("</") {
                let Some(k) = stack.iter().rposition(|(n, _, _)| *n == name) else {
                    stack.last_mut()?.2.push(H::Raw(tag));
                    i = end;
                    continue;
                };
                if k == 0 {
                    return None;
                }
                // 閉じていない内側の要素は、閉じタグなしで閉じる
                while stack.len() > k + 1 {
                    close(&mut stack, "");
                }
                close(&mut stack, tag);
            } else if VOID.contains(&name.as_str()) || tag.ends_with("/>") {
                stack.last_mut()?.2.push(H::El {
                    name,
                    open: tag,
                    children: Vec::new(),
                    close: "",
                });
            } else {
                // 深さの上限（#54）。整えない（None）だけで、再帰が 2 MiB のスタックを溢れさせない
                if stack.len() > MAX_DEPTH {
                    return None;
                }
                stack.push((name, tag, Vec::new()));
            }
            i = end;
        } else {
            let end = s[i..].find('<').map_or(s.len(), |k| i + k);
            stack.last_mut()?.2.push(H::Text(&s[i..end]));
            i = end;
        }
    }
    while stack.len() > 1 {
        close(&mut stack, "");
    }
    stack.pop().map(|(_, _, c)| c)
}

fn close<'a>(stack: &mut Vec<(String, &'a str, Vec<H<'a>>)>, tag: &'a str) {
    let (name, open, children) = stack.pop().expect("根より上の要素");
    stack.last_mut().expect("根").2.push(H::El {
        name,
        open,
        children,
        close: tag,
    });
}

fn is_inline(h: &H<'_>) -> bool {
    match h {
        H::Text(_) => true,
        H::Raw(_) => false,
        H::El { name, children, .. } => {
            INLINE.contains(&name.as_str()) && children.iter().all(is_inline)
        }
    }
}

fn is_blank(h: &H<'_>) -> bool {
    matches!(h, H::Text(t) if t.trim().is_empty())
}

/// 1 行に並べた字面。文字の中の空白の並びは 1 つに畳む。
fn flat(hs: &[H<'_>]) -> String {
    let mut out = String::new();
    for h in hs {
        match h {
            H::Text(t) => {
                let mut prev_space = out.ends_with(' ');
                for c in t.chars() {
                    if c.is_whitespace() {
                        if !prev_space {
                            out.push(' ');
                        }
                        prev_space = true;
                    } else {
                        out.push(c);
                        prev_space = false;
                    }
                }
            }
            H::Raw(r) => out.push_str(r),
            H::El {
                open,
                children,
                close,
                ..
            } => {
                out.push_str(open);
                out.push_str(&flat(children));
                out.push_str(close);
            }
        }
    }
    out
}

/// 子の並びを行にする。続くインラインの並びは 1 行、ブロックの要素は [`element`]。
fn block(hs: &[H<'_>], depth: usize, out: &mut Vec<(usize, String)>) {
    let mut k = 0;
    while k < hs.len() {
        if is_inline(&hs[k]) {
            let start = k;
            while k < hs.len() && is_inline(&hs[k]) {
                k += 1;
            }
            let line = flat(&hs[start..k]);
            if !line.trim().is_empty() {
                out.push((depth, line.trim().to_string()));
            }
        } else {
            element(&hs[k], depth, out);
            k += 1;
        }
    }
}

fn element(h: &H<'_>, depth: usize, out: &mut Vec<(usize, String)>) {
    let H::El {
        name,
        open,
        children,
        close,
    } = h
    else {
        out.push((depth, flat(std::slice::from_ref(h))));
        return;
    };
    if children.iter().all(is_inline) {
        out.push((depth, format!("{open}{}{close}", flat(children).trim())));
        return;
    }
    // <html><head>…</head><body> を 1 行に、body の中身を 1 段下げる
    let kids: Vec<&H<'_>> = children.iter().filter(|c| !is_blank(c)).collect();
    if name == "html" {
        let (head, body) = match kids.as_slice() {
            [h @ H::El { name: hn, .. }, b @ H::El { name: bn, .. }]
                if hn == "head" && bn == "body" =>
            {
                (Some(*h), *b)
            }
            [b @ H::El { name: bn, .. }] if bn == "body" => (None, *b),
            _ => (None, h),
        };
        if let H::El {
            open: bo,
            children: bc,
            close: bcl,
            name: bn,
        } = body
            && bn == "body"
        {
            let head = head.map_or(String::new(), |h| flat(std::slice::from_ref(h)));
            out.push((depth, format!("{open}{head}{bo}")));
            block(bc, depth + 1, out);
            out.push((depth, format!("{bcl}{close}")));
            return;
        }
    }
    // 先頭の文字は開きタグの行に続ける（`<ul>List`）
    let lead = children.iter().take_while(|c| is_inline(c)).count();
    out.push((
        depth,
        format!("{open}{}", flat(&children[..lead]).trim_end()),
    ));
    block(&children[lead..], depth + 1, out);
    if !close.is_empty() {
        out.push((depth, close.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::pretty;

    fn lines(s: &str) -> Vec<String> {
        pretty(s)
            .unwrap()
            .into_iter()
            .map(|(d, l)| format!("{}{l}", "  ".repeat(d)))
            .collect()
    }

    #[test]
    fn user_example() {
        let s = "\"<html><head></head><body><h1>The Infomation</h1><ul>List<li>One</li><li>Two</li><li>Tree.js</li></ul></body></html>\"";
        let want = [
            "<html><head></head><body>",
            "  <h1>The Infomation</h1>",
            "  <ul>List",
            "    <li>One</li>",
            "    <li>Two</li>",
            "    <li>Tree.js</li>",
            "  </ul>",
            "</body></html>",
        ];
        assert_eq!(lines(s), want);
        assert_eq!(
            lines("\"<html><head></head><body>the Header</body></html>\""),
            [
                "<html><head></head><body>",
                "  the Header",
                "</body></html>"
            ]
        );
        // 整えた結果をもう一度整えても同じ
        let again = format!("\"\n{}\n\"", want.join("\n"));
        assert_eq!(lines(&again), want);
    }

    #[test]
    fn very_deep_nesting_is_left_alone_not_a_stack_overflow() {
        // 1 MiB のスタックの上で回す。上限が無いと再帰（block・flat・drop）が溢れて abort する
        std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(|| {
                let deep = format!("\"{}x{}\"", "<div>".repeat(10_000), "</div>".repeat(10_000));
                assert_eq!(pretty(&deep), None);
                let ok = format!("\"{}x{}\"", "<div>".repeat(200), "</div>".repeat(200));
                assert!(pretty(&ok).is_some());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn leaves_non_html_and_verbatim_alone() {
        assert_eq!(pretty("\"plain text\""), None);
        assert_eq!(pretty("\"<pre>a\n  b</pre>\""), None);
        assert_eq!(pretty("\"a < b\""), None);
        // インラインの要素は行を分けない。閉じの無い <p> も読める
        assert_eq!(
            lines("\"<p>Hello <b>world</b>!<p>Next\""),
            ["<p>Hello <b>world</b>!", "  <p>Next"]
        );
    }
}
