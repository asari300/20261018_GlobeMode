#!/usr/bin/env python3
"""git の差分を LM Studio のローカルモデルに渡し、副レビューの指摘を Markdown で出す。

  python3 -I .claude/skills/local-review/lmstudio_review.py --check          # 接続と読み込みの確認
  python3 -I .claude/skills/local-review/lmstudio_review.py --load 12288     # ctx を指定して読み込む
  python3 -I .claude/skills/local-review/lmstudio_review.py --dry-run        # 分割の計画と所要時間の見積もりだけ
  python3 -I .claude/skills/local-review/lmstudio_review.py                  # main との差分をレビュー
  python3 -I .claude/skills/local-review/lmstudio_review.py --range HEAD~1..HEAD --out review.md

設定（引数が優先）:
  LMSTUDIO_BASE_URL   既定 http://localhost:1234（WSL2 の mirrored モードなら Windows 側に届く）
  LMSTUDIO_MODEL      既定 prism-ml/bonsai-27b
  LMSTUDIO_API_TOKEN  LM Studio の Require Authentication を有効にしたときのトークン

方針:
  - 標準ライブラリだけで書く（依存を足さない。doc/licenses.md を変えずに済む）。
  - モデルが読み込まれていなければ止まる。JIT 読み込みに任せると、既定のコンテキスト長
    （最大値のことがある）で読み込まれ、小さい PC ではメモリが足りなくなるため。
  - 指摘は助言にすぎない。採否は Claude がコードを読んで決める（SKILL.md）。

終了コード: 0 成功 / 1 その他の失敗 / 2 サーバに届かない・モデルが読み込まれていない。
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import math
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_BASE_URL = "http://localhost:1234"
DEFAULT_MODEL = "prism-ml/bonsai-27b"
# LM Studio のモデル一覧（/api/v0/models、/api/v1/models）は、眠っていたサービスを起こすときや
# ディスクを走査するときに 14〜63 秒かかった（S005 で実測。/v1/models は 0.1 秒以内）。
STATUS_TIMEOUT = 120

# 送らないファイル。生成物・ロックファイル・取得物・ログ・秘密の入りうるもの。
EXCLUDE_GLOBS = (
    "*.lock", "Cargo.lock", "uv.lock", "mise.lock", "package-lock.json", "pnpm-lock.yaml",
    "data/*", "log/*", "target/*", ".venv/*", "node_modules/*", "*/node_modules/*",
    ".env", ".env.*", "*.pem", "*.key", "id_rsa*", "*credentials*", "*secret*",
    "*.png", "*.jpg", "*.jpeg", "*.gif", "*.ico", "*.pdf", "*.gz", "*.zip", "*.min.js",
)

SYSTEM_PROMPT = """You are a meticulous senior code reviewer giving a second opinion.
Review ONLY the diff chunk below. Each diff line is prefixed with its line number in the NEW file
(blank for deleted lines) and a marker: '+' added, '-' deleted, ' ' unchanged context.

Report concrete defects only: bugs, logic errors, panics or unwraps on fallible input, off-by-one,
wrong error handling, resource leaks, security problems, race conditions, broken links or commands
in documentation, and changed behaviour without tests. Do not comment on formatting that
rustfmt / ruff / markdownlint handle. Do not guess about code that is not shown.

Output format, one finding per line, at most {max_findings} findings, most severe first:
<path>:<line> — <high|medium|low> — <problem> — <suggested fix>
If there is nothing significant, reply with exactly: NO_FINDINGS

