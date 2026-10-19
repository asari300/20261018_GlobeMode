//! 参照結果との比較（矩形の管、csv-compare の既定のアルゴリズム）。
//!
//! csv-compare（modelica-tools/csv-compare、BSD-3-Clause、ESI ITI）の既定の「Rectangle」の管を、
//! アルゴリズムの記述から書き直したもの（コードは写していない）。
//!
//! # 管
//!
//! 参照の信号を、頂点を直線で結んだ折れ線とみなす（同じ時刻が続くところは、イベントでの縦の跳び）。
//! 幅 2δt・高さ 2δy の矩形の中心を折れ線の上で動かしたときに掃く領域を管とする。
//!
//! - δt = tol · max(t_N − t_0, |t_0|, 1e-12)（t_0、t_N は参照の最初と最後の時刻）
//! - δy = tol · max(y_max − y_min, |y_min|, 0.001)（y_min、y_max は参照の信号の最小と最大）
//!
//! 矩形の掃く領域の縁は、窓 |s − t| ≤ δt の中での折れ線の最大・最小を δy だけ広げたものになる
//! （この言い換えは GlobeMode での導出）。
//!
//! - 上の縁 U(t) = max{ y_ref(s) : |s − t| ≤ δt, t_0 ≤ s ≤ t_N } + δy
//! - 下の縁 L(t) = min{ y_ref(s) : |s − t| ≤ δt, t_0 ≤ s ≤ t_N } − δy
//!
//! 試験側の点 (t, y) のうち t_0 ≤ t ≤ t_N のものだけを判定し（外の点は外挿せずに読み飛ばす）、
//! y < L(t) か y > U(t) なら管の外とする。
//!
//! 試験側の時刻の範囲が参照の区間を覆わない（最初の時刻が t_0 + δt より後か、最後の時刻が t_N − δt より前。
//! 途中で止まった実行など）信号は、管の中にあっても不合格にする（`uncovered`）。
//!
//! # 計算
//!
//! 折れ線は頂点の間で 1 次式なので、窓 [a, b] = [max(t − δt, t_0), min(t + δt, t_N)] の中の最大・最小は、
//! 窓の中の頂点か、窓の両端（頂点の間にあれば 1 次補間した値）のどれかで取る。そこで
//!
//! 1. 窓の中の頂点の値の最大・最小を、セグメント木（最大と最小の 2 本）で引く
//! 2. 窓の両端が頂点の間にあれば、その区間を 1 次補間した値も比べる
//!
//! とすれば、試験側の点ごとに U(t)、L(t) が（丸めの誤差を除いて）厳密に決まる。イベントでの縦の跳び
//! （同じ時刻の頂点）は、窓に入れば跳ぶ前と後の両方の値が比べられる。参照 n 点、試験 m 点のとき、
//! 信号 1 つあたり時間 O(n + m log n)、場所 O(n)（セグメント木は信号ごとに作り直し、場所は使い回す）。

use std::borrow::Cow;
use std::collections::HashMap;

use globe_mode_ir::{Trajectory, TrajectoryVariable};
use serde::Serialize;

/// 管の幅。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TubeOptions {
    /// 相対の許容差（時間と値の両方向。csv-compare の既定は 0.002）。負・NaN・無限大は 0（完全一致）とみなす
    pub tolerance: f64,
}

impl Default for TubeOptions {
    fn default() -> Self {
        TubeOptions { tolerance: 0.002 }
    }
}

/// 信号 1 つの比較。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignalComparison {
    pub name: String,
    /// 両方に信号があり、試験側が参照の区間を覆い、判定した点が 1 つ以上あり、管の外に出た点が 0 なら true
    pub passed: bool,
    /// 試験側か参照側にその信号が無い（値の数が時刻の数と違う壊れた変数も、無いものとみなす）
    pub missing: bool,
    /// 参照側にその信号が無い（`missing` のうち参照側のもの）
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub missing_in_reference: bool,
    /// 試験側の時刻の範囲が参照の区間を覆わない（最初の時刻 > t_0 + δt か、最後の時刻 < t_N − δt。
    /// 試験側に有限の時刻が無いときも true）
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub uncovered: bool,
    /// 判定した点の数（参照の区間の中にある試験側の点）
    pub checked: usize,
    /// 管の外に出た点の数（試験側の値が NaN の点も数える）
    pub errors: usize,
    /// 管からはみ出した量の最大（値の単位。NaN の点は無限大）
    pub max_excess: f64,
    /// 最初に管の外に出た時刻
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_error_time: Option<f64>,
}

