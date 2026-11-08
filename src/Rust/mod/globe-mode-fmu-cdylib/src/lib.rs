//! `globe-mode-fmu` の共有ライブラリ（S240）。FMI の `extern "C"` の入口（`fmi2*`・`fmi3*`）は `globe-mode-fmu` にあり、
//! ここは cdylib にそれを載せるだけ（`pub use` で、リンカが入口を落とさない）。FMU の `binaries/<os>/globe_mode_fmu.<拡張子>` に入れる。
pub use globe_mode_fmu::*;
