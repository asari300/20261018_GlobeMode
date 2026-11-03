# S145 `web-bundle`

| | |
| --- | --- |
| 日付 | 2026-11-04 |
| ブランチ | `feature/s145-web-bundle` |
| worktree | `.git-worktree/s145-web-bundle` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S145-web-bundle/` |
| セッション記録 | （親が写す） |

## 依頼

S140 の点検の順位 7: ブラウザ版を配布できる形にする。静的な配布物を作って artifact に出す手動の workflow と、配布の手順まで。公開（Pages の有効化、URL の配布）はユーザーの判断なので行わない。

## 決めたこと

- vite の `base` を `./` にするだけでサブパスに対応できた（コードは `import.meta.env.BASE_URL` と `location.href` からの相対で引いていた）。`test_ui.py` に `/app/` だけで配るサーバを足して確かめた。
- CSP の正は `index.html` の meta。`web-bundle.sh` が取り出して `_headers` と `nginx.csp.conf` にする（二重管理しない）。
- `Cross-Origin-*` は不要: `SharedArrayBuffer`・`crossOriginIsolated` の使用は 0 件（grep）。
- 小画面: 400 px では CSS 無しだと木の固定幅（16 rem）にエディタが押されて 86 px になる（試験で再現。ボタンの横あふれはなく `scrollWidth` は 400 のまま）。`@media (max-width: 600px)` で木を上に積む。試験は `scrollWidth`・ボタンの位置・エディタの幅 300 px 以上の 3 つ。CSS を外すと最後の assert で落ちることを確かめた。
- 新しい workflow は main に無いと dispatch できないので、一時的に `push` トリガを feature に限って足し、最後のコミットで消した（S098 の流儀）。

## やったこと

- 新: `.github/workflows/web.yml`、`src/Svelte/scripts/web-bundle.sh`。
- 変: `src/Svelte/vite.config.ts`（`base`）、`src/Svelte/src/App.svelte`（`<style>` の末尾に media query だけ）、`src/Svelte/tests/test_ui.py`（`serve(prefix)`、サブパスと 400 px の 2 本）。
- 文書: `doc/features.md`・`doc/roadmap.md`・ADR-0023・セッション文書・索引。

## 検証

- `pnpm check`: 0 errors / 0 warnings。
- `test_ui.py`: 通過（サブパス・400 px を含む）。
- web.yml の run: run 41700103696 成功（一時の push トリガで起動。ジョブの中の `test_ui.py` も通過）。
- 大きさ: wasm 14.7 MiB（gzip 4.5 / brotli 3.1）、MSL.json 15.2 MiB（2.5 / 1.7）、合計 30.9 MiB（7.3 / 5.0）。手元の wasm は 17.7 MiB（古い pkg）で CI の方が小さい
- `render_slides.py`: 手元では 2 回とも通らず（30 s の時間切れ。load が高かった）、CI の docs で確認
- `check_docs.py`: デッキ OK（S145 を含む）、リンク 2941 件中の切れ 0 件
- セッション記録の `--check`: 親が写す。
- ステージした差分の漏洩検査: 0 件（メール・ホームのパス・トークンを grep）

## 残したこと

- 公開の判断（Pages か別のホストか）はユーザー。PWA、ダークモード、`MSL.json` の分割配信。
- GitHub Pages は header を付けられないので、Worker への CSP は効かない（解説書の 5 章）。header が要るなら Cloudflare Pages / Netlify / nginx。
