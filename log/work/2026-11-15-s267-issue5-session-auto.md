# S267 `issue5-session-auto`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s267-issue5-session-auto` |
| worktree | `.git-worktree/s267-issue5-session-auto` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S267-issue5-session-auto/` |
| セッション記録 | なし（このセッションの記録が `log/local/` に無いため省く） |

## 依頼

issue #5（session の道で BDF が始められないとき auto が dopri45 に切り替わらない）を今の main で確かめて直す。issue は閉じない。

## 決めたこと

- issue の例（`GenerationOfFMUs`）は ADR-0015 で解消済み（両方の道で GM-U0004 で止まる）。切り替える規則 `fallback()` は元から両方の道で同じ関数。
- 残る差は `into_session` の失敗の段だけ（session の道は Integration、ネイティブの BDF は Initialization）。段の無い `Numerical` の失敗だけに出る。
- 直しは `session::start_phase` で BDF だけ Initialization。rk45 と自前の積分器はネイティブも積分の段なので Integration のまま（esdirk34 が始められなくても dopri45 に回る規則を壊さない）。

## やったこと

- `rumoca/session.rs`: `start_phase` を足し、`into_session` の失敗に使う。`rumoca.rs`: `Phase::Initialization` の doc を直し、単体試験を 1 本足した。
- 再現を試した: 小さなモデル 13 本と `GenerationOfFMUs` で `bdf`・`auto` の両方の道が一致。段の無い構築の失敗を起こすモデルは見つからなかった。
- ADR-0014 に付記、roadmap の #5 の行を更新。

## 検証

- `cargo test -p globe-mode-exec` 24 + 14 + 31 本 pass（5 本 ignored は MSL）。`cargo clippy -p globe-mode-exec --all-targets -- -D warnings` 警告 0、`cargo fmt` 済み。
- `mise run wasm:node`・`wasm:compare` は release の wasm ビルドが重いので手元では回していない。CI の wasm-node に任せる。`msl:sim` も回していない（ネイティブの挙動は変えず、wasm では段の無い BDF の始動の失敗だけが変わる）。

## 残したこと

- モデルでの再現の試験は無い。diffsol の構築が段なしで失敗するモデルが見つかったら、`simulate` と `simulate_via_session` を比べる試験を足す。
- issue #5 は閉じていない。