impl SignalComparison {
    fn missing(name: &str, in_reference: bool) -> Self {
        SignalComparison {
            name: name.to_string(),
            passed: false,
            missing: true,
            missing_in_reference: in_reference,
            uncovered: false,
            checked: 0,
            errors: 0,
            max_excess: 0.0,
            first_error_time: None,
        }
    }
}

/// モデル 1 本の比較。
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Comparison {
    pub signals: Vec<SignalComparison>,
}

impl Comparison {
    /// すべての信号が合格なら true（信号が 1 つも無ければ false）。
    pub fn passed(&self) -> bool {
        !self.signals.is_empty() && self.signals.iter().all(|s| s.passed)
    }
}

/// `signals`（`time` を除いた信号の名前。MAP-LIB の `comparisonSignals.txt`）を、参照結果の管で判定する。
///
/// 名前は大文字と小文字を区別して完全一致で引く。`time`（大文字と小文字は問わない）は読み飛ばす。
/// 参照の時刻が並んでいない・有限でないときは、有限の点だけを時刻の順に並べ直して使う。
pub fn compare(
    reference: &Trajectory,
    actual: &Trajectory,
    signals: &[&str],
    options: &TubeOptions,
) -> Comparison {
    let tolerance = if options.tolerance.is_finite() && options.tolerance > 0.0 {
        options.tolerance
    } else {
        0.0
    };
    let reference_time = ReferenceTime::new(&reference.time);
    let reference_vars = by_name(reference);
    let actual_vars = by_name(actual);
    let actual_span =
        actual
            .time
            .iter()
            .filter(|t| t.is_finite())
            .fold(None, |span: Option<(f64, f64)>, &t| {
                Some(span.map_or((t, t), |(first, last)| (first.min(t), last.max(t))))
            });
    let mut extrema = Extrema::default();
    let signals = signals
        .iter()
        .filter(|name| !name.eq_ignore_ascii_case("time"))
        .map(|&name| {
            let Some(r) = reference_vars.get(name) else {
                return SignalComparison::missing(name, true);
            };
            let Some(a) = actual_vars.get(name) else {
                return SignalComparison::missing(name, false);
            };
            let values = reference_time.values(&r.values);
            extrema.rebuild(&values);
            let tube = Tube::new(&reference_time.time, &values, &extrema, tolerance);
            check(name, tube.as_ref(), actual_span, &actual.time, &a.values)
        })
        .collect();
    Comparison { signals }
}

/// MAP-LIB の `comparisonSignals.txt` を読む（1 行に名前 1 つ）。前後の空白を除き、空行と `time` の行
/// （ふつうは 1 行目。大文字と小文字は問わない）と、`//` で始まる行（比べないことにした信号を
/// 書き残したもの。MSL 4.1.0 の参照結果の 6 本にある。S011）を読み飛ばす。
pub fn parse_comparison_signals(text: &str) -> Vec<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case("time") && !l.starts_with("//"))
        .map(str::to_string)
        .collect()
}

/// 名前で変数を引く表（同じ名前が 2 つあれば先のもの。値の数が時刻の数と違う変数は入れない）。
fn by_name(t: &Trajectory) -> HashMap<&str, &TrajectoryVariable> {
    let mut map = HashMap::with_capacity(t.variables.len());
    for v in &t.variables {
        if v.values.len() == t.time.len() {
            map.entry(v.name.as_str()).or_insert(v);
        }
    }
    map
}

