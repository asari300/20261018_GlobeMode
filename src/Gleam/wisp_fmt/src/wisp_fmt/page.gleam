//// 画面（1 ページ、日本語、外部 CDN なし）。属性は ' で囲む（Gleam の文字列の中で \ を使わないため）。

pub fn html() -> String {
  "<!doctype html>
<html lang='ja'>
<head>
<meta charset='utf-8'>
<meta name='viewport' content='width=device-width, initial-scale=1'>
<title>GlobeMode 整形・lint（Gleam）</title>
<style>
body { font-family: system-ui, sans-serif; margin: 0 auto; max-width: 960px; padding: 16px; }
textarea { width: 100%; box-sizing: border-box; font-family: ui-monospace, monospace; font-size: 14px; }
.row { margin: 8px 0; }
button { margin-right: 8px; padding: 6px 14px; }
#err { color: #b00020; white-space: pre-wrap; }
table { border-collapse: collapse; width: 100%; margin-top: 8px; }
th, td { border: 1px solid #999; padding: 4px 8px; text-align: left; font-size: 14px; }
</style>
</head>
<body>
<h1>Modelica の整形・lint</h1>
<p>GlobeMode の整形器と lint を、Gleam（wisp + mist）から CLI <code>globe</code> を呼んで使います。</p>

<div class='row'>
<label>Modelica のファイル（.mo）: <input type='file' id='file' accept='.mo'></label>
</div>
<textarea id='source' name='source' rows='12' placeholder='ここに Modelica のコードを貼る'></textarea>

<div class='row'>
<label>インデント: <select id='indent'><option value='2' selected>2</option><option value='4'>4</option></select></label>
<label><input type='checkbox' id='origin' checked> origin = {0, 0} を足す</label>
</div>

<div class='row'>
<button id='fmt' type='button'>整形</button>
<button id='lint' type='button'>lint</button>
</div>

<div id='err'></div>

<h2>整形の結果</h2>
<textarea id='out' name='output' rows='12' readonly></textarea>
<div class='row'><button id='back' type='button'>入力へ戻す</button></div>

<h2>lint の結果</h2>
<table>
<thead><tr><th>規則</th><th>行</th><th>桁</th><th>メッセージ</th></tr></thead>
<tbody id='lint-rows'></tbody>
</table>

<script>
const $ = (id) => document.getElementById(id);

function showError(message) {
  $('err').textContent = message || '';
}

function post(path, body) {
  return fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  }).then((r) => r.json().then((j) => ({ status: r.status, body: j })));
}

$('file').onchange = (e) => {
  const f = e.target.files[0];
  if (!f) return;
  const reader = new FileReader();
  reader.onload = () => { $('source').value = reader.result; };
  reader.readAsText(f);
};

$('fmt').onclick = () => {
  showError('');
  post('/api/format', {
    text: $('source').value,
    indent: Number($('indent').value),
    insert_origin: $('origin').checked
  }).then(({ body }) => {
    if (body.ok) {
      $('out').value = body.text;
    } else {
      showError(body.error);
    }
  }).catch((e) => showError(String(e)));
};

$('lint').onclick = () => {
  showError('');
  post('/api/lint', { text: $('source').value }).then(({ status, body }) => {
    if (status !== 200) {
      showError(body.error);
      return;
    }
    const tbody = $('lint-rows');
    tbody.textContent = '';
    for (const m of body.messages) {
      const tr = document.createElement('tr');
      for (const v of [m.rule, m.line, m.column, m.message]) {
        const td = document.createElement('td');
        td.textContent = String(v);
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
  }).catch((e) => showError(String(e)));
};

$('back').onclick = () => {
  $('source').value = $('out').value;
};
</script>
</body>
</html>
"
}
