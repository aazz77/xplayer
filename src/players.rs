//! 3 个播放页：视频 / IPTV / 音乐。

use gpui::*;

use crate::app::{AppView, Dropdown};
use crate::{theme, widgets};

// ============================ 视频播放页 ============================

impl AppView {
    pub(crate) fn render_video_player(&mut self, entity: Entity<Self>) -> AnyElement {
        let st = &self.state;
        let title = if st.title.is_empty() { "播放中".to_string() } else { st.title.clone() };
        let pos = Self::fmt_time(st.position);
        let dur = Self::fmt_time(st.duration);
        let paused = st.paused;
        let ratio = self.ratio();
        let mute = st.mute;
        let rate_label = format!("{:.2}x", self.rate);
        let show_list = self.show_playlist && !self.fullscreen;
        let frame = self.current_frame.clone();
        let has_frame = frame.is_some();

        // ---- 画面区 ----
        let mut stage = div()
            .id("video-stage")
            .relative()
            .flex_1()
            .h_full()
            .bg(rgb(theme::player_black()))
            .flex()
            .items_center()
            .justify_center();

        if let Some(ri) = frame {
            stage = stage.child(
                img(ri)
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            );
        } else {
            stage = stage.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(40.))
                            .text_color(rgb(0x555555))
                            .child(theme::icon::VIDEO),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x888888))
                            .child(if has_frame { "缓冲中..." } else { "等待视频帧" }),
                    ),
            );
        }

        // 顶部栏（半透明黑）
        stage = stage.child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .w_full()
                .h(px(56.))
                .flex()
                .items_center()
                .px_4()
                .gap_3()
                .bg(rgb(0x000000))
                .opacity(0.55)
                .child(widgets::icon_btn(theme::icon::BACK, {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.go_browse();
                            cx.notify();
                        });
                    }
                }))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.))
                        .text_color(rgb(0xffffff))
                        .overflow_hidden()
                        .child(SharedString::from(title)),
                ),
        );

        // 底部控制条
        let e_seek = entity.clone();
        let bar = widgets::player_bar("vprog", ratio, 40, move |i, _w, cx| {
            let _ = e_seek.update(cx, |v, cx| {
                if v.state.duration > 0.0 {
                    let t = v.state.duration * ((i + 1) as f64 / 40.0);
                    let _ = v.player.seek_absolute(t);
                }
                cx.notify();
            });
        });

        let e_play = entity.clone();
        let e_back10 = entity.clone();
        let e_fwd10 = entity.clone();
        let e_vol_dn = entity.clone();
        let e_vol_up = entity.clone();
        let e_mute = entity.clone();
        let e_rate = entity.clone();
        let e_fs = entity.clone();
        let e_prev = entity.clone();
        let e_next = entity.clone();

        let controls = div()
            .absolute()
            .bottom(px(0.))
            .left(px(0.))
            .w_full()
            .px_4()
            .pb_3()
            .pt_2()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0x000000))
            .opacity(0.6)
            .child(bar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0xdddddd))
                            .child(SharedString::from(format!("{pos} / {dur}"))),
                    )
                    .child(div().flex_1())
                    .child(player_icon_btn(theme::icon::PREV, "上一项", move |_, _, cx| {
                        let _ = e_prev.update(cx, |v, cx| {
                            v.prev_playlist();
                            cx.notify();
                        });
                    }))
                    .child(player_icon_btn(
                        theme::icon::REPLAY,
                        "-10s",
                        move |_, _, cx| {
                            let _ = e_back10.update(cx, |v, cx| {
                                v.seek_rel(-10.0);
                                cx.notify();
                            });
                        },
                    ))
                    .child(player_icon_btn(
                        if paused { theme::icon::PLAY } else { theme::icon::PAUSE },
                        "播放/暂停",
                        move |_, _, cx| {
                            let _ = e_play.update(cx, |v, cx| {
                                v.pause_toggle();
                                cx.notify();
                            });
                        },
                    ))
                    .child(player_icon_btn(
                        theme::icon::FORWARD,
                        "+10s",
                        move |_, _, cx| {
                            let _ = e_fwd10.update(cx, |v, cx| {
                                v.seek_rel(10.0);
                                cx.notify();
                            });
                        },
                    ))
                    .child(player_icon_btn(theme::icon::NEXT, "下一项", move |_, _, cx| {
                        let _ = e_next.update(cx, |v, cx| {
                            v.next_playlist();
                            cx.notify();
                        });
                    }))
                    .child(div().w(px(8.)))
                    .child(player_icon_btn(
                        if mute { theme::icon::MUTE } else { theme::icon::VOLUME },
                        "静音",
                        move |_, _, cx| {
                            let _ = e_mute.update(cx, |v, cx| {
                                let _ = v.player.toggle_mute();
                                cx.notify();
                            });
                        },
                    ))
                    .child(player_icon_btn("−", "音量-", move |_, _, cx| {
                        let _ = e_vol_dn.update(cx, |v, cx| {
                            v.change_volume(-5);
                            cx.notify();
                        });
                    }))
                    .child(player_icon_btn("+", "音量+", move |_, _, cx| {
                        let _ = e_vol_up.update(cx, |v, cx| {
                            v.change_volume(5);
                            cx.notify();
                        });
                    }))
                    .child(div().w(px(8.)))
                    .child(widgets::text_btn(&rate_label, move |_, _, cx| {
                        let _ = e_rate.update(cx, |v, cx| {
                            v.dropdown = if v.dropdown == Some(Dropdown::Rate) {
                                None
                            } else {
                                Some(Dropdown::Rate)
                            };
                            cx.notify();
                        });
                    }))
                    .child(player_icon_btn(
                        theme::icon::FULLSCREEN,
                        "全屏",
                        move |_, _, cx| {
                            let _ = e_fs.update(cx, |v, cx| {
                                v.fullscreen = !v.fullscreen;
                                cx.notify();
                            });
                        },
                    )),
            );

        stage = stage.child(controls);

        let mut row = div().flex().flex_1().h_full().child(stage);

        if show_list {
            row = row
                .child(widgets::v_line())
                .child(self.render_video_playlist(entity.clone()));
        }

        row.into_any_element()
    }

    fn render_video_playlist(&self, entity: Entity<Self>) -> AnyElement {
        let mut col = div()
            .id("vplaylist")
            .w(px(theme::PLAYLIST_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(theme::bg()));

        col = col.child(
            div()
                .w_full()
                .h(px(48.))
                .flex_none()
                .flex()
                .items_center()
                .px_4()
                .border_b_1()
                .border_color(rgb(theme::border()))
                .text_size(px(14.))
                .text_color(rgb(theme::text()))
                .child("播放列表"),
        );

        if self.playlist.is_empty() {
            col = col.child(widgets::empty_state(theme::icon::QUEUE, "无播放列表", "从视频库点击播放", None));
        } else {
            let mut list = div()
                .id("vplaylist-items")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for (i, f) in self.playlist.iter().take(400).enumerate() {
                let sel = i == self.playlist_idx;
                let ext = f
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_uppercase();
                let e = entity.clone();
                let f2 = f.clone();
                list = list.child(
                    div()
                        .id(SharedString::from(format!("vpl-{i}")))
                        .w_full()
                        .h(px(44.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .px_3()
                        .gap_2()
                        .bg(rgb(if sel { theme::selected() } else { theme::bg() }))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::hover())))
                        .child(
                            div()
                                .w(px(20.))
                                .text_size(px(11.))
                                .text_color(rgb(if sel { theme::primary() } else { theme::text_faint() }))
                                .child(if sel { theme::icon::PLAY } else { "" }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(rgb(if sel { theme::primary() } else { theme::text() }))
                                .overflow_hidden()
                                .child(SharedString::from(f.name.clone())),
                        )
                        .child(widgets::soft_badge(&ext))
                        .on_click(move |_, _, cx| {
                            let _ = e.update(cx, |v, cx| {
                                v.playlist_idx = i;
                                let _ = v.player.load(&f2.path);
                                cx.notify();
                            });
                        }),
                );
            }
            col = col.child(list);
        }
        col.into_any_element()
    }
}