/// 試験側の点を管で判定する（管が無い＝参照に有限の時刻が無いときは、判定した点 0 で不合格）。
/// `actual_span` は試験側の有限の時刻の（最初, 最後）。
fn check(
    name: &str,
    tube: Option<&Tube<'_>>,
    actual_span: Option<(f64, f64)>,
    time: &[f64],
    values: &[f64],
) -> SignalComparison {
    let mut result = SignalComparison {
        name: name.to_string(),
        passed: false,
        missing: false,
        missing_in_reference: false,
        uncovered: false,
        checked: 0,
        errors: 0,
        max_excess: 0.0,
        first_error_time: None,
    };
    let Some(tube) = tube else {
        return result;
    };
    result.uncovered = actual_span
        .is_none_or(|(first, last)| first > tube.t0 + tube.dt || last < tube.tn - tube.dt);
    for (&t, &y) in time.iter().zip(values) {
        if !(t >= tube.t0 && t <= tube.tn) {
            continue;
        }
        result.checked += 1;
        let (lower, upper) = tube.bounds(t);
        let excess = if y.is_nan() {
            f64::INFINITY
        } else if y > upper {
            y - upper
        } else if y < lower {
            lower - y
        } else {
            continue;
        };
        result.errors += 1;
        result.max_excess = result.max_excess.max(excess);
        result.first_error_time = Some(result.first_error_time.map_or(t, |f| f.min(t)));
    }
    result.passed = !result.uncovered && result.checked > 0 && result.errors == 0;
    result
}

/// 参照の時刻（有限の点だけを、時刻の順に）。並んでいれば元の列をそのまま借りる。
struct ReferenceTime<'a> {
    time: Cow<'a, [f64]>,
    /// 並べ直したときの元の行の番号
    order: Option<Vec<usize>>,
}

impl<'a> ReferenceTime<'a> {
    fn new(time: &'a [f64]) -> Self {
        let in_order = time.iter().all(|t| t.is_finite()) && time.windows(2).all(|w| w[0] <= w[1]);
        if in_order {
            return ReferenceTime {
                time: Cow::Borrowed(time),
                order: None,
            };
        }
        let mut order: Vec<usize> = (0..time.len()).filter(|&i| time[i].is_finite()).collect();
        // 安定な並べ替え（同じ時刻の行の順＝イベントの前と後を保つ）
        order.sort_by(|&i, &j| time[i].total_cmp(&time[j]));
        ReferenceTime {
            time: order.iter().map(|&i| time[i]).collect(),
            order: Some(order),
        }
    }

    fn values<'b>(&self, values: &'b [f64]) -> Cow<'b, [f64]> {
        match &self.order {
            None => Cow::Borrowed(values),
            Some(order) => order.iter().map(|&i| values[i]).collect(),
        }
    }
}

/// 頂点の値の区間の最大・最小を引くセグメント木（下から組む形。葉は `n..2n`）。
#[derive(Debug, Default)]
struct Extrema {
    n: usize,
    min: Vec<f64>,
    max: Vec<f64>,
}

impl Extrema {
    /// 値を入れ替えて作り直す（場所は使い回す）。NaN は `f64::min` / `f64::max` で無視される。
    fn rebuild(&mut self, values: &[f64]) {
        let n = values.len();
        self.n = n;
        for tree in [&mut self.min, &mut self.max] {
            tree.clear();
            tree.resize(n, 0.0);
            tree.extend_from_slice(values);
        }
        for i in (1..n).rev() {
            self.min[i] = self.min[2 * i].min(self.min[2 * i + 1]);
            self.max[i] = self.max[2 * i].max(self.max[2 * i + 1]);
        }
    }

    /// 頂点 `lo..hi` の値の（最小, 最大）。空なら (+∞, −∞)。
    fn range(&self, lo: usize, hi: usize) -> (f64, f64) {
        let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut l, mut r) = (lo + self.n, hi + self.n);
        while l < r {
            if l & 1 == 1 {
                min = min.min(self.min[l]);
                max = max.max(self.max[l]);
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                min = min.min(self.min[r]);
                max = max.max(self.max[r]);
            }
            l >>= 1;
            r >>= 1;
        }
        (min, max)
    }
}

/// 参照の信号 1 つの管。
#[derive(Debug)]
struct Tube<'a> {
    time: &'a [f64],
    values: &'a [f64],
    extrema: &'a Extrema,
    t0: f64,
    tn: f64,
    dt: f64,
    dy: f64,
}

