//! stream 変数の接続集合と `inStream`・`actualStream` の展開（MLS 15.2〜15.3。S355）。
//!
//! 平らにし終わったあとで、すべての式の印字の中の `inStream(x)`・`actualStream(x)` と、外側のコネクタの端の方程式 `x = __outStream(x)` を、
//! 接続集合の混合の式に置き換える（Rumoca の flat と同じ形。数は方程式の本数だけが比べられる）。
//!
//! - 集合の端は、内側のコネクタ（部品のコネクタ）なら `x` の値、外側のコネクタ（そのモデル自身のコネクタ）なら `inStream(x)`（1 段上の集合で決まる）を出す。
//! - `inStream(x)` は、`x` が内側の端になっている集合の、`x` を除いた端の混合。端が 0 なら `x` 自身、1 つならその端の値、2 つ以上なら
//!   `Σ max(±流量, ε)·値 / Σ max(±流量, ε)`（内側の端は `-m_flow`、外側の端は `m_flow`。`ε` は集合の流量の nominal の最小の `1e-7` 倍）。
//! - 流量が出ていく向きにだけ流れる端（内側で `min >= 0`、外側で `max <= 0` が数で書いてある）は、混合に入れない。
//! - 外側のコネクタの端の方程式 `x = …` は、`x` が外側の端になっている集合の、`x` を除いた端の混合（`inStream(x)` ではない）。
//!
//! shortcut: Rumoca の `smooth(0, m_flow * actualStream(x))`（流量の重みつきの積）は作らない。`min`・`max` は数のリテラルだけ読む（Rumoca と同じ）。
//! 添字が定数でない `inStream(ports[i].h)` と、流量が 1 つに決まらないコネクタは断る（`stream`）。

use super::F;
use crate::{Res, un};
use std::collections::HashMap;

/// 外側のコネクタの端の方程式の右辺の目印（`expand_streams` が置き換える）
pub(super) const OUT_STREAM: &str = "__outStream";

/// 再帰（外側の端 → 1 段上の集合）の上限。階層の深さを超えれば自己参照
const MAX_DEPTH: u32 = 64;

struct Sx {
    sets: Vec<Vec<(String, bool)>>,
    /// 名前 → 内側の端になっている集合・外側の端になっている集合
    inside: HashMap<String, usize>,
    outside: HashMap<String, usize>,
}