Project rules: Rust (cargo clippy -D warnings, no unwrap on user input), Python standard library with
type hints, documents in Japanese. Dependencies must be MIT/Apache-2.0 compatible; GPL, AGPL and LGPL
are forbidden."""

FINDING = re.compile(
    r"^\s*(?:[-*]|\d+[.)])?\s*`?(?P<path>[^\s`:]+):(?P<line>\d+)`?\s*[—–-]+\s*"
    r"\**(?P<sev>high|medium|low)\**\s*[—–-]+\s*(?P<body>.+)$",
    re.I,
)
DIFF_HEADER = re.compile(r"^diff --git a/(?P<a>.+?) b/(?P<b>.+)$", re.M)
HUNK = re.compile(r"^@@ -(?P<ostart>\d+)(?:,\d+)? \+(?P<start>\d+)(?:,\d+)? @@")


# --------------------------------------------------------------------------- 設定と HTTP

@dataclass
class Config:
    base_url: str
    model: str
    token: str | None
    timeout: float


class ServerError(RuntimeError):
    """サーバに届かない、またはモデルが使えない。終了コード 2。"""


def api(cfg: Config, path: str, body: dict | None = None, timeout: float | None = None) -> dict:
    url = cfg.base_url.rstrip("/") + path
    data = None if body is None else json.dumps(body).encode()
    headers = {"Content-Type": "application/json"}
    if cfg.token:
        headers["Authorization"] = f"Bearer {cfg.token}"
    req = urllib.request.Request(url, data=data, headers=headers, method="GET" if body is None else "POST")
    try:
        with urllib.request.urlopen(req, timeout=timeout or cfg.timeout) as res:
            return json.load(res)
    except urllib.error.HTTPError as e:
        detail = e.read().decode(errors="replace")[:500]
        raise ServerError(f"{url} が HTTP {e.code} を返した: {detail}") from e
    except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
        raise ServerError(f"{url} に届かない: {e}") from e


def model_info(cfg: Config) -> dict | None:
    """LM Studio の /api/v0/models から対象モデルの状態を取る。無ければ None。"""
    for m in api(cfg, "/api/v0/models", timeout=STATUS_TIMEOUT).get("data", []):
        if m.get("id") == cfg.model:
            return m
    return None


def load_model(cfg: Config, context_length: int) -> dict:
    """REST API で読み込む。Flash Attention を有効にし、最終的な設定を返させる。"""
    body = {"model": cfg.model, "context_length": context_length, "flash_attention": True, "echo_load_config": True}
    return api(cfg, "/api/v1/models/load", body, timeout=900)


# --------------------------------------------------------------------------- 差分

@dataclass
class FileDiff:
    path: str
    text: str

    @property
    def binary(self) -> bool:
        return "\nBinary files " in self.text or "\nGIT binary patch" in self.text


def run_git(args: list[str], cwd: Path, ok_codes: tuple[int, ...] = (0,)) -> str:
    res = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if res.returncode not in ok_codes:
        raise RuntimeError(f"git {' '.join(args)} が失敗した: {res.stderr.strip()}")
    return res.stdout


def collect_diff(root: Path, base: str, rng: str | None, staged: bool, untracked: bool) -> str:
    if rng:
        return run_git(["diff", "--no-color", "--no-ext-diff", rng], root)
    if staged:
        return run_git(["diff", "--no-color", "--no-ext-diff", "--cached"], root)
    merge_base = run_git(["merge-base", base, "HEAD"], root).strip()
    text = run_git(["diff", "--no-color", "--no-ext-diff", merge_base], root)
    if untracked:
        for path in run_git(["ls-files", "--others", "--exclude-standard", "-z"], root).split("\0"):
            if path:
                # 差分があると終了コード 1 になるのが git diff --no-index の仕様。
                text += run_git(["diff", "--no-color", "--no-index", "--", "/dev/null", path], root, ok_codes=(0, 1))
    return text


def split_files(diff: str) -> list[FileDiff]:
    heads = list(DIFF_HEADER.finditer(diff))
    files = []
    for i, m in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(diff)
        files.append(FileDiff(path=m.group("b"), text=diff[m.start():end]))
    return files


def excluded(path: str) -> bool:
    name = path.rsplit("/", 1)[-1]
    return any(fnmatch.fnmatch(path, g) or fnmatch.fnmatch(name, g) for g in EXCLUDE_GLOBS)


def annotate(text: str) -> str:
    """差分の各行に、新しいファイルでの行番号を付ける。小さいモデルは @@ から行番号を数えられないため。"""
    out: list[str] = []
    line_no: int | None = None
    for raw in text.splitlines():
        m = HUNK.match(raw)
        if m:
            line_no = int(m.group("start"))
            out.append(raw)
        elif line_no is None or raw.startswith("\\"):
            out.append(raw)                       # ファイルの見出し、"\ No newline at end of file"
        elif raw.startswith("-"):
            out.append(f"{'':>5} - {raw[1:]}")
        else:
            sign = "+" if raw.startswith("+") else " "
            out.append(f"{line_no:>5} {sign} {raw[1:]}")
            line_no += 1
    return "\n".join(out) + "\n"


def estimate_tokens(text: str) -> int:
    """トークン数を多めに見積もる。ASCII は 3 文字で 1、それ以外（日本語など）は 1 文字で 1。

    Bonsai-27B（Qwen 系のトークナイザ）で Rust のソースを測ると 3.3 文字で 1 トークンだった（S005）。
    """
    ascii_chars = sum(1 for c in text if ord(c) < 128)
    return math.ceil(ascii_chars / 3) + (len(text) - ascii_chars)


# --------------------------------------------------------------------------- 分割

@dataclass
class Chunk:
    files: list[str] = field(default_factory=list)
    text: str = ""
    tokens: int = 0
    truncated: bool = False


def split_hunks(fd: FileDiff) -> tuple[str, list[str]]:
    """ファイルの見出しと、@@ ごとの塊に分ける。"""
    lines = fd.text.splitlines(keepends=True)
    head: list[str] = []
    hunks: list[list[str]] = []
    for ln in lines:
        if ln.startswith("@@"):
            hunks.append([ln])
        elif hunks:
            hunks[-1].append(ln)
        else:
            head.append(ln)
    return "".join(head), ["".join(h) for h in hunks]


def split_long_hunk(head: str, hunk: str, budget: int) -> list[tuple[str, bool]]:
    """1 つの @@ が予算を超えるとき、行番号を引き継いだ続きの @@ に分ける。

    新しいファイル（全体が 1 つの @@）でもレビューが切れないようにするため。
    1 行だけで予算を超える場合に限り、その行を切り詰める。
    """
    lines = hunk.splitlines(keepends=True)
    m = HUNK.match(lines[0])
    old_no, new_no = (int(m.group("ostart")), int(m.group("start"))) if m else (1, 1)
    fixed = estimate_tokens(head) + 16
    parts: list[tuple[str, bool]] = []
    cur: list[str] = []
    used = 0
    header = lines[0]
    truncated = False
    for ln in lines[1:]:
        cost = estimate_tokens(ln) + 3                 # 行番号と印の分
        line_cut = fixed + cost > budget
        if line_cut:
            ln = ln[: max(0, (budget - fixed - 40) * 3)] + " ... [truncated: line too long]\n"
            cost = estimate_tokens(ln) + 3
        if cur and fixed + used + cost > budget:
            parts.append((annotate(head + header + "".join(cur)), truncated))
            cur, used, truncated = [], 0, False
            header = f"@@ -{old_no} +{new_no} @@ (continued)\n"
        cur.append(ln)
        used += cost
        truncated |= line_cut
        if ln.startswith("-"):
            old_no += 1
        elif ln.startswith("+"):
            new_no += 1
        elif not ln.startswith("\\"):
            old_no += 1
            new_no += 1
    if cur:
        parts.append((annotate(head + header + "".join(cur)), truncated))
    return parts


def pieces_of(fd: FileDiff, budget: int) -> list[tuple[str, bool]]:
    """1 ファイルの差分を、予算に収まる断片の列にする。(注釈付きの本文, 切り詰めたか)。"""
    whole = annotate(fd.text)
    if estimate_tokens(whole) <= budget:
        return [(whole, False)]
    head, hunks = split_hunks(fd)
    out: list[tuple[str, bool]] = []
    cur = ""
    for h in hunks:
        if estimate_tokens(annotate(head + h)) > budget:
            if cur:
                out.append((annotate(head + cur), False))
                cur = ""
            out += split_long_hunk(head, h, budget)
        elif cur and estimate_tokens(annotate(head + cur + h)) > budget:
            out.append((annotate(head + cur), False))
            cur = h
        else:
            cur += h
    if cur:
        out.append((annotate(head + cur), False))
    return out


def make_chunks(files: list[FileDiff], budget: int) -> list[Chunk]:
    chunks: list[Chunk] = []
    cur = Chunk()
    for fd in files:
        for text, truncated in pieces_of(fd, budget):
            t = estimate_tokens(text)
            if cur.files and cur.tokens + t > budget:
                chunks.append(cur)
                cur = Chunk()
            if fd.path not in cur.files:
                cur.files.append(fd.path)
            cur.text += text
            cur.tokens += t
            cur.truncated |= truncated
    if cur.files:
        chunks.append(cur)
    return chunks


# --------------------------------------------------------------------------- レビュー

@dataclass
class Finding:
    path: str
    line: int
    severity: str
    body: str


def parse_findings(text: str) -> tuple[list[Finding], list[str]]:
    """モデルの出力から指摘を取り出す。形式に合わない行は (その他) として返す。"""
    found: list[Finding] = []
    other: list[str] = []
    for ln in text.splitlines():
        s = ln.strip()
        if not s or s == "NO_FINDINGS" or s.startswith("```"):
            continue
        m = FINDING.match(s)
        if m:
            found.append(Finding(m.group("path"), int(m.group("line")), m.group("sev").lower(), m.group("body").strip()))
        else:
            other.append(s)
    return found, other


def review_chunk(cfg: Config, chunk: Chunk, idx: int, total: int, args: argparse.Namespace) -> dict:
    system = SYSTEM_PROMPT.format(max_findings=args.max_findings)
    user = f"Diff chunk {idx}/{total} (files: {', '.join(chunk.files)}):\n\n{chunk.text}"
    body = {
        "model": cfg.model,
        "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
        "temperature": args.temperature,
        "top_p": 0.9,
        "top_k": 20,
        "max_tokens": args.max_tokens,
        "stream": False,
        "ttl": args.ttl,
    }
    if not args.think:
        # Bonsai-27B（qwen35）は推論が既定で有効。LM Studio では reasoning_effort "none" で切れる。
        # chat_template_kwargs.enable_thinking は無視された（S005 で確認）。
        body["reasoning_effort"] = "none"
    start = time.monotonic()
    res = api(cfg, "/api/v0/chat/completions", body)
    elapsed = time.monotonic() - start
    msg = res["choices"][0]["message"]
    return {
        "content": (msg.get("content") or "").strip(),
        "usage": res.get("usage", {}),
        "stats": res.get("stats", {}),
        "elapsed": elapsed,
        "finish": res["choices"][0].get("finish_reason"),
    }


def eta_seconds(tokens: int, args: argparse.Namespace) -> float:
    return tokens / args.pp_tps + args.max_tokens / args.gen_tps


def fmt_secs(s: float) -> str:
    return f"{s / 60:.1f} 分" if s >= 90 else f"{s:.0f} 秒"


# --------------------------------------------------------------------------- 入口

def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="LM Studio のローカルモデルで git の差分を副レビューする")
    p.add_argument("--base-url", default=os.environ.get("LMSTUDIO_BASE_URL", DEFAULT_BASE_URL))
    p.add_argument("--model", default=os.environ.get("LMSTUDIO_MODEL", DEFAULT_MODEL))
    p.add_argument("--check", action="store_true", help="接続とモデルの状態を表示して終わる")
    p.add_argument("--load", type=int, metavar="CTX", help="このコンテキスト長で読み込んで終わる")
    p.add_argument("--ctx", type=int, help="読み込みの確認（数十秒かかることがある）を省き、この ctx とみなす。"
                   "未読み込みだと JIT で既定の ctx に読み込まれるので、読み込み済みと分かっているときだけ使う")
    p.add_argument("--base", default="main", help="比べる相手のブランチ（merge-base を取る）。既定 main")
    p.add_argument("--range", help="git diff に渡す範囲（例 HEAD~1..HEAD）。--base より優先")
    p.add_argument("--staged", action="store_true", help="ステージした差分だけ")
    p.add_argument("--no-untracked", dest="untracked", action="store_false", help="未追跡のファイルを含めない")
    p.add_argument("--diff-file", help="差分をファイルから読む（- で標準入力）")
    p.add_argument("--path", action="append", default=[], help="このパス（glob）に合うファイルだけ。複数可")
    p.add_argument("--max-tokens", type=int, default=768, help="1 回の出力の上限。既定 768")
    p.add_argument("--max-findings", type=int, default=10)
    p.add_argument("--chunk-tokens", type=int, default=6000,
                   help="1 塊の差分の上限（見積もりのトークン数）。小さいほど 1 回が早く返り、指摘も絞られる。既定 6000")
    p.add_argument("--max-chunks", type=int, default=3, help="送る塊の上限。超えた分は送らずに一覧に出す。既定 3")
    p.add_argument("--temperature", type=float, default=0.4)
    p.add_argument("--think", action="store_true", help="推論（thinking）を切らない。遅くなる")
    p.add_argument("--ttl", type=int, default=1800, help="使われなくなってから解放するまでの秒数。既定 1800")
    p.add_argument("--timeout", type=float, default=3600, help="1 回のリクエストの待ち時間（秒）。既定 3600")
    p.add_argument("--pp-tps", type=float, default=8.0, help="見積もり用。prompt 処理の tok/s（S005 の実測 8）")
    p.add_argument("--gen-tps", type=float, default=2.0, help="見積もり用。生成の tok/s（S005 の実測 2）")
    p.add_argument("--dry-run", action="store_true", help="分割の計画と見積もりだけを出す（モデルに送らない）")
    p.add_argument("--out", help="レポートの書き出し先（既定は標準出力だけ）")
    return p.parse_args(argv)


def do_check(cfg: Config) -> int:
    info = model_info(cfg)
    loaded = [m["id"] for m in api(cfg, "/api/v0/models", timeout=STATUS_TIMEOUT).get("data", []) if m.get("state") == "loaded"]
    print(f"サーバ: {cfg.base_url} に届いた")
    print(f"読み込み中のモデル: {', '.join(loaded) or 'なし'}")
    if info is None:
        print(f"NG: {cfg.model} がダウンロード済みの一覧に無い")
        return 2
    print(f"{cfg.model}: state={info.get('state')} arch={info.get('arch')} quant={info.get('quantization')} "
          f"ctx={info.get('loaded_context_length', '-')} / max {info.get('max_context_length')}")
    if info.get("state") != "loaded":
        print(f"NG: 読み込まれていない。`--load 12288` か `lms load {cfg.model} -c 12288 --parallel 1` で読み込む")
        return 2
    return 0


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    cfg = Config(args.base_url, args.model, os.environ.get("LMSTUDIO_API_TOKEN") or None, args.timeout)
    try:
        if args.check:
            return do_check(cfg)
        if args.load:
            res = load_model(cfg, args.load)
            print(json.dumps(res, ensure_ascii=False, indent=2))
            return 0

        if args.diff_file:
            diff = sys.stdin.read() if args.diff_file == "-" else Path(args.diff_file).read_text(encoding="utf-8")
        else:
            root = Path(run_git(["rev-parse", "--show-toplevel"], Path.cwd()).strip())
            diff = collect_diff(root, args.base, args.range, args.staged, args.untracked)
        files = split_files(diff)
        if args.path:
            files = [f for f in files if any(fnmatch.fnmatch(f.path, g) for g in args.path)]
        skipped = [f.path for f in files if excluded(f.path) or f.binary]
        files = [f for f in files if not (excluded(f.path) or f.binary)]
        if not files:
            print("レビューする差分が無い。")
            return 0

        if args.dry_run:
            ctx = 12288
            info = None
            try:
                info = model_info(cfg)
            except ServerError:
                pass
            if info and info.get("loaded_context_length"):
                ctx = int(info["loaded_context_length"])
        elif args.ctx:
            ctx = args.ctx
        else:
            print("[local-review] 読み込みを確かめている（LM Studio の一覧は数十秒かかることがある）", file=sys.stderr)
            info = model_info(cfg)
            if not info or info.get("state") != "loaded":
                raise ServerError(f"{cfg.model} が読み込まれていない。先に --check / --load を実行する")
            ctx = int(info["loaded_context_length"])

        system_tokens = estimate_tokens(SYSTEM_PROMPT) + 64
        budget = min(args.chunk_tokens, ctx - system_tokens - args.max_tokens - 256)
        if budget < 512:
            raise ServerError(f"ctx {ctx} では差分に回せる予算が {budget} トークンしかない。ctx を増やすか --max-tokens を減らす")
        chunks = make_chunks(files, budget)
        send, rest = chunks[: args.max_chunks], chunks[args.max_chunks:]

        lines = [
            "# ローカル LLM の副レビュー",
            "",
            f"- モデル: `{cfg.model}`（ctx {ctx}、差分の予算 {budget} トークン/塊）",
            f"- ファイル: {len(files)} 件 → 塊 {len(chunks)} 個（送る {len(send)} 個）",
            f"- 見積もり: 約 {fmt_secs(sum(eta_seconds(c.tokens + system_tokens, args) for c in send))}"
            f"（prompt {args.pp_tps:g} tok/s、生成 {args.gen_tps:g} tok/s で計算）",
        ]
        if skipped:
            lines.append(f"- 送らなかったファイル（除外の規則）: {', '.join(f'`{s}`' for s in skipped)}")
        if rest:
            lines.append(f"- 上限（--max-chunks {args.max_chunks}）で送らなかった塊: "
                         + "; ".join(", ".join(c.files) for c in rest))
        lines.append("")
        if args.dry_run:
            for i, c in enumerate(chunks, 1):
                mark = "" if i <= args.max_chunks else "（送らない）"
                trunc = "、切り詰めあり" if c.truncated else ""
                lines.append(f"- 塊 {i}{mark}: 約 {c.tokens} トークン{trunc} — {', '.join(c.files)}")
            report = "\n".join(lines) + "\n"
            print(report, end="")
            return 0

        total_start = time.monotonic()
        for i, c in enumerate(send, 1):
            print(f"[local-review] 塊 {i}/{len(send)}: 約 {c.tokens} トークン、見積もり "
                  f"{fmt_secs(eta_seconds(c.tokens + system_tokens, args))} — {', '.join(c.files)}", file=sys.stderr)
            r = review_chunk(cfg, c, i, len(send), args)
            found, other = parse_findings(r["content"])
            u, s = r["usage"], r["stats"]
            lines += [f"## 塊 {i}: {', '.join(f'`{f}`' for f in c.files)}", ""]
            if c.truncated:
                lines += ["> 予算を超えた @@ を切り詰めて送った。切った後ろはレビューされていない。", ""]
            if found:
                lines += ["| # | 場所 | 重大度 | 指摘 |", "| --- | --- | --- | --- |"]
                for n, f in enumerate(found, 1):
                    body = f.body.replace("|", r"\|")
                    lines.append(f"| {n} | `{f.path}:{f.line}` | {f.severity} | {body} |")
                lines.append("")
            elif not other:
                lines += ["指摘なし（NO_FINDINGS）。", ""]
            if other:
                lines += ["形式に合わなかった出力:", "", "```text", *other, "```", ""]
            ttft = s.get("time_to_first_token")
            lines += [
                f"統計: 入力 {u.get('prompt_tokens', '?')} / 出力 {u.get('completion_tokens', '?')} トークン、"
                f"最初のトークンまで {ttft:.0f} 秒、生成 {s.get('tokens_per_second', 0):.2f} tok/s、"
                f"合計 {r['elapsed']:.0f} 秒、終了理由 {r['finish']}" if ttft is not None else
                f"統計: 合計 {r['elapsed']:.0f} 秒",
                "",
            ]
        lines.append(f"全体: {fmt_secs(time.monotonic() - total_start)}。"
                     "この指摘は小型モデルの助言であり、採否はコードを確かめてから決める。")
        report = "\n".join(lines) + "\n"
        print(report, end="")
        if args.out:
            Path(args.out).write_text(report, encoding="utf-8")
        return 0
    except ServerError as e:
        print(f"[local-review] {e}", file=sys.stderr)
        return 2
    except (RuntimeError, OSError, KeyError, ValueError) as e:
        print(f"[local-review] 失敗: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
