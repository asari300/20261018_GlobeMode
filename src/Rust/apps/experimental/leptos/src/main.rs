//! GlobeMode の Leptos 版（S153、ADR-0035）。エディタ（`<textarea>`）、検査と診断の一覧、実行と結果の表、MSL を読む。
//! 計算は facade `globe-mode` を同じ wasm の中で直接呼ぶ（Worker に出さない）ので、重い検査・実行の間は
//! メインスレッドが止まる。試作として許容する（解説書）。プロットと graph は持ち越し。
//! `?smoke=1` で Svelte 版と同じ `GM-SMOKE …` の行を console に出す（ADR-0034）。

use std::cell::RefCell;
use std::collections::BTreeMap;

use globe_mode::{Diagnostic, SimulateRequest, Workspace};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

const DECAY: &str =
    "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
const CHUA: &str = "Modelica.Electrical.Analog.Examples.ChuaCircuit";

thread_local! {
    // wasm32 のスレッドは 1 本。MSL を読むと中身を差し替える
    static WS: RefCell<Workspace> = RefCell::new(Workspace::new());
}

fn now() -> f64 {
    window().performance().map_or(0.0, |p| p.now())
}

fn line_of(d: &Diagnostic) -> String {
    format!("[{:?}] {}: {}", d.severity, d.code, d.message)
}

/// 画面の状態。`RwSignal` は Copy なので、そのままクロージャに渡せる。
#[derive(Clone, Copy)]
struct Ui {
    text: RwSignal<String>,
    model: RwSignal<String>,
    diags: RwSignal<Vec<Diagnostic>>,
    finals: RwSignal<Vec<(String, f64)>>,
    status: RwSignal<String>,
    msl: RwSignal<bool>,
    theme: RwSignal<&'static str>,
}

impl Ui {
    fn check(self) -> bool {
        let model = self.model.get_untracked();
        let r = WS.with_borrow_mut(|ws| {
            ws.set_file(&format!("{model}.mo"), self.text.get_untracked());
            ws.check(&model)
        });
        self.finals.set(vec![]);
        self.status.set(format!(
            "検査 {}: {}",
            model,
            if r.passed {
                "通った"
            } else {
                "通らない"
            }
        ));
        let ok = r.passed;
        self.diags.set(r.diagnostics);
        ok
    }

    /// 実行して、最後の値の表を出す。戻り値は最後の値（変数名 → 値）。
    fn simulate(self) -> BTreeMap<String, f64> {
        let model = self.model.get_untracked();
        let res = WS.with_borrow_mut(|ws| {
            ws.set_file(&format!("{model}.mo"), self.text.get_untracked());
            let request = SimulateRequest {
                model: model.clone(),
                no_csv: true,
                ..Default::default()
            };
            ws.simulate_with(&request, &now).response
        });
        self.status.set(match &res.refused {
            Some(why) => format!("実行できない: {why}"),
            None => format!("実行 {model}: {}", if res.ok { "済んだ" } else { "失敗" }),
        });
        self.diags.set(res.diagnostics);
        let finals = res.final_values.unwrap_or_default();
        self.finals
            .set(finals.iter().map(|(k, v)| (k.clone(), *v)).collect());
        finals
    }

    /// `msl/MSL.json`（`{"data/msl/…": 本文}`）を取得して読む。成功すると true。
    async fn load_msl(self) -> bool {
        self.status.set("MSL を読んでいる…".into());
        match fetch_text("msl/MSL.json").await {
            Err(e) => self.status.set(format!("MSL.json を取得できない: {e}")),
            Ok(json) => {
                let started = now();
                // 読み込みの間は画面が止まる（試作として許容）
                let loaded = serde_json::from_str::<BTreeMap<String, String>>(&json)
                    .map_err(|e| e.to_string())
                    .and_then(|files| {
                        Workspace::with_library_sources_split("MSL", &files)
                            .map_err(|ds| ds.iter().map(line_of).collect::<Vec<_>>().join(" / "))
                    });
                match loaded {
                    Ok(ws) => {
                        WS.set(ws);
                        self.msl.set(true);
                        self.status.set(format!(
                            "MSL を読んだ（{:.1} 秒）",
                            (now() - started) / 1000.0
                        ));
                    }
                    Err(e) => self.status.set(format!("MSL を読めない: {e}")),
                }
            }
        }
        self.msl.get_untracked()
    }

