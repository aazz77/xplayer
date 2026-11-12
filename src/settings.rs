//! 应用设置持久化（JSON）。只使用 mpv，不含任何 VLC/libVLC 后端。
//!
//! - 硬解 / 软解：映射到 mpv `hwdec`（auto-copy / no）
//! - 画质预设：映射到 mpv 的一组滤镜/缩放参数

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DecodeMode {
    /// GPU 硬解，mpv hwdec=auto-copy（拷贝回内存，保证 SW 取帧可见）
    #[default]
    Hardware,
    /// 纯 CPU 软解，mpv hwdec=no
    Software,
}

impl DecodeMode {
    pub fn label(self) -> &'static str {
        match self {
            DecodeMode::Hardware => "硬解",
            DecodeMode::Software => "软解",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            DecodeMode::Hardware => "硬解 — GPU 解码，降低 CPU 占用（帧回拷内存）",
            DecodeMode::Software => "软解 — CPU 解码，兼容性最好",
        }
    }
    /// mpv `hwdec` 属性值。auto-copy 强制把解码帧拷回系统内存，
    /// 这样 vo=libmpv 的 SW render API 才能稳定取到帧。
    pub fn hwdec(self) -> &'static str {
        match self {
            DecodeMode::Hardware => "auto-copy",
            DecodeMode::Software => "no",
        }
    }
    pub fn next(self) -> Self {
        match self {
            DecodeMode::Hardware => DecodeMode::Software,
            DecodeMode::Software => DecodeMode::Hardware,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Quality {
    Performance,
    #[default]
    Balanced,
    Quality,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Quality::Performance => "性能优先",
            Quality::Balanced => "均衡",
            Quality::Quality => "画质优先",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Quality::Performance => "性能优先 — 关闭滤镜，最低开销",
            Quality::Balanced => "均衡 — 画质与性能平衡（推荐）",
            Quality::Quality => "画质优先 — 高质量缩放与去抖",
        }
    }
    /// 返回要写进 mpv 的 (property, value) 列表
    pub fn props(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Quality::Performance => &[
                ("scale", "bilinear"),
                ("cscale", "bilinear"),
                ("dscale", "bilinear"),
                ("interpolation", "no"),
                ("deband", "no"),
            ],
            Quality::Balanced => &[
                ("scale", "bilinear"),
                ("cscale", "bilinear"),
                ("dscale", "bilinear"),
                ("interpolation", "no"),
                ("deband", "no"),
            ],
            Quality::Quality => &[
                ("scale", "lanczos"),
                ("cscale", "lanczos"),
                ("dscale", "mitchell"),
                ("interpolation", "yes"),
                ("deband", "yes"),
            ],
        }
    }
    pub fn next(self) -> Self {
        match self {
            Quality::Performance => Quality::Balanced,
            Quality::Balanced => Quality::Quality,
            Quality::Quality => Quality::Performance,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 主题色索引（0..6）
    pub theme_index: usize,
    /// 硬解/软解
    pub decode: DecodeMode,
    /// 画质预设
    pub quality: Quality,
    /// 音量 0..150
    pub volume: i64,
    /// 默认倍速
    pub rate: f64,
    /// 记住上次导航页（0 视频 / 1 IPTV / 2 音乐 / 3 设置）
    pub last_nav: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_index: 0,
            decode: DecodeMode::default(),
            quality: Quality::default(),
            volume: 100,
            rate: 1.0,
            last_nav: 0,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let p = path();
        if let Ok(t) = fs::read_to_string(&p) {
            if let Ok(s) = serde_json::from_str::<Settings>(&t) {
                return s;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(dir) = path().parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path(), t);
        }
    }

    pub fn theme_color(&self) -> u32 {
        crate::theme::THEME_COLORS
            .get(self.theme_index)
            .copied()
            .unwrap_or(crate::theme::THEME_COLORS[0])
    }
}

fn path() -> PathBuf {
    data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xplayer")
        .join("settings.json")
}

fn data_local_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
    }
}