impl<'a> Tube<'a> {
    /// 管を作る。参照に点が無ければ None。
    fn new(
        time: &'a [f64],
        values: &'a [f64],
        extrema: &'a Extrema,
        tolerance: f64,
    ) -> Option<Self> {
        let (&t0, &tn) = (time.first()?, time.last()?);
        let (y_min, y_max) = values
            .iter()
            .filter(|y| y.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &y| {
                (lo.min(y), hi.max(y))
            });
        let (range, floor) = if y_min <= y_max {
            (y_max - y_min, y_min.abs())
        } else {
            (0.0, 0.0)
        };
        Some(Tube {
            time,
            values,
            extrema,
            t0,
            tn,
            dt: tolerance * (tn - t0).max(t0.abs()).max(1e-12),
            dy: tolerance * range.max(floor).max(0.001),
        })
    }

    /// 時刻 t（t_0 ≤ t ≤ t_N）での管の（下の縁, 上の縁）。
    fn bounds(&self, t: f64) -> (f64, f64) {
        let (min, max) = self.window(t);
        (min - self.dy, max + self.dy)
    }

    /// 窓 [max(t − δt, t_0), min(t + δt, t_N)] の中での折れ線の（最小, 最大）。
    fn window(&self, t: f64) -> (f64, f64) {
        let ts = self.time;
        let n = ts.len();
        let a = (t - self.dt).max(self.t0);
        let b = (t + self.dt).min(self.tn);
        // 窓の中の頂点は lo..hi（同じ時刻の頂点は全部入る）
        let lo = ts.partition_point(|&s| s < a);
        let hi = ts.partition_point(|&s| s <= b);
        let (mut min, mut max) = self.extrema.range(lo, hi.max(lo));
        let mut include = |v: f64| {
            min = min.min(v);
            max = max.max(v);
        };
        // 左端が頂点 lo − 1（a より前で最後の頂点。跳びの後の値）と lo（跳ぶ前の値）の間にある
        if lo > 0 && lo < n && ts[lo] > a {
            include(self.interpolate(lo - 1, lo, a));
        }
        // 右端が頂点 hi − 1 と hi の間にある
        if hi > 0 && hi < n && ts[hi - 1] < b {
            include(self.interpolate(hi - 1, hi, b));
        }
        (min, max)
    }

    /// 頂点 i と j（ts[i] < s < ts[j]）の間を 1 次補間する。
    fn interpolate(&self, i: usize, j: usize, s: f64) -> f64 {
        let (ti, tj) = (self.time[i], self.time[j]);
        let (yi, yj) = (self.values[i], self.values[j]);
        yi + (yj - yi) * ((s - ti) / (tj - ti))
    }
}

#[cfg(test)]
mod tests {
    use globe_mode_ir::VariableRole;

    use super::*;

    fn trajectory(time: Vec<f64>, signals: &[(&str, Vec<f64>)]) -> Trajectory {
        Trajectory {
            time,
            variables: signals
                .iter()
                .map(|(name, values)| TrajectoryVariable {
                    name: (*name).to_string(),
                    role: VariableRole::Algebraic,
                    unit: None,
                    values: values.clone(),
                })
                .collect(),
        }
    }

    /// `f` を [t0, t1] で等間隔に `n` 点取る。
    fn sampled(f: impl Fn(f64) -> f64, t0: f64, t1: f64, n: usize) -> Trajectory {
        let time: Vec<f64> = (0..n)
            .map(|i| t0 + (t1 - t0) * i as f64 / (n - 1) as f64)
            .collect();
        let values = time.iter().map(|&t| f(t)).collect();
        trajectory(time, &[("y", values)])
    }

    fn compare_y(reference: &Trajectory, actual: &Trajectory) -> SignalComparison {
        let c = compare(reference, actual, &["y"], &TubeOptions::default());
        assert_eq!(c.signals.len(), 1);
        c.signals.into_iter().next().unwrap()
    }

    #[test]
    fn identical_curves_pass() {
        let r = sampled(f64::sin, 0.0, 10.0, 1001);
        let s = compare_y(&r, &r);
        assert!(s.passed, "{s:?}");
        assert_eq!((s.checked, s.errors, s.max_excess), (1001, 0, 0.0));
        assert_eq!(s.first_error_time, None);
        assert!(compare(&r, &r, &["y"], &TubeOptions::default()).passed());
    }