    /// Svelte 版の smoke と同じ手順（Decay の検査・実行、MSL を読んで ChuaCircuit の検査）で `GM-SMOKE` の行を出す。
    async fn smoke(self) {
        let ready = now(); // 同じ wasm の中なので往復は無く、ページの読み込みの始まりからここまで
        let checked = self.check();
        let x1 = self.simulate().get("x").copied().unwrap_or(f64::NAN);
        leptos::task::tick().await; // 結果が DOM に反映されるまで
        let decay = now() - ready;
        let t = now();
        let mut msl_ok = self.load_msl().await;
        if msl_ok {
            self.model.set(CHUA.into());
            msl_ok = self.check();
        }
        let msl_ms = if msl_ok {
            format!("{:.0}", now() - t)
        } else {
            "-".into()
        };
        let ok = |b: bool| if b { "ok" } else { "ng" };
        let simulated = (x1 - (-1f64).exp()).abs() < 1e-3;
        let failure = self
            .diags
            .get_untracked()
            .iter()
            .filter(|d| d.is_error())
            .map(line_of)
            .collect::<Vec<_>>()
            .join(" / ");
        let failure = if msl_ok || failure.is_empty() {
            String::new()
        } else {
            format!(" failure={failure}")
        };
        leptos::logging::log!(
            "GM-SMOKE check={} simulate={} x1={x1} msl={} ready_ms={ready:.0} decay_ms={decay:.0} msl_ms={msl_ms}{failure}",
            ok(checked),
            ok(simulated),
            ok(msl_ok)
        );
    }
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let res: web_sys::Response = JsFuture::from(window().fetch_with_str(url))
        .await
        .map_err(js)?
        .unchecked_into();
    if !res.ok() {
        return Err(format!("HTTP {}", res.status()));
    }
    let text = JsFuture::from(res.text().map_err(js)?).await.map_err(js)?;
    text.as_string().ok_or_else(|| "本文が文字列でない".into())
}

#[component]
fn App() -> impl IntoView {
    let ui = Ui {
        text: RwSignal::new(DECAY.into()),
        model: RwSignal::new("Decay".into()),
        diags: RwSignal::new(vec![]),
        finals: RwSignal::new(vec![]),
        status: RwSignal::new(String::new()),
        msl: RwSignal::new(false),
        theme: RwSignal::new("auto"),
    };
    // ダークモード: "auto" は OS の設定（CSS の prefers-color-scheme）に任せ、"light" / "dark" は data-theme で固定する
    Effect::new(move |_| {
        if let Some(root) = document().document_element() {
            let _ = root.set_attribute("data-theme", ui.theme.get());
        }
    });
    if window()
        .location()
        .search()
        .is_ok_and(|q| q.contains("smoke"))
    {
        leptos::task::spawn_local(ui.smoke());
    }
    view! {
        <header>
            <h1>"GlobeMode (Leptos)"</h1>
            <button id="theme" on:click=move |_| ui.theme.update(|t| *t = match *t { "auto" => "light", "light" => "dark", _ => "auto" })>
                "テーマ: " {move || ui.theme.get()}
            </button>
        </header>
        <main>
            <label>"モデル名 " <input aria-label="モデル名" prop:value=move || ui.model.get()
                on:input:target=move |e| ui.model.set(e.target().value()) /></label>
            <textarea aria-label="エディタ" spellcheck="false" prop:value=move || ui.text.get()
                on:input:target=move |e| ui.text.set(e.target().value())></textarea>
            <div class="row">
                <button id="check" on:click=move |_| { ui.check(); }>"検査"</button>
                <button id="run" on:click=move |_| { ui.simulate(); }>"実行"</button>
                <button id="msl" prop:disabled=move || ui.msl.get()
                    on:click=move |_| leptos::task::spawn_local(async move { ui.load_msl().await; })>"MSL を読む"</button>
                <span id="status" role="status">{move || ui.status.get()}</span>
            </div>
            <ul id="diagnostics" aria-label="診断">
                {move || ui.diags.get().iter().map(|d| view! { <li class=if d.is_error() { "error" } else { "" }>{line_of(d)}</li> }).collect_view()}
            </ul>
            <table id="finals" aria-label="最後の値">
                <thead><tr><th>"変数"</th><th>"最後の値"</th></tr></thead>
                <tbody>
                    {move || ui.finals.get().into_iter().map(|(k, v)| view! { <tr><td>{k}</td><td>{format!("{v:.6}")}</td></tr> }).collect_view()}
                </tbody>
            </table>
        </main>
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
