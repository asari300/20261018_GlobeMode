// 側のパネルの編集（S264）。webview（信頼の境界の外）から届いた移動・結線のメッセージを検査し、LSP の要求にする。
// 検査を通らなければ null（黙って捨てる）。file と class は webview の言い分を使わず、パネルが最後に描かせた図のものを使う（panel.js）。
const name = (s) => typeof s === "string" && /^\w+$/.test(s); // 部品・コネクタの名前（Rust 側もソースに入れる文字を絞る）
const num = (v) => typeof v === "number" && Number.isFinite(v) && Math.abs(v) <= 1e6;

/** @returns {{ id: number, method: string, args: object } | null} */
function parseEdit(m) {
  if (!m || !Number.isSafeInteger(m.id)) return null;
  if (m.type === "move") {
    return name(m.part) && num(m.dx) && num(m.dy) ? { id: m.id, method: "globe/movePart", args: { part: m.part, dx: m.dx, dy: m.dy } } : null;
  }
  if (m.type === "connect") {
    const end = (s) => typeof s === "string" && s.split(".").length === 2 && s.split(".").every(name);
    const ok = Array.isArray(m.points) && m.points.length <= 64 && m.points.every((p) => Array.isArray(p) && p.length === 2 && p.every(num));
    return end(m.from) && end(m.to) && ok ? { id: m.id, method: "globe/addConnect", args: { from: m.from, to: m.to, points: m.points } } : null;
  }
  return null;
}

module.exports = { parseEdit };
