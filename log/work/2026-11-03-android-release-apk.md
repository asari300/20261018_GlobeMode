# S106 `android-release-apk`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s106-android-release-apk` |
| worktree | `.git-worktree/s106-android-release-apk` |
| Phase | P6 |
| セッション文書 | `doc/sessions/S106-android-release-apk/` |
| セッション記録 | （親が写す） |

## 依頼

`android.yml` に release の APK（`--target aarch64` だけでよい）を足し、debug と大きさを並べて Step Summary に出す。secret（`ANDROID_KEYSTORE_BASE64`・`ANDROID_KEYSTORE_PASSWORD`・`ANDROID_KEY_ALIAS`・`ANDROID_KEY_PASSWORD`）がある場合だけ署名する（`env` に写して `if: env.X != ''`）。鍵は作らない・コミットしない。鍵の作り方と secret の登録手順を解説書と ADR-0023 に書く（出典 URL と確認日）。release の APK でもエミュレータの smoke（`GM-SMOKE … msl=ok`）が通る。前の担当エージェントが落ちたので、2 コミット（fe5900c・6317097）から再開した。

## 決めたこと

- release は `--split-per-abi --target aarch64 --target x86_64`。配るのは aarch64 だけだが、エミュレータ（x86_64）の smoke に x86_64 の release が要るため（依頼の「aarch64 だけ」から足した）。
- 署名は `gen/android/keystore.properties`（`.gitignore` 済み）を secret から書き、`build.gradle.kts` がそれを見る（Tauri の手引きの形）。パスワードは `storePassword`・`keyPassword` に分けた。
- smoke 用の release は署名なしなら runner の中で作る使い捨ての鍵で署名する。`~/.android/debug.keystore` は runner に無かった（run 41530541915 の失敗）。
- smoke 用の x86_64 は `GM_SMOKE_DEBUGGABLE=1` で debuggable に作り直す。Tauri（wry の `Logger.kt`）は `BuildConfig.DEBUG` のときだけ WebView の console を logcat に出すので、run 41537253164 では release の smoke で `GM-SMOKE` の行が出なかった。アプリのコードは変えない条件なので、配布物を upload したあと smoke 用だけを作り直す道にした。R8 と release の Rust は配布と同じ。
- UI は debug と同じ `VITE_GM_SMOKE=1` のビルドを release にも使う（ビルド 1 回。配布用を分けるのは持ち越し）。

## やったこと

- `.github/workflows/android.yml`: release の APK、secret があるときだけの `keystore.properties`、大きさの表（Step Summary）、artifact `globe-android-release`・`globe-android-smoke`、`emulator` の matrix（debug・release、logcat は `android-logcat`・`android-logcat-release`）。
- `src/Rust/apps/desktop/gen/android/app/build.gradle.kts`: `keystore.properties` があるときだけ `signingConfigs.release`、`GM_SMOKE_DEBUGGABLE=1` のときだけ release を debuggable に。
- 文書: 解説書・スライド、ADR-0023 の「S106」、roadmap の P6 の行と「S106 で済んだこと」、索引の行。

## 検証

- APK の大きさ（run 41544046811 の Step Summary）: debug の universal 257,983,951 bytes（246.0 MiB）、release の aarch64 12,706,158 bytes（12.1 MiB）、x86_64 13,290,251 bytes（12.7 MiB）。いずれも署名なし（secret 未登録）。
- Actions: run 41530541915 は failure（debug.keystore が無い）、41537215067 は cancelled、41537253164 は release の smoke だけ failure（`GM-SMOKE` が logcat に出ない）、41543973496 は push の失敗で古いコミットを回したので cancel。**41544046811（aec3f7b）が全ジョブ success**。release の smoke は `Displayed` の 14.7 s 後、debug は 11.8 s 後に `GM-SMOKE check=ok simulate=ok x1=0.3678797279080248 msl=ok`
- コードの検査（fmt / clippy / test / deny）: Rust のコードは触っていないので回していない（`build.gradle.kts` と workflow だけ）
- `check_docs.py`: デッキ全件 OK（S106 を含む）/ 文書 433 件・相対リンク 2113 件中の切れ 0 件
- セッション記録の `--check`: 親が写すので、このセッションでは写していない
- ステージした差分の漏洩検査: 0 件（ステージした差分 6 ファイル）

## 残したこと

- secret を登録した run で、署名つきの aarch64 の APK を実機に入れて確かめる（ユーザーの鍵が要る）。
- 配布用の UI を smoke なし（`VITE_GM_SMOKE` を付けない）で作るか。
- Rust を mobile の lib に直接 link する道。
- emulator のジョブは smoke の後で `adb shell pidof` を見るので、gms の落ちに巻き込まれてアプリが kill されると失敗しうる（run 41537253164 では 60 s 後に kill されていた。debug は通った）。
