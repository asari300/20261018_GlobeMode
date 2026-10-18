# ADR-0005 salsa による遅延評価とシミュレーション前のアサーション

- 状態: Accepted（2026-10-18、S001）

## 背景

- 要件は2つある。
  - 遅延評価で処理を軽くすること。
  - シミュレーションできるモデルかを実行時にアサートすること。
- Rumoca には salsa のようなインクリメンタル計算の基盤が無い。
- IDE では、打鍵ごとに MSL 全体を flatten し直すわけにはいかない。

## 決定

- Phase A から **salsa** のクエリ DB（`globe-mode-db`）を置き、Rumoca の各段階の出力をクエリとして覚えておく。
- Eq で比較できる中立 IR を**防火壁**にする。
  - `outline`: 空白やコメントだけの編集では、ここから先が再計算されない。
  - `dae_structure`: ここが変わらなければ、検査と実行形式は作り直さない。
- MSL は必要になったファイルだけ parse する（`class_index`）。connect グラフは flatten しなくても作れるようにする（`connect_graph`）。
- simulate と export の前に必ず `assert_simulatable`（S0〜S10）を走らせる。
  - Error があれば `--force` 無しでは実行しない。
  - 「コンパイラが未対応」は GM-U コードとして分ける。
- シミュレーションの実行は salsa の外のジョブにする。

## 結果

- P2 の完了条件: 空白だけの編集の再検査が 50 ms 未満で、`flat` が再実行されないこと。salsa のイベントで確かめる。
- S5 は `omc checkModel()` と照合して正しさを確かめる。