    #[test]
    fn time_shift_within_dt_passes_and_beyond_fails() {
        // 区間 4π、δt = 0.002 · 4π ≈ 0.025、値の幅 2 で δy = 0.004。傾きは最大 1 なので、時間の許容が
        // 無ければ 0.015 のずれは落ちる。両端で傾きが 0 の曲線にする（参照の区間の外は窓に入らないため）
        let f = |t: f64| 1.0 - t.cos();
        let end = 4.0 * std::f64::consts::PI;
        let r = sampled(f, 0.0, end, 2001);
        let shifted = |shift: f64| sampled(move |t| f(t - shift), 0.0, end, 1500);
        let s = compare_y(&r, &shifted(0.015));
        assert!(s.passed, "{s:?}");
        let s = compare_y(&r, &shifted(-0.015));
        assert!(s.passed, "{s:?}");
        let s = compare_y(&r, &shifted(0.05));
        assert!(!s.passed);
        assert!(s.errors > 0);
    }

    /// 試験側の 1 点だけを判定し、管の中なら true（区間を覆わないので `passed` は見ない）。
    fn inside(reference: &Trajectory, t: f64, y: f64) -> bool {
        let s = compare_y(reference, &trajectory(vec![t], &[("y", vec![y])]));
        assert_eq!(s.checked, 1);
        s.errors == 0
    }

    #[test]
    fn value_offset_uses_dt_times_slope_plus_dy() {
        // y = t（0〜1）: δt = δy = 0.002。窓の最大は t + δt なので、上の縁は t + 0.004
        let r = trajectory(vec![0.0, 1.0], &[("y", vec![0.0, 1.0])]);
        assert!(inside(&r, 0.5, 0.5 + 0.0039));
        assert!(!inside(&r, 0.5, 0.5 + 0.0041));
        assert!(inside(&r, 0.5, 0.5 - 0.0039));
        assert!(!inside(&r, 0.5, 0.5 - 0.0041));
        // 端では窓が切れる: t = 1 の上の縁は 1 + δy
        assert!(inside(&r, 1.0, 1.0019));
        assert!(!inside(&r, 1.0, 1.0021));
    }

    #[test]
    fn actual_must_cover_the_reference_interval() {
        // 区間 0〜10、δt = 0.02
        let r = sampled(|t| t, 0.0, 10.0, 101);
        // 途中（5）で止まった結果: 点はすべて管の中でも不合格
        let s = compare_y(&r, &sampled(|t| t, 0.0, 5.0, 51));
        assert!(s.uncovered && !s.passed, "{s:?}");
        assert_eq!(s.errors, 0);
        // 遅れて（1 から）始まった結果
        let s = compare_y(&r, &sampled(|t| t, 1.0, 10.0, 91));
        assert!(s.uncovered && !s.passed, "{s:?}");
        // δt より手前で終わる・δt より後に始まるのは覆ったとみなす
        let s = compare_y(&r, &sampled(|t| t, 0.015, 9.985, 101));
        assert!(!s.uncovered && s.passed, "{s:?}");
        let s = compare_y(&r, &sampled(|t| t, 0.0, 9.975, 101));
        assert!(s.uncovered && !s.passed, "{s:?}");
        // 全体を覆う（参照の外まで続く）結果は合格
        let s = compare_y(&r, &sampled(|t| t, -1.0, 11.0, 241));
        assert!(!s.uncovered && s.passed, "{s:?}");
        assert!(
            !compare(
                &r,
                &sampled(|t| t, 0.0, 5.0, 51),
                &["y"],
                &TubeOptions::default()
            )
            .passed()
        );
    }

    #[test]
    fn offset_larger_than_tube_fails_at_the_right_time() {
        let r = sampled(|t| t, 0.0, 1.0, 101);
        let a = sampled(|t| if t >= 0.5 { t + 0.01 } else { t }, 0.0, 1.0, 101);
        let s = compare_y(&r, &a);
        assert!(!s.passed);
        assert_eq!(s.checked, 101);
        assert_eq!(s.errors, 51);
        assert_eq!(s.first_error_time, Some(0.5));
        // はみ出しは 0.01 − (δt · 傾き + δy) = 0.006（終わりの点では窓が切れて 0.008）
        assert!((s.max_excess - 0.008).abs() < 1e-9, "{}", s.max_excess);
    }

