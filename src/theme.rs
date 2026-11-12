//! 浅色主题，按 Nexus Windows 端截图取色。
//! 只做浅色主题；主色可在设置页 6 个预设里切换。

use std::sync::atomic::{AtomicU32, Ordering};

/// 当前主色（可由设置页切换）。默认蓝 #1e6fb8。
static PRIMARY: AtomicU32 = AtomicU32::new(0x1e6fb8);

pub fn primary() -> u32 {
    PRIMARY.load(Ordering::Relaxed)
}
pub fn set_primary(c: u32) {
    PRIMARY.store(c, Ordering::Relaxed);
}

/// 设置页 6 个预设主题色。
pub const THEME_COLORS: [u32; 6] = [
    0x3498db, // 蓝
    0x2ecc71, // 绿
    0xe74c3c, // 红
    0x9b59b6, // 紫
    0xf39c12, // 橙
    0x1abc9c, // 青
];

// ---- 色板 ----
pub fn bg() -> u32 {
    0xffffff
}
/// 导航/侧栏底
pub fn nav_bg() -> u32 {
    0xffffff
}
/// 卡片、工具条、迷你栏底
pub fn surface() -> u32 {
    0xf4f7fa
}
/// 更深的表面（hover、选中行）
pub fn surface_alt() -> u32 {
    0xeaf1f9
}
/// 选中态浅蓝底
pub fn selected() -> u32 {
    0xdcebf9
}
pub fn border() -> u32 {
    0xe3e9f0
}
pub fn text() -> u32 {
    0x2b3542
}
pub fn text_dim() -> u32 {
    0x8a94a2
}
pub fn text_faint() -> u32 {
    0xaab3bf
}
pub fn hover() -> u32 {
    0xeef3f9
}
/// 主色上的文字
pub fn on_primary() -> u32 {
    0xffffff
}
/// 主色加深（按钮 pressed / 强调）
pub fn primary_deep() -> u32 {
    darken(primary(), 0.82)
}
pub fn danger() -> u32 {
    0xe0483f
}
pub fn live() -> u32 {
    0xe53935
}
pub fn green_bg() -> u32 {
    0xe4f4ea
}
pub fn green_fg() -> u32 {
    0x2e9e5b
}
pub fn badge_bg() -> u32 {
    0x5a94c9
}
/// 播放页纯黑画面区
pub fn player_black() -> u32 {
    0x000000
}
/// 卡片占位缩略图底色
pub fn placeholder() -> u32 {
    0xdfe7ef
}

pub fn darken(c: u32, f: f64) -> u32 {
    let r = (((c >> 16) & 0xff) as f64 * f) as u32;
    let g = (((c >> 8) & 0xff) as f64 * f) as u32;
    let b = ((c & 0xff) as f64 * f) as u32;
    (r << 16) | (g << 8) | b
}
pub fn mix(c: u32, other: u32, t: f64) -> u32 {
    let ch = |s: u32, o: u32| -> u32 {
        (((s as f64) * (1.0 - t) + (o as f64) * t) as u32).min(255)
    };
    let r = ch((c >> 16) & 0xff, (other >> 16) & 0xff);
    let g = ch((c >> 8) & 0xff, (other >> 8) & 0xff);
    let b = ch(c & 0xff, other & 0xff);
    (r << 16) | (g << 8) | b
}

// ---- 尺寸 ----
pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 800.0;
pub const WINDOW_MIN_W: f32 = 900.0;
pub const WINDOW_MIN_H: f32 = 600.0;

pub const NAV_W: f32 = 72.0;
pub const MINI_H: f32 = 64.0;
pub const SUBNAV_W: f32 = 200.0;
pub const PLAYLIST_W: f32 = 260.0;
pub const IPTV_PANEL_W: f32 = 340.0;
pub const IPTV_GROUP_W: f32 = 76.0;
pub const IPTV_SOURCE_W: f32 = 62.0;
pub const COVER_PANEL_W: f32 = 300.0;
pub const QUEUE_W: f32 = 300.0;
pub const SEARCH_W: f32 = 220.0;
pub const SEARCH_H: f32 = 34.0;
pub const CARD_W: f32 = 232.0;
pub const CARD_H: f32 = 145.0; // 16:10
pub const IPTV_CARD_W: f32 = 300.0;
pub const IPTV_CARD_H: f32 = 94.0; // 3.2
pub const THUMB_H: f32 = 44.0;

pub const FONT: &str = "Microsoft YaHei";

// ---- 图标（用字符近似 Material 线性图标，避免额外资源依赖）----
pub mod icon {
    pub const VIDEO: &str = "▣";
    pub const IPTV: &str = "▤";
    pub const MUSIC: &str = "♫";
    pub const SETTINGS: &str = "⚙";
    pub const SEARCH: &str = "⌕";
    pub const REFRESH: &str = "⟳";
    pub const SORT: &str = "≡";
    pub const FOLDER_ADD: &str = "+";
    pub const FOLDER: &str = "▢";
    pub const CAST: &str = "◧";
    pub const BACK: &str = "←";
    pub const PLAY: &str = "▶";
    pub const PAUSE: &str = "❚❚";
    pub const PREV: &str = "⏮";
    pub const NEXT: &str = "⏭";
    pub const REPLAY: &str = "↺";
    pub const FORWARD: &str = "↻";
    pub const VOLUME: &str = "◂))";
    pub const MUTE: &str = "◂×";
    pub const FULLSCREEN: &str = "⛶";
    pub const EXIT_FULLSCREEN: &str = "⤡";
    pub const REPEAT: &str = "↻";
    pub const REPEAT_ONE: &str = "①";
    pub const SHUFFLE: &str = "⇄";
    pub const QUEUE: &str = "☰";
    pub const DOWNLOAD: &str = "⇩";
    pub const ALBUM: &str = "◉";
    pub const ARTIST: &str = "☺";
    pub const CHEVRON: &str = "›";
    pub const CLOSE: &str = "×";
    pub const EDIT: &str = "✎";
    pub const DELETE: &str = "⌫";
    pub const NETWORK: &str = "◎";
    pub const FILE: &str = "▤";
    pub const CHECK: &str = "✓";
    pub const LINK: &str = "⚯";
    pub const LIVE_DOT: &str = "●";
    pub const LYRICS: &str = "♪";
    pub const NOTE: &str = "♫";
    pub const CLOCK: &str = "◷";
}
