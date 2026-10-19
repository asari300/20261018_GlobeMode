//! シミュレーションの設定・ソルバの登録表・結果の置き場（ResultStore）・参照結果との比較（S011、ADR-0013）。
//!
//! Rumoca にも実行系にも依存しない（wasm32 でビルドできる）。実行は `globe-mode-exec` が受け持つ。
//!
//! - [`solvers`] … GlobeMode のソルバの名前の登録表（doc/architecture.md §3.3）。別名（`dassl` → `bdf`）と、
//!   計画だけあってまだ実装の無いもの（`esdirk34` など）を見分ける。
//! - [`settings`] … 利用者の指定・experiment annotation・既定値から実行の設定を決める。
//! - [`store`] … 結果の CSV（MAP-LIB の参照結果と同じ形: 1 行目は二重引用符で囲んだ名前、先頭の列は `time`）。
//! - [`compare`] … 参照結果との比較。csv-compare（BSD-3-Clause）の矩形の管（ファネル）を、アルゴリズムの
//!   記述から書き直したもの（コードは写していない）。

pub mod compare;
pub mod settings;
pub mod solvers;
pub mod store;
