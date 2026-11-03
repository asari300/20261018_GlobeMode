// ブラウザ（と Android の WebView）のファイル操作（S141）。`<input type=file>` と Blob のダウンロードだけ。
// shortcut: File System Access API は使わない（Firefox・Safari に無い）。同じファイルへの上書きはできず、保存は毎回ダウンロード。入れるなら Chromium だけで分岐する。

/** 選んだファイルの名前と本文。選ばずに閉じたら null。 */
export function browserOpen(): Promise<{ name: string; text: string } | null> {
  return new Promise((resolve, reject) => {
    // DOM に付けておく（付けないと change の前に回収されるブラウザがある）
    const input = Object.assign(document.createElement("input"), { type: "file", accept: ".mo", hidden: true });
    const done = () => input.remove();
    input.onchange = () => {
      const f = input.files?.[0];
      done();
      if (!f) return resolve(null);
      f.text().then((text) => resolve({ name: f.name, text }), reject);
    };
    input.oncancel = () => { done(); resolve(null); };
    document.body.append(input);
    input.click();
  });
}

/** 本文をダウンロードさせる。保存した名前を返す（保存先はブラウザが決めるので、名前だけ）。 */
async function download(name: string, data: BlobPart, type: string): Promise<string> {
  const a = Object.assign(document.createElement("a"), { href: URL.createObjectURL(new Blob([data], { type })), download: name });
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 10_000);
  return name;
}
/** 本文（S144 から FMU のバイト列も）をダウンロードさせる。 */
export const browserSave = (name: string, data: string | Uint8Array<ArrayBuffer>) =>
  download(name, data, typeof data === "string" ? "text/plain" : "application/octet-stream");
/** PNG をダウンロードさせる（S146）。 */
export const browserSaveBinary = (name: string, bytes: Uint8Array) => download(name, bytes as BlobPart, "image/png");
