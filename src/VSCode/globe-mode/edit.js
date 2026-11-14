// 側のパネルの編集（S264）。webview（信頼の境界の外）から届いた移動・結線・削除（S293）のメッセージを検査し、LSP の要求にする。
// 検査を通らなければ null（黙って捨てる）。file と class は webview の言い分を使わず、パネルが最後に描かせた図のものを使う（panel.js）。
const name = (s) => typeof s === "string" && /^\w+$/.test(s); // 部品・コネクタの名前（Rust 側もソースに入れる文字を絞る）
// 結線の端: `名前` か `名前[添字]` を `.` でつないだ 2 つ（`a.p`・`b[2].n`・`b[i+1].n`）。Rust 側が字を絞って見る（addConnect）か、比べるだけで書かない（deleteConnect）
const seg = (s) => /^\w+(\[[\w,+*-]*\])?$/.test(s);
const end = (s) => typeof s === "string" && s.split(".").length === 2 && s.split(".").every(seg);
const num = (v) => typeof v === "number" && Number.isFinite(v) && Math.abs(v) <= 1e6;

/** @returns {{ id: number, method: string, args: object } | null} */
function parseEdit(m) {
  if (!m || !Number.isSafeInteger(m.id)) return null;
  if (m.type === "move") {
    return name(m.part) && num(m.dx) && num(m.dy) ? { id: m.id, method: "globe/movePart", args: { part: m.part, dx: m.dx, dy: m.dy } } : null;
  }
  if (m.type === "connect") {
    const ok = Array.isArray(m.points) && m.points.length <= 64 && m.points.every((p) => Array.isArray(p) && p.length === 2 && p.every(num));
    return end(m.from) && end(m.to) && ok ? { id: m.id, method: "globe/addConnect", args: { from: m.from, to: m.to, points: m.points } } : null;
  }
  if (m.type === "deletePart") return name(m.part) ? { id: m.id, method: "globe/deletePart", args: { part: m.part } } : null;
  if (m.type === "deleteConnect") return end(m.from) && end(m.to) ? { id: m.id, method: "globe/deleteConnect", args: { from: m.from, to: m.to } } : null;
  return null;
}

module.exports = { parseEdit };