fn is_id(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `s` を数として読む（`0`・`-1.5`・`1e-3`）。式なら `None`
fn literal(s: &str) -> Option<f64> {
    let t = s.trim();
    t.chars()
        .all(|c| c.is_ascii_digit() || "+-.eE".contains(c))
        .then(|| t.parse().ok())
        .flatten()
}

impl F<'_> {
    pub(super) fn expand_streams(&mut self) -> Res<()> {
        let mut sx = Sx {
            sets: std::mem::take(&mut self.stream_sets),
            inside: HashMap::new(),
            outside: HashMap::new(),
        };
        for (k, set) in sx.sets.iter().enumerate() {
            for (n, out) in set {
                let m = if *out {
                    &mut sx.outside
                } else {
                    &mut sx.inside
                };
                m.insert(n.clone(), k);
            }
        }
        // 方程式の列は取り出して書き換える（`rw` は変数の表だけ読む）
        let mut lists = [
            std::mem::take(&mut self.eqs),
            std::mem::take(&mut self.whens),
            std::mem::take(&mut self.ieqs),
        ];
        for e in lists.iter_mut().flatten() {
            if let Some(t) = self.rw(&sx, &e.text)? {
                e.text = t;
            }
        }
        let [eqs, whens, ieqs] = lists;
        (self.eqs, self.whens, self.ieqs) = (eqs, whens, ieqs);
        for i in 0..self.algs.len() {
            if let Some(t) = self.rw(&sx, &self.algs[i])? {
                self.algs[i] = t;
            }
        }
        for i in 0..self.vars.len() {
            let v = &self.vars[i];
            let binding = v.binding.as_deref().map(|s| self.rw(&sx, s)).transpose()?;
            let start = v.start.as_deref().map(|s| self.rw(&sx, s)).transpose()?;
            let mut attrs = Vec::new();
            for (k, (_, a)) in v.attrs.iter().enumerate() {
                if let Some(t) = self.rw(&sx, a)? {
                    attrs.push((k, t));
                }
            }
            let v = &mut self.vars[i];
            if let Some(Some(t)) = binding {
                v.binding = Some(t);
            }
            if let Some(Some(t)) = start {
                v.start = Some(t);
            }
            for (k, t) in attrs {
                v.attrs[k].1 = t;
            }
        }
        Ok(())
    }

    /// 印字の中の `inStream(x)`・`actualStream(x)`・`__outStream(x)` を置き換える。無ければ `None`
    fn rw(&self, sx: &Sx, s: &str) -> Res<Option<String>> {
        if !["inStream", "actualStream", OUT_STREAM]
            .iter()
            .any(|k| s.contains(k))
        {
            return Ok(None);
        }
        let b = s.as_bytes();
        let (mut out, mut i, mut last) = (String::new(), 0, 0);
        while i < b.len() {
            if b[i] == b'"' {
                // 文字列の中は読まない
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
                continue;
            }
            if !(b[i].is_ascii_alphabetic() || b[i] == b'_')
                || (i > 0 && (is_id(b[i - 1]) || b[i - 1] == b'.'))
            {
                i += 1;
                continue;
            }
            let mut j = i;
            while j < b.len() && is_id(b[j]) {
                j += 1;
            }
            let word = &s[i..j];
            let mut k = j;
            while k < b.len() && b[k] == b' ' {
                k += 1;
            }
            if !matches!(word, "inStream" | "actualStream" | OUT_STREAM) || b.get(k) != Some(&b'(')
            {
                i = j;
                continue;
            }
            let (mut depth, mut e) = (0, k);
            loop {
                match b.get(e) {
                    Some(b'(') => depth += 1,
                    Some(b')') => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Some(_) => {}
                    None => return un("stream", format!("unbalanced {word}( in {s}")),
                }
                e += 1;
            }
            out += &s[last..i];
            out += &self.stream_call(sx, word, s[k + 1..e].trim(), 0)?;
            (i, last) = (e + 1, e + 1);
        }
        out += &s[last..];
        Ok(Some(out))
    }

    /// `op(arg)` の置き換え。`arg` は stream 変数（配列なら要素 `x[2]`）
    fn stream_call(&self, sx: &Sx, op: &str, arg: &str, depth: u32) -> Res<String> {
        let (base, sfx) = match arg.rfind('[') {
            Some(p) if arg.ends_with(']') && !self.var_idx.contains_key(arg) => arg.split_at(p),
            _ => (arg, ""),
        };
        let Some(&vi) = self.var_idx.get(base) else {
            return un("stream", format!("{op}({arg}): not a variable"));
        };
        let v = &self.vars[vi];
        if !v.stream {
            return un("stream", format!("{op}({arg}): not a stream variable"));
        }
        if v.dims.contains(&0) {
            return Ok("{}".to_string());
        }
        if op == OUT_STREAM {
            return self.mix(sx, sx.outside.get(base).copied(), base, sfx, depth);
        }
        let inflow = self.mix(sx, sx.inside.get(base).copied(), base, sfx, depth)?;
        if op == "inStream" {
            return Ok(inflow);
        }
        let flow = self.flow_of(base)?;
        Ok(format!(
            "(if {} > 0 then {inflow} else {base}{sfx})",
            self.vars[flow].name
        ))
    }

    /// 集合 `set` の、`name` を除いた端の混合（`inStream` の定義。端が無ければ自分）
    fn mix(&self, sx: &Sx, set: Option<usize>, name: &str, sfx: &str, depth: u32) -> Res<String> {
        let Some(k) = set else {
            return Ok(format!("{name}{sfx}"));
        };
        if depth > MAX_DEPTH {
            return un("stream", format!("{name}: stream sets refer to themselves"));
        }
        let mut eps = f64::INFINITY;
        let mut peers = Vec::new();
        for (n, out) in &sx.sets[k] {
            let fv = &self.vars[self.flow_of(n)?];
            let attr = |a: &str| {
                fv.attrs
                    .iter()
                    .find(|(k, _)| k == a)
                    .and_then(|(_, t)| literal(t))
            };
            let nominal = attr("nominal").filter(|x| x.is_finite() && *x > 0.0);
            eps = eps.min(1e-7 * nominal.unwrap_or(1.0));
            // 流量が集合へ入る向きにならない端は、混合に入れない
            let supplies = if *out {
                !attr("max").is_some_and(|m| m <= 0.0)
            } else {
                !attr("min").is_some_and(|m| m >= 0.0)
            };
            if n != name && supplies {
                peers.push((n, *out, fv.name.as_str()));
            }
        }
        let value = |n: &str, out: bool| -> Res<String> {
            if out {
                // 外側の端は 1 段上の集合が決める
                self.mix(sx, sx.inside.get(n).copied(), n, sfx, depth + 1)
            } else {
                Ok(format!("{n}{sfx}"))
            }
        };
        let weight =
            |flow: &str, out: bool| format!("max({}{flow}, {eps:e})", if out { "" } else { "-" });
        Ok(match peers.as_slice() {
            [] => format!("{name}{sfx}"),
            [(n, out, _)] => value(n, *out)?,
            _ => {
                let (mut num, mut den) = (Vec::new(), Vec::new());
                for (n, out, flow) in &peers {
                    let w = weight(flow, *out);
                    num.push(format!("{w} * {}", value(n, *out)?));
                    den.push(w);
                }
                format!("(({}) / ({}))", num.join(" + "), den.join(" + "))
            }
        })
    }

    /// stream 変数 `n` と同じコネクタの流量の変数（ちょうど 1 つ。Rumoca と同じ）の番号
    fn flow_of(&self, n: &str) -> Res<usize> {
        let Some((parent, _)) = n.rsplit_once('.') else {
            return un("stream", format!("{n}: no connector"));
        };
        let pre = format!("{parent}.");
        let mut flows = self
            .var_idx
            .range(pre.clone()..)
            .take_while(|(k, _)| k.starts_with(&pre))
            .filter(|(k, i)| self.vars[**i].flow && !k[pre.len()..].contains('.'))
            .map(|(_, &i)| i);
        match (flows.next(), flows.next()) {
            (Some(i), None) => Ok(i),
            _ => un(
                "stream",
                format!("{n}: needs exactly one flow variable in {parent}"),
            ),
        }
    }
}
