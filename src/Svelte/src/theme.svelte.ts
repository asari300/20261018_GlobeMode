// 明 / 暗 / OS に合わせる（S152）。選択は localStorage。配色は theme.css の変数で、JS で色を渡す部品（uPlot・sigma・CodeMirror の darkTheme）は theme.dark を見る。
import "./theme.css";

export type Pref = "auto" | "light" | "dark";
const KEY = "globe-mode.theme";
const mq = matchMedia("(prefers-color-scheme: dark)");
let saved: Pref = "auto";
try {
  const v = localStorage.getItem(KEY);
  if (v === "light" || v === "dark") saved = v;
} catch { /* 読めなければ OS に合わせる */ }

export const theme = $state({ pref: saved, dark: false });

function apply() {
  const el = document.documentElement;
  if (theme.pref === "auto") el.removeAttribute("data-theme");
  else el.dataset.theme = theme.pref;
  theme.dark = theme.pref === "dark" || (theme.pref === "auto" && mq.matches);
}

export function setPref(p: Pref) {
  theme.pref = p;
  try { localStorage.setItem(KEY, p); } catch { /* 保存できなくても使える */ }
  apply();
}

mq.addEventListener("change", apply);
apply();

/** いまの変数の値（canvas に色を渡す部品用。theme.dark が変わったあとに呼ぶ） */
export const cssVar = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
