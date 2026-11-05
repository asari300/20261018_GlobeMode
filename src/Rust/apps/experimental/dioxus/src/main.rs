//! GlobeMode の Dioxus 版（S155、ADR-0036）。S153 の Leptos 版と同じ機能: エディタ（`<textarea>`）、検査と診断の一覧、
//! 実行と最後の値の表、MSL を読む、ダークモード。計算は facade `globe-mode` を同じ wasm の中で直接呼ぶ
//! （重い検査・実行の間はメインスレッドが止まる。試作として許容する）。プロットと graph は持ち越し。
//! `?smoke=1` で Svelte 版・Leptos 版と同じ `GM-SMOKE …` の行を console に出す（ADR-0034）。

use std::cell::RefCell;
use std::collections::BTreeMap;

use dioxus::prelude::*;
use globe_mode::{Diagnostic, SimulateRequest, Workspace};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

const DECAY: &str =
    "model Decay\n  Real x(start = 1, fixed = true);\nequation\n  der(x) = -x;\nend Decay;\n";
const CHUA: &str = "Modelica.Electrical.Analog.Examples.ChuaCircuit";

thread_local! {
    // wasm32 のスレッドは 1 本。MSL を読むと中身を差し替える
    static WS: RefCell<Workspace> = RefCell::new(Workspace::new());
}

fn window() -> web_sys::Window {
    web_sys::window().expect("window")
}

fn now() -> f64 {
    window().performance().map_or(0.0, |p| p.now())
}

fn line_of(d: &Diagnostic) -> String {
    format!("[{:?}] {}: {}", d.severity, d.code, d.message)
}

/// 画面の状態。`Signal` は Copy なので、そのままクロージャに渡せる。
#[derive(Clone, Copy)]
struct Ui {
    text: Signal<String>,
    model: Signal<String>,
    diags: Signal<Vec<Diagnostic>>,
    finals: Signal<Vec<(String, f64)>>,
    status: Signal<String>,
    msl: Signal<bool>,
    theme: Signal<&'static str>,
}

impl Ui {
    fn check(mut self) -> bool {
        let model = self.model.peek().clone();
        let r = WS.with_borrow_mut(|ws| {
            ws.set_file(&format!("{model}.mo"), self.text.peek().clone());
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
    fn simulate(mut self) -> BTreeMap<String, f64> {
        let model = self.model.peek().clone();
        let res = WS.with_borrow_mut(|ws| {
            ws.set_file(&format!("{model}.mo"), self.text.peek().clone());
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
    async fn load_msl(mut self) -> bool {
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
        *self.msl.peek()
    }

    /// Svelte 版・Leptos 版の smoke と同じ手順で `GM-SMOKE` の行を出す。
    async fn smoke(mut self) {
        let ready = now(); // 同じ wasm の中なので往復は無く、ページの読み込みの始まりからここまで
        let checked = self.check();
        let x1 = self.simulate().get("x").copied().unwrap_or(f64::NAN);
        next_task().await; // 結果が DOM に反映されるまで
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
            .peek()
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
        web_sys::console::log_1(
            &format!(
                "GM-SMOKE check={} simulate={} x1={x1} msl={} ready_ms={ready:.0} decay_ms={decay:.0} msl_ms={msl_ms}{failure}",
                ok(checked),
                ok(simulated),
                ok(msl_ok)
            )
            .into(),
        );
    }
}

/// setTimeout(0) を待つ。Dioxus の描画の後まで進める（Leptos 版の `tick()` に当たる）。
async fn next_task() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
    });
    let _ = JsFuture::from(promise).await;
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
fn App() -> Element {
    let mut ui = Ui {
        text: use_signal(|| DECAY.to_string()),
        model: use_signal(|| "Decay".to_string()),
        diags: use_signal(Vec::new),
        finals: use_signal(Vec::new),
        status: use_signal(String::new),
        msl: use_signal(|| false),
        theme: use_signal(|| "auto"),
    };
    // ダークモード: "auto" は OS の設定（CSS の prefers-color-scheme）に任せ、"light" / "dark" は data-theme で固定する
    use_effect(move || {
        if let Some(root) = window().document().and_then(|d| d.document_element()) {
            let _ = root.set_attribute("data-theme", (ui.theme)());
        }
    });
    use_hook(|| {
        if window()
            .location()
            .search()
            .is_ok_and(|q| q.contains("smoke"))
        {
            spawn(ui.smoke());
        }
    });
    rsx! {
        header {
            h1 { "GlobeMode (Dioxus)" }
            button { id: "theme",
                onclick: move |_| ui.theme.set(match (ui.theme)() { "auto" => "light", "light" => "dark", _ => "auto" }),
                "テーマ: {ui.theme}"
            }
        }
        main {
            label { "モデル名 "
                input { "aria-label": "モデル名", value: "{ui.model}", oninput: move |e| ui.model.set(e.value()) }
            }
            textarea { "aria-label": "エディタ", spellcheck: "false", value: "{ui.text}", oninput: move |e| ui.text.set(e.value()) }
            div { class: "row",
                button { id: "check", onclick: move |_| { ui.check(); }, "検査" }
                button { id: "run", onclick: move |_| { ui.simulate(); }, "実行" }
                button { id: "msl", disabled: (ui.msl)(),
                    onclick: move |_| { spawn(async move { ui.load_msl().await; }); },
                    "MSL を読む"
                }
                span { id: "status", role: "status", "{ui.status}" }
            }
            ul { id: "diagnostics", "aria-label": "診断",
                for (i, d) in ui.diags.read().iter().enumerate() {
                    li { key: "{i}", class: if d.is_error() { "error" } else { "" }, "{line_of(d)}" }
                }
            }
            table { id: "finals", "aria-label": "最後の値",
                thead { tr { th { "変数" } th { "最後の値" } } }
                tbody {
                    for (k, v) in ui.finals.read().iter() {
                        tr { key: "{k}", td { "{k}" } td { "{v:.6}" } }
                    }
                }
            }
        }
    }
}

fn main() {
    dioxus::launch(App);
}
