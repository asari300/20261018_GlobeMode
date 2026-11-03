# S148 `ci-zig-mirror`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s148-ci-zig-mirror` |
| worktree | `.git-worktree/s148-ci-zig-mirror` |
| Phase | CI の基盤 |
| セッション文書 | `doc/sessions/S148-ci-zig-mirror/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

`ci.yml` の `lang` ジョブが、`jdx/mise-action@v5` の zig 0.17.0 の取得で、コミュニティのミラーが落ちて失敗する（run 41702810070）。取得を安定させる。

## 決めたこと

- 原因: mise 2026.10.25 は zig を、ziglang.org の推すコミュニティのミラー 17 件から順不同で先に試す。zigmirror.hryx.net が 502、zig.mirror.mschae23.de が 500。ミラーを使い切った後の ziglang.org は、ファイル名が `zig-linux-x86_64-0.17.0.tar.xz`（正しくは `zig-x86_64-linux-0.17.0.tar.xz`）で 404。直後の run 41705882970 は同じ `mise.lock` で成功した。名前の取り違えの理由は【未確認】。
- 対策は (b): `MISE_ZIG_USE_COMMUNITY_MIRRORS=false`（mise の設定 `zig.use_community_mirrors`。[出典](https://mise.jdx.dev/lang/zig.html)、確認 2026-11-04）。(a) の `actions/cache` は `mise-action` が既にキャッシュしていて（キーは `mise.toml`・`mise.lock` で外れる）、外れた回の取得を直さない。(c) の setup-zig は zig の版を二重に持つ。
- 当てた所は zig を入れる 3 か所: `ci.yml` の `lang`、`fmu-export.yml`、`release.yml`（cargo-zigbuild の zig）。ADR は作らず、ADR-0006 に 1 段落。
- ziglang.org は稼働率を保証しない。落ちたらこの行を消してミラーに戻す。

## やったこと

- 変: `.github/workflows/ci.yml`・`fmu-export.yml`・`release.yml`（env を 1 行ずつ）、`doc/adr/0006-incremental-ci.md`（段落）、`doc/roadmap.md`（S148 の節）、`doc/sessions/README.md`。
- 新: `doc/sessions/S148-ci-zig-mirror/`、この作業記録。
- `ci_plan.py`・`test_ci_plan.py` は影響なし。

## 検証

- 手元: `MISE_ZIG_USE_COMMUNITY_MIRRORS=false mise install zig@0.17.0`（別の `MISE_DATA_DIR`）が 15.8 s で `zig-x86_64-linux-0.17.0.tar.xz` を取れた。
- CI: `gh workflow run ci.yml --ref feature/s148-ci-zig-mirror -f full=true` の run 41708602678。`lang` は success（ログに `MISE_ZIG_USE_COMMUNITY_MIRRORS: false`、`mirror N/17` の行なし、`zig-x86_64-linux-0.17.0.tar.xz` を 14.4 s で取得）。同じ run の plan・deny・rust-sundials・ui も success（他は書いた時点で実行中。結果は親が確かめる）。
- `check_docs.py`: デッキ（S148 を含む全部）OK。
- `render_slides.py`（S148 だけ）: デッキ 1 件 / 問題 0 件。
- コードは触っていないので `mise run check` は回していない（cargo を回さない指示）。

## 残したこと

- ファイル名の取り違えの原因（mise 側。再発したら `MISE_VERBOSE=1` のログを取って報告）。
- ziglang.org が落ちたときの対策（zig の install 先の `actions/cache`、または設定を消してミラーに戻す）。
