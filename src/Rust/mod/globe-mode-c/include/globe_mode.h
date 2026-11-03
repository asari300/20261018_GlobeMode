/*
 * GlobeMode の C ABI（S075、crate globe-mode-c）。手書きのヘッダ。
 * 署名は src/lib.rs と tests/header.rs で突き合わせる。SPDX-License-Identifier: MIT OR Apache-2.0
 *
 * - char* を返す関数の結果は JSON の文字列。必ず gm_string_free で放す。誤りは {"error": "..."}。
 * - GmWorkspace は作ったスレッドからだけ使う。
 */
#ifndef GLOBE_MODE_H
#define GLOBE_MODE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct GmWorkspace GmWorkspace;

/* 実行の進捗（S131、ADR-0033）。stage は "lowering"・"integration" などの小文字で、コールバックの間だけ有効。
 * time は積分の段だけ（has_time が 0 なら NaN）。 */
typedef struct GmProgress {
    const char *stage;
    double time;
    int has_time;
    double start_time;
    double stop_time;
} GmProgress;
/* 進捗のコールバック。非 0 を返すと次の区切りで中断する（termination.kind = "cancelled" と GM-I11004）。 */
typedef int (*GmProgressCallback)(const GmProgress *progress, void *user_data);

/* libraries_json: ライブラリのディレクトリの JSON の配列（NULL か "[]" で無し）。失敗で NULL。 */
GmWorkspace *gm_workspace_new(const char *libraries_json);
void gm_workspace_free(GmWorkspace *ws);
/* 0 で成功、-1 で失敗。 */
int gm_set_file(GmWorkspace *ws, const char *path, const char *text);
/* 読んだ .mo の数、-1 で失敗。 */
int64_t gm_load_dir(GmWorkspace *ws, const char *dir);
/* CheckResponse の JSON。 */
char *gm_check(GmWorkspace *ws, const char *model);
/* ConnectGraph の JSON。 */
char *gm_graph(GmWorkspace *ws, const char *model, uint32_t depth);
/* SimulateResponse の JSON。settings_json は NULL 可（{"stop_time": 2, "solver": "bdf"} など）。 */
char *gm_simulate(GmWorkspace *ws, const char *model, const char *settings_json);
/* 列の道（S116）。{"response": SimulateResponse（csv 無し）, "names": ["time", ...], "points": n} の JSON。
 * *data に列を順に並べた names の数 * points 個の double（列 i は data + i * points）、*len にその個数。
 * *data は gm_doubles_free で放す。誤りや実行できなかったときは *data が NULL、*len が 0。 */
char *gm_simulate_columns(GmWorkspace *ws, const char *model, const char *settings_json, double **data, size_t *len);
void gm_doubles_free(double *data, size_t len);
/* gm_simulate・gm_simulate_columns に進捗と中断を足したもの（S131）。callback は NULL 可（そのときは同じ道）。
 * 積分の途中で止めたら止めた時刻までの CSV・列、前なら ok: false。 */
char *gm_simulate_observed(GmWorkspace *ws, const char *model, const char *settings_json, GmProgressCallback callback, void *user_data);
char *gm_simulate_columns_observed(GmWorkspace *ws, const char *model, const char *settings_json, double **data, size_t *len, GmProgressCallback callback, void *user_data);
/* FMU の書き出し（S118、globe fmu export と同じ）。{"path": ".fmu のパス"}。options_json は NULL 可
 * （{"fmi_version": 2, "build": false, "targets": [], "fmi_headers": null}）。 */
char *gm_export_fmu(GmWorkspace *ws, const char *model, const char *out_dir, const char *options_json);
/* ソルバの登録表（JSON の配列）。 */
char *gm_solvers(void);
/* 整形（S111）。{"text": "..."}、文法の誤りは {"error": "..."}。insert_origin は 0 か 1。 */
char *gm_format(const char *text, uint32_t indent, int insert_origin);
/* lint（S111）。[{"rule", "line", "column", "message"}] の JSON。 */
char *gm_lint(const char *text);
void gm_string_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* GLOBE_MODE_H */
