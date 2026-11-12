//! Nexus 风格播放器（Rust + GPUI + libmpv）。
//!
//! 默认使用 crates.io 的 gpui 0.2.2；如需切回 nyaterm 的 zed fork，见 README。
//!
//! - 4 页导航：视频 / IPTV / 音乐 / 设置
//! - 3 个播放页 + 底部常驻迷你播放栏
//! - 画面由 libmpv SW render 取帧，转 RenderImage 后用 img() 显示
//! - 仅 MPV 后端，支持硬解(auto-copy)/软解(no) 切换

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod app;
mod iptv;
mod library;
mod media;
mod music;
mod pages;
mod player;
mod players;
mod settings;
mod theme;
mod widgets;

use anyhow::Result;
use app::{AppView, APP_TITLE};
use gpui::prelude::*;
use gpui::{
    px, size, App, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use theme::{WINDOW_H, WINDOW_MIN_H, WINDOW_MIN_W, WINDOW_W};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    gpui::Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_W), px(WINDOW_H)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(APP_TITLE.into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                window_min_size: Some(size(px(WINDOW_MIN_W), px(WINDOW_MIN_H))),
                ..Default::default()
            },
            |_, cx| cx.new(AppView::new),
        )
        .expect("open window");
        cx.activate(true);
    });

    Ok(())
}