fn player_icon_btn(
    glyph: impl Into<SharedString>,
    tag: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let g: SharedString = glyph.into();
    div()
        .id(SharedString::from(format!("pib-{tag}")))
        .w(px(32.))
        .h(px(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(14.))
        .text_color(rgb(0xeeeeee))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x444444)))
        .child(g)
        .on_click(on_click)
        .into_any_element()
}

// ============================ IPTV 播放页 ============================

impl AppView {
    pub(crate) fn render_iptv_player(&mut self, entity: Entity<Self>) -> AnyElement {
        let title = self.iptv_title.clone();
        let paused = self.state.paused;
        let mute = self.state.mute;
        let frame = self.current_frame.clone();
        let show_panel = !self.fullscreen;

        let mut stage = div()
            .id("iptv-stage")
            .relative()
            .flex_1()
            .h_full()
            .bg(rgb(theme::player_black()))
            .flex()
            .items_center()
            .justify_center();
        if let Some(ri) = frame {
            stage = stage.child(img(ri).size_full().object_fit(ObjectFit::Contain));
        } else {
            stage = stage.child(
                div()
                    .text_size(px(40.))
                    .text_color(rgb(0x555555))
                    .child(theme::icon::IPTV),
            );
        }

        let e_back = entity.clone();
        stage = stage.child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .w_full()
                .h(px(56.))
                .flex()
                .items_center()
                .px_4()
                .gap_3()
                .bg(rgb(0x000000))
                .opacity(0.55)
                .child(widgets::icon_btn(theme::icon::BACK, move |_, _, cx| {
                    let _ = e_back.update(cx, |v, cx| {
                        v.go_browse();
                        cx.notify();
                    });
                }))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(15.))
                        .text_color(rgb(0xffffff))
                        .overflow_hidden()
                        .child(SharedString::from(title)),
                )
                .child(widgets::live_badge()),
        );

        let e_play = entity.clone();
        let e_mute = entity.clone();
        let e_vdn = entity.clone();
        let e_vup = entity.clone();
        let e_fs = entity.clone();
        stage = stage.child(
            div()
                .absolute()
                .bottom(px(0.))
                .left(px(0.))
                .w_full()
                .h(px(52.))
                .flex()
                .items_center()
                .px_4()
                .gap_2()
                .bg(rgb(0x000000))
                .opacity(0.6)
                .child(player_icon_btn(
                    if paused { theme::icon::PLAY } else { theme::icon::PAUSE },
                    "iptv-play",
                    move |_, _, cx| {
                        let _ = e_play.update(cx, |v, cx| {
                            v.pause_toggle();
                            cx.notify();
                        });
                    },
                ))
                .child(player_icon_btn(
                    if mute { theme::icon::MUTE } else { theme::icon::VOLUME },
                    "iptv-mute",
                    move |_, _, cx| {
                        let _ = e_mute.update(cx, |v, cx| {
                            let _ = v.player.toggle_mute();
                            cx.notify();
                        });
                    },
                ))
                .child(player_icon_btn("−", "iptv-vdn", move |_, _, cx| {
                    let _ = e_vdn.update(cx, |v, cx| {
                        v.change_volume(-5);
                        cx.notify();
                    });
                }))
                .child(player_icon_btn("+", "iptv-vup", move |_, _, cx| {
                    let _ = e_vup.update(cx, |v, cx| {
                        v.change_volume(5);
                        cx.notify();
                    });
                }))
                .child(div().flex_1())
                .child(player_icon_btn(
                    theme::icon::FULLSCREEN,
                    "iptv-fs",
                    move |_, _, cx| {
                        let _ = e_fs.update(cx, |v, cx| {
                            v.fullscreen = !v.fullscreen;
                            cx.notify();
                        });
                    },
                )),
        );

        let mut row = div().flex().flex_1().h_full().child(stage);
        if show_panel {
            row = row
                .child(widgets::v_line())
                .child(self.render_iptv_channel_panel(entity.clone()));
        }
        row.into_any_element()
    }

    fn render_iptv_channel_panel(&self, entity: Entity<Self>) -> AnyElement {
        let groups = self.iptv.playlist.group_names();
        let cur_group = self.iptv_group.clone();
        let channels = self.iptv_channels.clone();
        let ch_idx = self.iptv_ch_idx;
        let src_idx = self.iptv_src_idx;

        // 分组列
        let mut gcol = div()
            .id("iptv-groups")
            .w(px(theme::IPTV_GROUP_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(theme::border()))
            .overflow_y_scroll();
        for (i, g) in groups.iter().enumerate() {
            let sel = *g == cur_group;
            let e = entity.clone();
            let gname = g.clone();
            gcol = gcol.child(
                div()
                    .id(SharedString::from(format!("ig-{i}")))
                    .w_full()
                    .h(px(40.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_1()
                    .bg(rgb(if sel { theme::primary() } else { theme::bg() }))
                    .text_size(px(11.))
                    .text_color(rgb(if sel { theme::on_primary() } else { theme::text_dim() }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::hover())))
                    .child(SharedString::from(g.clone()))
                    .on_click(move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.iptv_group = gname.clone();
                            v.sync_iptv_channels(&gname);
                            v.iptv_ch_idx = 0;
                            cx.notify();
                        });
                    }),
            );
        }

        // 频道列
        let mut ccol = div()
            .id("iptv-channels")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .overflow_y_scroll();
        if channels.is_empty() {
            ccol = ccol.child(widgets::empty_state(theme::icon::IPTV, "暂无频道", "请选择分组或导入源", None));
        } else {
            for (i, ch) in channels.iter().take(400).enumerate() {
                let sel = i == ch_idx;
                let e = entity.clone();
                let name = ch.name.clone();
                ccol = ccol.child(
                    div()
                        .id(SharedString::from(format!("ich-{i}")))
                        .w_full()
                        .h(px(40.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .px_2()
                        .gap_2()
                        .bg(rgb(if sel { theme::selected() } else { theme::bg() }))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::hover())))
                        .child(
                            div()
                                .w(px(10.))
                                .text_size(px(10.))
                                .text_color(rgb(if sel { theme::primary() } else { theme::text_faint() }))
                                .child(if sel { theme::icon::PLAY } else { "" }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(rgb(if sel { theme::primary() } else { theme::text() }))
                                .overflow_hidden()
                                .child(SharedString::from(name)),
                        )
                        .on_click(move |_, _, cx| {
                            let _ = e.update(cx, |v, cx| {
                                v.play_channel(i, v.iptv_src_idx.min(
                                    v.iptv_channels.get(i).map(|c| c.urls.len().saturating_sub(1)).unwrap_or(0),
                                ));
                                cx.notify();
                            });
                        }),
                );
            }
        }

        // 流源列
        let cur_urls: Vec<String> = channels
            .get(ch_idx)
            .map(|c| (0..c.urls.len()).map(|i| format!("源{}", i + 1)).collect())
            .unwrap_or_default();
        let mut scol = div()
            .id("iptv-sources")
            .w(px(theme::IPTV_SOURCE_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(theme::border()))
            .overflow_y_scroll();
        for (i, label) in cur_urls.iter().enumerate() {
            let sel = i == src_idx;
            let e = entity.clone();
            scol = scol.child(
                div()
                    .id(SharedString::from(format!("isrc-{i}")))
                    .mx_1()
                    .my_1()
                    .h(px(28.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_xl()
                    .bg(rgb(if sel { theme::primary() } else { theme::surface() }))
                    .text_size(px(11.))
                    .text_color(rgb(if sel { theme::on_primary() } else { theme::text_dim() }))
                    .cursor_pointer()
                    .child(SharedString::from(label.clone()))
                    .on_click(move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.play_channel(v.iptv_ch_idx, i);
                            cx.notify();
                        });
                    }),
            );
        }

        div()
            .w(px(theme::IPTV_PANEL_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(theme::bg()))
            .child(
                div()
                    .w_full()
                    .h(px(48.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .px_3()
                    .border_b_1()
                    .border_color(rgb(theme::border()))
                    .text_size(px(14.))
                    .text_color(rgb(theme::text()))
                    .child(SharedString::from(if self.iptv_title.is_empty() {
                        "频道列表".to_string()
                    } else {
                        self.iptv_title.clone()
                    })),
            )
            .child(div().flex().flex_1().min_h(px(0.)).child(gcol).child(ccol).child(scol))
            .into_any_element()
    }
}

// ============================ 音乐播放页 ============================

impl AppView {
    pub(crate) fn render_music_player(&mut self, entity: Entity<Self>) -> AnyElement {
        let st = &self.state;
        let pos = Self::fmt_time(st.position);
        let dur = Self::fmt_time(st.duration);
        let paused = st.paused;
        let ratio = self.ratio();
        let rate_label = format!("{:.2}x", self.rate);
        let repeat = self.repeat;
        let show_queue = self.show_queue;

        let track = self.current_track.clone();
        let title = track
            .as_ref()
            .map(|t| t.display_title().to_string())
            .unwrap_or_else(|| "未在播放".into());
        let artist = track
            .as_ref()
            .map(|t| t.artist.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "未知艺术家".into());
        let cover = track.as_ref().and_then(|t| {
            t.cover.as_ref().and_then(|b| self.images.from_bytes(&t.key(), b.as_slice()))
        });

        // 顶栏
        let e_back = entity.clone();
        let e_queue = entity.clone();
        let top = div()
            .w_full()
            .h(px(52.))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .gap_3()
            .bg(rgb(theme::surface()))
            .border_b_1()
            .border_color(rgb(theme::border()))
            .child(widgets::icon_btn(theme::icon::BACK, move |_, _, cx| {
                let _ = e_back.update(cx, |v, cx| {
                    // 音乐切回浏览页不停止播放
                    v.page = crate::app::Page::Browse;
                    cx.notify();
                });
            }))
            .child(
                div()
                    .flex_1()
                    .text_size(px(16.))
                    .text_color(rgb(theme::text()))
                    .child("正在播放"),
            )
            .child(widgets::icon_btn(theme::icon::QUEUE, move |_, _, cx| {
                let _ = e_queue.update(cx, |v, cx| {
                    v.show_queue = !v.show_queue;
                    cx.notify();
                });
            }));

        // 左封面面板
        let mut cover_panel = div()
            .w(px(theme::COVER_PANEL_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .px_5()
            .border_r_1()
            .border_color(rgb(theme::border()));
        cover_panel = cover_panel.child(
            div()
                .w(px(220.))
                .h(px(220.))
                .flex_none()
                .rounded_xl()
                .overflow_hidden()
                .shadow_lg()
                .bg(rgb(theme::placeholder()))
                .flex()
                .items_center()
                .justify_center()
                .child(if let Some(ri) = cover {
                    img(ri)
                        .w(px(220.))
                        .h(px(220.))
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    div()
                        .text_size(px(60.))
                        .text_color(rgb(theme::text_faint()))
                        .child(theme::icon::NOTE)
                        .into_any_element()
                }),
        );
        cover_panel = cover_panel
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(rgb(theme::text()))
                    .child(SharedString::from(title)),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(theme::text_dim()))
                    .child(SharedString::from(artist)),
            );

        // 歌词区
        let mut lyric = div()
            .id("lyric-list")
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .overflow_y_scroll()
            .pt_5();
        if self.lyrics.is_empty() {
            lyric = lyric.child(widgets::empty_state(
                theme::icon::LYRICS,
                "暂无歌词",
                "放置同名 .lrc 文件或使用内嵌歌词",
                None,
            ));
        } else {
            for (i, l) in self.lyrics.iter().enumerate() {
                let active = i == self.lyric_idx;
                lyric = lyric.child(
                    div()
                        .id(SharedString::from(format!("lrc-{i}")))
                        .w_full()
                        .h(px(36.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(if active { 16. } else { 13. }))
                        .text_color(rgb(if active { theme::primary() } else { theme::text_dim() }))
                        .child(SharedString::from(l.text.clone())),
                );
            }
        }

        // 底部控制条
        let e_seek = entity.clone();
        let prog = widgets::segmented_bar("mprog", ratio, 40, 4.0, move |i, _w, cx| {
            let _ = e_seek.update(cx, |v, cx| {
                if v.state.duration > 0.0 {
                    let t = v.state.duration * ((i + 1) as f64 / 40.0);
                    let _ = v.player.seek_absolute(t);
                }
                cx.notify();
            });
        });

        let e_vdn = entity.clone();
        let e_vup = entity.clone();
        let e_repeat = entity.clone();
        let e_prev = entity.clone();
        let e_play = entity.clone();
        let e_next = entity.clone();
        let e_rate = entity.clone();

        let controls = div()
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .px_6()
            .pt_2()
            .pb_4()
            .gap_2()
            .border_t_1()
            .border_color(rgb(theme::border()))
            .child(prog)
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(theme::text_dim()))
                            .child(SharedString::from(pos)),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(theme::text_dim()))
                            .child(SharedString::from(dur)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(140.))
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(theme::text_dim()))
                                    .child(theme::icon::VOLUME),
                            )
                            .child(widgets::icon_btn("−", move |_, _, cx| {
                                let _ = e_vdn.update(cx, |v, cx| {
                                    v.change_volume(-5);
                                    cx.notify();
                                });
                            }))
                            .child(widgets::icon_btn("+", move |_, _, cx| {
                                let _ = e_vup.update(cx, |v, cx| {
                                    v.change_volume(5);
                                    cx.notify();
                                });
                            })),
                    )
                    .child(div().flex_1())
                    .child(widgets::icon_btn(repeat.icon(), move |_, _, cx| {
                        let _ = e_repeat.update(cx, |v, cx| {
                            v.repeat = v.repeat.next();
                            cx.notify();
                        });
                    }))
                    .child(widgets::icon_btn(theme::icon::PREV, move |_, _, cx| {
                        let _ = e_prev.update(cx, |v, cx| {
                            v.prev_track();
                            cx.notify();
                        });
                    }))
                    .child(
                        div()
                            .id("music-play")
                            .w(px(44.))
                            .h(px(44.))
                            .flex_none()
                            .rounded_full()
                            .bg(rgb(theme::primary()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(17.))
                            .text_color(rgb(theme::on_primary()))
                            .cursor_pointer()
                            .child(if paused { theme::icon::PLAY } else { theme::icon::PAUSE })
                            .on_click(move |_, _, cx| {
                                let _ = e_play.update(cx, |v, cx| {
                                    v.pause_toggle();
                                    cx.notify();
                                });
                            }),
                    )
                    .child(widgets::icon_btn(theme::icon::NEXT, move |_, _, cx| {
                        let _ = e_next.update(cx, |v, cx| {
                            v.next_track();
                            cx.notify();
                        });
                    }))
                    .child(widgets::text_btn(&rate_label, move |_, _, cx| {
                        let _ = e_rate.update(cx, |v, cx| {
                            v.dropdown = if v.dropdown == Some(Dropdown::Rate) {
                                None
                            } else {
                                Some(Dropdown::Rate)
                            };
                            cx.notify();
                        });
                    }))
                    .child(div().w(px(140.))),
            );

        let center = div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(lyric)
            .child(controls);

        let mut body = div().flex().flex_1().min_h(px(0.)).child(cover_panel).child(center);

        if show_queue {
            body = body.child(self.render_music_queue(entity.clone()));
        }

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(top)
            .child(body)
            .into_any_element()
    }

    fn render_music_queue(&self, entity: Entity<Self>) -> AnyElement {
        let mut col = div()
            .id("mqueue")
            .w(px(theme::QUEUE_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(theme::border()))
            .bg(rgb(theme::bg()));
        col = col.child(
            div()
                .w_full()
                .h(px(48.))
                .flex_none()
                .flex()
                .items_center()
                .px_3()
                .gap_2()
                .border_b_1()
                .border_color(rgb(theme::border()))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.))
                        .text_color(rgb(theme::text()))
                        .child("播放列表"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(theme::text_dim()))
                        .child(SharedString::from(format!("{} 首", self.music_queue.len()))),
                )
                .child(widgets::icon_btn(theme::icon::CLOSE, {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.show_queue = false;
                            cx.notify();
                        });
                    }
                })),
        );

        if self.music_queue.is_empty() {
            col = col.child(widgets::empty_state(theme::icon::QUEUE, "播放列表为空", "从音乐库点击播放", None));
        } else {
            let mut list = div()
                .id("mqueue-items")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for (i, t) in self.music_queue.iter().take(400).enumerate() {
                let sel = i == self.queue_idx;
                let e = entity.clone();
                list = list.child(
                    div()
                        .id(SharedString::from(format!("mq-{i}")))
                        .w_full()
                        .h(px(44.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .px_3()
                        .gap_2()
                        .bg(rgb(if sel { theme::selected() } else { theme::bg() }))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::hover())))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(rgb(if sel { theme::primary() } else { theme::text() }))
                                .overflow_hidden()
                                .child(SharedString::from(t.display_title().to_string())),
                        )
                        .on_click(move |_, _, cx| {
                            let _ = e.update(cx, |v, cx| {
                                v.queue_idx = i;
                                if let Some(t) = v.music_queue.get(i).cloned() {
                                    v.current_track = Some(t.clone());
                                    v.lyrics = crate::music::load_lyrics(&t);
                                    let _ = v.player.load(&t.path);
                                }
                                cx.notify();
                            });
                        }),
                );
            }
            col = col.child(list);
        }
        col.into_any_element()
    }
}
