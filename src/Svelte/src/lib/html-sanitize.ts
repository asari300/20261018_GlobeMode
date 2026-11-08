// 説明パネルの HTML を洗う（S232）。Documentation の HTML は第三者のライブラリのもので信用しない。
// DOMParser（スクリプトを走らせず、画像も読まない不活性な文書）で読み、白リストのタグだけを素のデータの木に組み直す。
// 出力は文字列かタグ名つきの節だけで、HTML の文字列は作らない（呼び出し側は {@html} を使わず、要素と文字列として描く）。
// 属性は a の href（http・https・modelica だけ）以外は捨てる。img は alt の文字列に置き換える
// （`modelica://Pkg/Resources/…` の画像は読めない。表示は今回の範囲外）。

export type Safe = string | { tag: string; href?: string; children: Safe[] };

/** DOM のうち使う分だけ（node の試験が同じ形の木を渡せるように） */
export interface DomNode {
  nodeType: number;
  nodeName: string;
  nodeValue: string | null;
  childNodes: ArrayLike<DomNode>;
  getAttribute?(name: string): string | null;
}

const KEEP = new Set(["p", "br", "b", "i", "em", "strong", "sub", "sup", "ul", "ol", "li", "h1", "h2", "h3", "h4", "h5", "h6", "table", "thead", "tbody", "tr", "td", "th", "pre", "code", "a", "hr", "blockquote", "dl", "dt", "dd"]);
// 中身の文字列も見せない（コードや規則）。それ以外の知らないタグは外して中身だけ残す
const DROP = new Set(["script", "style", "noscript", "template", "iframe", "object", "embed", "svg", "math", "title", "head"]);
const SCHEMES = new Set(["http:", "https:", "modelica:"]);

function safeHref(h: string | null | undefined): string | undefined {
  try {
    const u = new URL(h ?? "");
    return SCHEMES.has(u.protocol) ? u.href : undefined;
  } catch {
    return undefined; // 相対や壊れた URL
  }
}

function walk(n: DomNode, out: Safe[]): void {
  if (n.nodeType === 3) {
    if (n.nodeValue) out.push(n.nodeValue);
    return;
  }
  if (n.nodeType !== 1) return; // コメントなど
  const tag = n.nodeName.toLowerCase();
  if (DROP.has(tag)) return;
  if (tag === "img") {
    const alt = n.getAttribute?.("alt");
    if (alt) out.push(alt);
    return;
  }
  const children: Safe[] = [];
  for (const c of Array.from(n.childNodes)) walk(c, children);
  const href = tag === "a" ? safeHref(n.getAttribute?.("href")) : undefined;
  if (KEEP.has(tag) && (tag !== "a" || href)) out.push({ tag, href, children });
  else out.push(...children);
}

/** html を洗って白リストの木にする。parse は試験が差し替える（既定はブラウザの DOMParser の body）。 */
export function sanitize(html: string, parse: (html: string) => DomNode = (h) => new DOMParser().parseFromString(h, "text/html").body): Safe[] {
  const out: Safe[] = [];
  for (const c of Array.from(parse(html).childNodes)) walk(c, out);
  return out;
}
