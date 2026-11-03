/*
 * GlobeMode の C ABI（S075、crate globe-mode-c）。手書きのヘッダ。
 * 署名は src/lib.rs と tests/header.rs で突き合わせる。SPDX-License-Identifier: MIT OR Apache-2.0
 *
 * - char* を返す関数の結果は JSON の文字列。必ず gm_string_free で放す。誤りは {"error": "..."}。
 * - GmWorkspace は作ったスレッドからだけ使う。
 */
#ifndef GLOBE_MODE_H
#define GLOBE_MODE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct GmWorkspace GmWorkspace;

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