    #[test]
    fn event_with_tiny_time_shift_passes() {
        // 0.5 で 0 → 1 に跳ぶ（同じ時刻が 2 行）。δt = 0.002
        let r = trajectory(vec![0.0, 0.5, 0.5, 1.0], &[("y", vec![0.0, 0.0, 1.0, 1.0])]);
        let step = |at: f64| {
            let mut time: Vec<f64> = (0..=1000).map(|i| f64::from(i) / 1000.0).collect();
            let k = time.partition_point(|&t| t < at);
            time.insert(k, at);
            time.insert(k, at);
            let mut values: Vec<f64> = time
                .iter()
                .map(|&t| if t < at { 0.0 } else { 1.0 })
                .collect();
            values[k] = 0.0;
            trajectory(time, &[("y", values)])
        };
        let s = compare_y(&r, &step(0.5005));
        assert!(s.passed, "{s:?}");
        let s = compare_y(&r, &step(0.4985));
        assert!(s.passed, "{s:?}");
        let s = compare_y(&r, &step(0.51));
        assert!(!s.passed);
        // 0.5 + δt より後で、まだ跳んでいない点から落ちる
        let first = s.first_error_time.unwrap();
        assert!(first > 0.502 && first < 0.504, "{first}");
        assert!((s.max_excess - (1.0 - 0.002)).abs() < 1e-9);
    }

    #[test]
    fn constant_reference_uses_the_floors() {
        // 0 の定数: δy = 0.002 · 0.001 = 2e-6
        let r = sampled(|_| 0.0, 0.0, 1.0, 11);
        assert!(compare_y(&r, &sampled(|_| 1.9e-6, 0.0, 1.0, 7)).passed);
        assert!(!compare_y(&r, &sampled(|_| 2.1e-6, 0.0, 1.0, 7)).passed);
        // 100 の定数: δy = 0.002 · |y_min| = 0.2
        let r = sampled(|_| 100.0, 0.0, 1.0, 11);
        assert!(compare_y(&r, &sampled(|_| 100.19, 0.0, 1.0, 7)).passed);
        let s = compare_y(&r, &sampled(|_| 99.79, 0.0, 1.0, 7));
        assert!(!s.passed);
        assert!((s.max_excess - 0.01).abs() < 1e-9);
    }

    #[test]
    fn time_floor_for_a_reference_at_a_single_instant() {
        // 1 点だけの参照（t_0 = t_N = 0）: δt = 0.002 · 1e-12。同じ時刻の点だけを判定する
        let r = trajectory(vec![0.0], &[("y", vec![5.0])]);
        let a = trajectory(vec![0.0, 1e-3], &[("y", vec![5.0, 7.0])]);
        let s = compare_y(&r, &a);
        assert!(s.passed);
        assert_eq!(s.checked, 1);
    }

    #[test]
    fn actual_sampled_at_other_times_and_outside_the_range() {
        // 参照は 0.1 刻み、試験は細かい刻みで真の sin（弦との差は h²/8 = 0.00125 < δy = 0.004）
        let r = sampled(f64::sin, 0.0, 10.0, 101);
        let mut a = sampled(f64::sin, -1.0, 12.0, 997);
        // 参照の区間の外の点は判定しない（外挿しない）
        for (t, y) in a.time.iter().zip(a.variables[0].values.iter_mut()) {
            if *t < 0.0 || *t > 10.0 {
                *y = 1e9;
            }
        }
        let s = compare_y(&r, &a);
        assert!(s.passed, "{s:?}");
        let inside = a
            .time
            .iter()
            .filter(|&&t| (0.0..=10.0).contains(&t))
            .count();
        assert_eq!(s.checked, inside);
        assert!(inside < a.time.len());
    }

    #[test]
    fn missing_signals_do_not_pass() {
        let r = trajectory(
            vec![0.0, 1.0],
            &[("y", vec![0.0, 1.0]), ("z", vec![1.0, 1.0])],
        );
        let a = trajectory(
            vec![0.0, 1.0],
            &[("y", vec![0.0, 1.0]), ("w", vec![0.0, 0.0])],
        );
        let c = compare(&r, &a, &["time", "y", "z", "w"], &TubeOptions::default());
        let summary: Vec<(&str, bool, bool, bool)> = c
            .signals
            .iter()
            .map(|s| (s.name.as_str(), s.passed, s.missing, s.missing_in_reference))
            .collect();
        assert_eq!(
            summary,
            [
                ("y", true, false, false),
                ("z", false, true, false),
                ("w", false, true, true)
            ]
        );
        assert!(!c.passed());
        assert!(!Comparison::default().passed());
    }

    #[test]
    fn nan_in_actual_is_an_error_and_empty_actual_does_not_pass() {
        let r = sampled(|t| t, 0.0, 1.0, 11);
        let s = compare_y(
            &r,
            &sampled(|t| if t > 0.45 { f64::NAN } else { t }, 0.0, 1.0, 11),
        );
        assert!(!s.passed);
        assert_eq!(s.errors, 6);
        assert_eq!(s.max_excess, f64::INFINITY);
        assert_eq!(s.first_error_time, Some(0.5));
        let s = compare_y(&r, &trajectory(vec![], &[("y", vec![])]));
        assert!(!s.passed && !s.missing && s.uncovered);
        assert_eq!(s.checked, 0);
    }

    #[test]
    fn unsorted_reference_is_put_in_order() {
        let r = trajectory(
            vec![1.0, f64::NAN, 0.0, 0.5],
            &[("y", vec![1.0, 100.0, 0.0, 0.5])],
        );
        let a = sampled(|t| t, 0.0, 1.0, 21);
        let s = compare_y(&r, &a);
        assert!(s.passed, "{s:?}");
    }

    #[test]
    fn invalid_tolerance_means_exact() {
        let r = sampled(|t| t, 0.0, 1.0, 11);
        let a = sampled(|t| t + 1e-9, 0.0, 1.0, 11);
        for tolerance in [f64::NAN, -1.0, 0.0] {
            let c = compare(&r, &a, &["y"], &TubeOptions { tolerance });
            assert!(!c.passed(), "{tolerance}");
        }
        assert!(compare(&r, &r, &["y"], &TubeOptions { tolerance: 0.0 }).passed());
    }

    /// 窓の最大・最小を、線分ごとに窓で切って両端を比べる素朴な方法（O(n)）と突き合わせる。
    #[test]
    fn window_extrema_match_a_brute_force() {
        // 決まった種の線形合同法で、跳び（同じ時刻）を含む折れ線を作る
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut time = vec![0.0];
        let mut values = vec![next()];
        for _ in 0..400 {
            let dt = if next() < 0.1 { 0.0 } else { next() * 0.01 };
            time.push(time.last().unwrap() + dt);
            values.push(next() * 2.0 - 1.0);
        }
        let mut extrema = Extrema::default();
        extrema.rebuild(&values);
        let tube = Tube::new(&time, &values, &extrema, 0.01).unwrap();
        let (t0, tn) = (tube.t0, tube.tn);
        for k in 0..=2000 {
            let t = t0 + (tn - t0) * f64::from(k) / 2000.0;
            let (a, b) = ((t - tube.dt).max(t0), (t + tube.dt).min(tn));
            let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
            for i in 0..time.len() - 1 {
                let (ti, tj) = (time[i], time[i + 1]);
                let (lo, hi) = (a.max(ti), b.min(tj));
                if lo > hi {
                    continue;
                }
                let at = |s: f64| {
                    if tj == ti {
                        None
                    } else {
                        Some(values[i] + (values[i + 1] - values[i]) * ((s - ti) / (tj - ti)))
                    }
                };
                let ends = match (at(lo), at(hi)) {
                    (Some(p), Some(q)) => [p, q],
                    // 縦の線分: 両方の頂点の値
                    _ => [values[i], values[i + 1]],
                };
                for v in ends {
                    min = min.min(v);
                    max = max.max(v);
                }
            }
            let (wmin, wmax) = tube.window(t);
            assert!(
                (wmin - min).abs() < 1e-12 && (wmax - max).abs() < 1e-12,
                "t = {t}: ({wmin}, {wmax}) と ({min}, {max})"
            );
        }
    }

    #[test]
    fn parses_comparison_signals() {
        let text = "\u{feff}time\r\n  x  \n\nder(y)\na[1,2]\n";
        assert_eq!(parse_comparison_signals(text), ["x", "der(y)", "a[1,2]"]);
        assert!(parse_comparison_signals("").is_empty());
        // 比べないことにした信号は `//` で書き残してある（MSL の HoldWithDAeffects1 など）
        assert_eq!(
            parse_comparison_signals("time\nx\n// shiftSample1.y\n  //y\n"),
            ["x"]
        );
    }
}
