//! 4 个库页（视频 / IPTV / 音乐 / 设置）+ 导入/网络流对话框。

use gpui::*;
use gpui::prelude::FluentBuilder;

use crate::app::{AppView, Dropdown, InputTarget, Nav};
use crate::iptv::IptvSource;
use crate::library::SortMode;
use crate::music::Category;
use crate::{theme, widgets};

const MAX_RENDER: usize = 400;

// ============================ 视频库 ============================

impl AppView {
    pub(crate) fn render_video_page(&mut self, entity: Entity<Self>) -> AnyElement {
        let st = self.library.state();
        let files = st.filtered();
        let count = files.len();
        let selected_root = st.selected_root.clone();
        let roots = st.roots.clone();
        let search = self.video_search.clone();
        let active_input = self.input == InputTarget::VideoSearch;

        // ---- 左侧 200px 路径栏 ----
        let mut side = div()
            .w(px(theme::SUBNAV_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .py_3()
            .border_r_1()
            .border_color(rgb(theme::border()))
            .bg(rgb(theme::bg()));

        side = side.child(
            div()
                .px_3()
                .pb_3()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.))
                        .text_color(rgb(theme::text()))
                        .child("媒体库"),
                )
                .child(widgets::icon_btn(theme::icon::FOLDER_ADD, {
                    let e = entity.clone();
                    move |_, _, cx| {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            let _ = e.update(cx, |v, cx| {
                                v.library.add_root(path);
                                v.status = "已添加路径".into();
                                cx.notify();
                            });
                        }
                    }
                })),
        );

        // 全部视频
        side = side.child(crate::pages::side_item(
            "all-video",
            theme::icon::VIDEO,
            "全部视频",
            Some(st.total_count()),
            selected_root.is_none(),
            {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.library.set_selected_root(None);
                        cx.notify();
                    });
                }
            },
        ));

        for (i, r) in roots.iter().enumerate() {
            let key = r.to_string_lossy().into_owned();
            let label = r
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.clone());
            let sel = selected_root.as_deref() == Some(key.as_str());
            let key_sel = key.clone();
            let key_del = key.clone();
            let label_show = label.clone();
            let n = st.files_by_root.get(&key).map(|v| v.len()).unwrap_or(0);
            let e_sel = entity.clone();
            let e_del = entity.clone();
            side = side.child(
                div()
                    .id(SharedString::from(format!("vroot-{i}")))
                    .mx_2()
                    .px_2()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .bg(rgb(if sel { theme::selected() } else { theme::bg() }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::hover())))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(theme::text_dim()))
                            .child(theme::icon::FOLDER),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(if sel { theme::primary() } else { theme::text() }))
                            .overflow_hidden()
                            .child(SharedString::from(label_show)),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(theme::text_faint()))
                            .child(SharedString::from(n.to_string())),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("vroot-del-{i}")))
                            .px_1()
                            .text_size(px(11.))
                            .text_color(rgb(theme::text_faint()))
                            .cursor_pointer()
                            .child(theme::icon::CLOSE)
                            .on_click(move |_, _, cx| {
                                let _ = e_del.update(cx, |v, cx| {
                                    v.library.remove_root(&key_del);
                                    cx.notify();
                                });
                            }),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = e_sel.update(cx, |v, cx| {
                            v.library.set_selected_root(Some(key_sel.clone()));
                            cx.notify();
                        });
                    }),
            );
        }
        side = side.child(div().flex_1());

        // ---- 右侧 ----
        let sort_label = match st.sort {
            SortMode::Name => "按名称",
            SortMode::Size => "按大小",
            SortMode::Ext => "按格式",
        };
        let cur_name = selected_root
            .as_ref()
            .and_then(|k| {
                roots
                    .iter()
                    .find(|r| r.to_string_lossy() == k.as_str())
                    .and_then(|r| r.file_name().map(|s| s.to_string_lossy().into_owned()))
            })
            .unwrap_or_else(|| "全部视频".into());

        let toolbar = div()
            .w_full()
            .h(px(56.))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .gap_2()
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(rgb(theme::text()))
                    .child(SharedString::from(cur_name)),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::text_dim()))
                    .child(SharedString::from(format!("{count} 个视频"))),
            )
            .child(div().flex_1())
            .child(widgets::search_field(
                "video",
                &search,
                "搜索视频...",
                active_input,
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.activate_input(InputTarget::VideoSearch);
                            cx.notify();
                        });
                    }
                },
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.activate_input(InputTarget::VideoSearch);
                            v.paste_to_input(cx);
                            cx.notify();
                        });
                    }
                },
            ))
            .child(widgets::icon_btn(theme::icon::REFRESH, {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.library.rescan();
                        v.status = "已扫描".into();
                        cx.notify();
                    });
                }
            }))
            .child(widgets::text_btn(sort_label, {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.dropdown = if v.dropdown == Some(Dropdown::VideoSort) {
                            None
                        } else {
                            Some(Dropdown::VideoSort)
                        };
                        v.video_sort_open = v.dropdown.is_some();
                        cx.notify();
                    });
                }
            }));

        let mut grid = div()
            .id("video-grid")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_wrap()
            .items_start()
            .p_4()
            .gap_3();

        if files.is_empty() {
            grid = grid.child(widgets::empty_state(
                theme::icon::VIDEO,
                if roots.is_empty() { "没有媒体库路径" } else { "未找到视频" },
                if roots.is_empty() {
                    "在左侧点击 + 添加本地文件夹"
                } else {
                    "试试更换路径或清空搜索"
                },
                if roots.is_empty() {
                    Some((
                        "添加媒体库路径",
                        Box::new({
                            let e = entity.clone();
                            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    let _ = e.update(cx, |v, cx| {
                                        v.library.add_root(path);
                                        cx.notify();
                                    });
                                }
                            }
                        }) as Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
                    ))
                } else {
                    None
                },
            ));
        } else {
            for (i, f) in files.into_iter().take(MAX_RENDER).enumerate() {
                let thumb = self
                    .images
                    .from_file(st.thumbnails.get(&f.path.to_string_lossy().into_owned()).map(|s| s.as_str()).unwrap_or(""));
                let ext = f
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_uppercase();
                grid = grid.child(video_card(
                    i,
                    &f.name,
                    &ext,
                    thumb,
                    {
                        let e = entity.clone();
                        let f2 = f.clone();
                        move |_, _, cx| {
                            let _ = e.update(cx, |v, cx| {
                                let list = v.library.state().filtered();
                                let idx = list.iter().position(|x| x.path == f2.path).unwrap_or(0);
                                v.play_video(&f2, list, idx);
                                cx.notify();
                            });
                        }
                    },
                ));
            }
        }

        // 右下角 FAB：播放网络流
        let e_fab = entity.clone();
        let fab = div()
            .id("net-fab")
            .absolute()
            .bottom(px(20.))
            .right(px(20.))
            .w(px(46.))
            .h(px(46.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_xl()
            .bg(rgb(theme::primary()))
            .text_size(px(20.))
            .text_color(rgb(theme::on_primary()))
            .shadow_lg()
            .cursor_pointer()
            .child(theme::icon::CAST)
            .on_click(move |_, _, cx| {
                let _ = e_fab.update(cx, |v, cx| {
                    v.net_dialog = true;
                    v.net_url.clear();
                    v.activate_input(InputTarget::NetUrl);
                    cx.notify();
                });
            });

        div()
            .relative()
            .flex()
            .flex_1()
            .h_full()
            .child(side)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(toolbar)
                    .child(widgets::h_line())
                    .child(grid),
            )
            .child(fab)
            .into_any_element()
    }

    // ---- 播放网络视频对话框 ----
    pub(crate) fn render_net_dialog(&self, entity: Entity<Self>) -> AnyElement {
        let active = self.input == InputTarget::NetUrl;
        let e_in = entity.clone();
        let e_paste = entity.clone();
        let e_cancel = entity.clone();
        let e_play = entity.clone();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(rgb(theme::text()))
                    .child("播放网络视频"),
            )
            .child(widgets::input_field(
                "net-url",
                &self.net_url,
                "输入视频链接（http / rtmp / rtsp）",
                active,
                move |_, _, cx| {
                    let _ = e_in.update(cx, |v, cx| {
                        v.activate_input(InputTarget::NetUrl);
                        cx.notify();
                    });
                },
                move |_, _, cx| {
                    let _ = e_paste.update(cx, |v, cx| {
                        v.activate_input(InputTarget::NetUrl);
                        v.paste_to_input(cx);
                        cx.notify();
                    });
                },
            ))
            .child(
                div()
                    .pt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(widgets::outline_btn("取消", move |_, _, cx| {
                        let _ = e_cancel.update(cx, |v, cx| {
                            v.net_dialog = false;
                            v.input = InputTarget::None;
                            cx.notify();
                        });
                    }))
                    .child(widgets::primary_btn("播放", move |_, _, cx| {
                        let _ = e_play.update(cx, |v, cx| {
                            let url = v.net_url.trim().to_string();
                            if !url.is_empty() {
                                v.net_dialog = false;
                                v.input = InputTarget::None;
                                v.play_url(&url, "网络视频");
                            }
                            cx.notify();
                        });
                    })),
            )
            .into_any_element()
    }
}

fn video_card(
    i: usize,
    name: &str,
    ext: &str,
    thumb: Option<std::sync::Arc<RenderImage>>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let mut card = div()
        .id(SharedString::from(format!("vcard-{i}")))
        .w(px(theme::CARD_W))
        .h(px(theme::CARD_H))
        .flex_none()
        .relative()
        .rounded_lg()
        .overflow_hidden()
        .bg(rgb(theme::placeholder()))
        .cursor_pointer();

    if let Some(ri) = thumb {
        card = card.child(
            img(ri)
                .w(px(theme::CARD_W))
                .h(px(theme::CARD_H))
                .object_fit(ObjectFit::Cover),
        );
    } else {
        card = card.child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(34.))
                .text_color(rgb(theme::text_faint()))
                .child(theme::icon::VIDEO),
        );
    }

    // 底部渐变近似：半透明黑条 + 文件名
    card = card
        .child(
            div()
                .absolute()
                .bottom(px(0.))
                .left(px(0.))
                .w_full()
                .h(px(48.))
                .bg(rgb(0x000000))
                .opacity(0.55),
        )
        .child(
            div()
                .absolute()
                .bottom(px(0.))
                .left(px(0.))
                .w_full()
                .px_2()
                .pb_2()
                .text_size(px(11.))
                .text_color(rgb(0xffffff))
                .line_clamp(2)
                .child(SharedString::from(name.to_string())),
        )
        .child(
            div()
                .absolute()
                .top(px(6.))
                .right(px(6.))
                .child(widgets::ext_badge(ext)),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(30.))
                .text_color(rgb(0xffffff))
                .opacity(0.0)
                .child(theme::icon::PLAY),
        )
        .on_click(on_click);
    card.into_any_element()
}

/// 侧栏通用项（全部视频 / 全部音乐）
fn side_item(
    id_tag: &str,
    glyph: &str,
    label: &str,
    count: Option<usize>,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(id_tag.to_string()))
        .mx_2()
        .px_2()
        .h(px(36.))
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .bg(rgb(if selected { theme::selected() } else { theme::bg() }))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::hover())))
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(if selected { theme::primary() } else { theme::text_dim() }))
                .child(SharedString::from(glyph.to_string())),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(12.))
                .text_color(rgb(if selected { theme::primary() } else { theme::text() }))
                .child(SharedString::from(label.to_string())),
        )
        .when(count.is_some(), |d| {
            d.child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(theme::text_dim()))
                    .child(SharedString::from(count.unwrap_or(0).to_string())),
            )
        })
        .on_click(on_click)
        .into_any_element()
}

// ============================ IPTV 源管理 ============================

impl AppView {
    pub(crate) fn render_iptv_page(&mut self, entity: Entity<Self>) -> AnyElement {
        let sources = self.iptv.sources.clone();
        let e_import = entity.clone();

        let header = div()
            .w_full()
            .h(px(64.))
            .flex_none()
            .flex()
            .items_center()
            .px_5()
            .child(
                div()
                    .flex_1()
                    .text_size(px(16.))
                    .text_color(rgb(theme::text()))
                    .child("IPTV 源管理"),
            )
            .child(widgets::primary_btn("+ 导入", move |_, _, cx| {
                let _ = e_import.update(cx, |v, cx| {
                    v.iptv_dialog = true;
                    v.iptv_edit_idx = None;
                    v.iptv_name.clear();
                    v.iptv_url.clear();
                    v.iptv_remote = true;
                    v.iptv_auto = false;
                    v.activate_input(InputTarget::IptvName);
                    cx.notify();
                });
            }));

        let mut grid = div()
            .id("iptv-grid")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_wrap()
            .items_start()
            .px_5()
            .pt_4()
            .gap_4();

        if sources.is_empty() {
            grid = grid.child(widgets::empty_state(
                theme::icon::IPTV,
                "暂无 IPTV 源",
                "点击右上角「导入」添加远程链接或本地 m3u 文件",
                None,
            ));
        } else {
            for (i, s) in sources.iter().enumerate() {
                grid = grid.child(iptv_source_card(i, s, entity.clone()));
            }
        }

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(header)
            .child(widgets::h_line())
            .child(grid)
            .into_any_element()
    }

    pub(crate) fn render_iptv_dialog(&self, entity: Entity<Self>) -> AnyElement {
        let name_active = self.input == InputTarget::IptvName;
        let url_active = self.input == InputTarget::IptvUrl;
        let remote = self.iptv_remote;

        let e_name = entity.clone();
        let e_name_paste = entity.clone();
        let e_url = entity.clone();
        let e_url_paste = entity.clone();
        let e_remote = entity.clone();
        let e_file = entity.clone();
        let e_auto = entity.clone();
        let e_cancel = entity.clone();
        let e_ok = entity.clone();

        let title = if self.iptv_edit_idx.is_some() {
            "编辑 IPTV 源"
        } else {
            "导入 IPTV 源"
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(rgb(theme::text()))
                    .child(title),
            )
            .child(widgets::input_field(
                "iptv-name",
                &self.iptv_name,
                "名称 *",
                name_active,
                move |_, _, cx| {
                    let _ = e_name.update(cx, |v, cx| {
                        v.activate_input(InputTarget::IptvName);
                        cx.notify();
                    });
                },
                move |_, _, cx| {
                    let _ = e_name_paste.update(cx, |v, cx| {
                        v.activate_input(InputTarget::IptvName);
                        v.paste_to_input(cx);
                        cx.notify();
                    });
                },
            ))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(theme::text_dim()))
                    .child("导入方式"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(seg_btn("远程导入", theme::icon::NETWORK, remote, move |_, _, cx| {
                        let _ = e_remote.update(cx, |v, cx| {
                            v.iptv_remote = true;
                            cx.notify();
                        });
                    }))
                    .child(seg_btn("文件导入", theme::icon::FILE, !remote, move |_, _, cx| {
                        let _ = e_file.update(cx, |v, cx| {
                            v.iptv_remote = false;
                            cx.notify();
                        });
                    })),
            )
            .child(widgets::input_field(
                "iptv-url",
                &self.iptv_url,
                if remote { "链接" } else { "本地 m3u 文件路径（点「贴」或选文件）" },
                url_active,
                move |_, _, cx| {
                    let _ = e_url.update(cx, |v, cx| {
                        v.activate_input(InputTarget::IptvUrl);
                        cx.notify();
                    });
                },
                move |_, _, cx| {
                    let _ = e_url_paste.update(cx, |v, cx| {
                        if !v.iptv_remote {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("M3U", &["m3u", "m3u8"])
                                .pick_file()
                            {
                                v.iptv_url = path.to_string_lossy().into_owned();
                                v.edit = v.iptv_url.clone();
                                v.input = InputTarget::IptvUrl;
                            }
                        } else {
                            v.activate_input(InputTarget::IptvUrl);
                            v.paste_to_input(cx);
                        }
                        cx.notify();
                    });
                },
            ))
            .child(
                div()
                    .id("iptv-auto-toggle")
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(if self.iptv_auto { theme::primary() } else { theme::text_faint() }))
                            .bg(rgb(if self.iptv_auto { theme::primary() } else { theme::bg() }))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.))
                            .text_color(rgb(0xffffff))
                            .child(if self.iptv_auto { theme::icon::CHECK } else { "" }),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(theme::text()))
                            .child("启动时自动更新"),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = e_auto.update(cx, |v, cx| {
                            v.iptv_auto = !v.iptv_auto;
                            cx.notify();
                        });
                    }),
            )
            .child(widgets::h_line())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(widgets::outline_btn("取消", move |_, _, cx| {
                        let _ = e_cancel.update(cx, |v, cx| {
                            v.iptv_dialog = false;
                            v.input = InputTarget::None;
                            cx.notify();
                        });
                    }))
                    .child(widgets::primary_btn(
                        if self.iptv_edit_idx.is_some() { "保存" } else { "导入" },
                        move |_, _, cx| {
                            let _ = e_ok.update(cx, |v, cx| {
                                let name = v.iptv_name.trim().to_string();
                                let url = v.iptv_url.trim().to_string();
                                if name.is_empty() || url.is_empty() {
                                    v.status = "请填写名称和链接".into();
                                } else {
                                    let src = IptvSource {
                                        name: name.clone(),
                                        type_: if v.iptv_remote { "network" } else { "local" }.into(),
                                        url,
                                        auto_update: v.iptv_auto,
                                    };
                                    if let Some(idx) = v.iptv_edit_idx {
                                        if idx < v.iptv.sources.len() {
                                            v.iptv.sources[idx] = src;
                                            let _ = v.iptv.save_sources();
                                        }
                                    } else {
                                        let _ = v.iptv.add_source(src);
                                    }
                                    v.iptv_dialog = false;
                                    v.input = InputTarget::None;
                                    v.status = format!("已保存源 {name}");
                                }
                                cx.notify();
                            });
                        },
                    )),
            )
            .into_any_element()
    }
}

fn seg_btn(
    label: &str,
    glyph: &str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("seg-{label}")))
        .flex_1()
        .h(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(if selected { theme::primary() } else { theme::border() }))
        .bg(rgb(if selected { theme::selected() } else { theme::surface() }))
        .text_size(px(12.))
        .text_color(rgb(if selected { theme::primary() } else { theme::text_dim() }))
        .cursor_pointer()
        .child(SharedString::from(glyph.to_string()))
        .child(SharedString::from(label.to_string()))
        .on_click(on_click)
        .into_any_element()
}

fn iptv_source_card(
    i: usize,
    s: &IptvSource,
    entity: Entity<AppView>,
) -> AnyElement {
    let e_load = entity.clone();
    let e_refresh = entity.clone();
    let e_edit = entity.clone();
    let e_del = entity.clone();
    let is_remote = s.type_ == "network";

    div()
        .id(SharedString::from(format!("iptv-card-{i}")))
        .w(px(theme::IPTV_CARD_W))
        .h(px(theme::IPTV_CARD_H))
        .flex_none()
        .flex()
        .items_center()
        .px_3()
        .gap_3()
        .rounded_xl()
        .bg(rgb(theme::surface()))
        .cursor_pointer()
        .hover(|s2| s2.bg(rgb(theme::hover())))
        .child(
            div()
                .w(px(40.))
                .h(px(40.))
                .flex_none()
                .rounded_full()
                .bg(rgb(theme::selected()))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.))
                .text_color(rgb(theme::primary()))
                .child(if is_remote { theme::icon::NETWORK } else { theme::icon::FOLDER }),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(theme::text()))
                        .child(SharedString::from(s.name.clone())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(widgets::soft_badge(if is_remote { "远程" } else { "文件" }))
                        .when(s.auto_update, |d| d.child(widgets::green_badge("自动更新"))),
                ),
        )
        .child(widgets::icon_btn(theme::icon::REFRESH, move |_, _, cx| {
            let _ = e_refresh.update(cx, |v, cx| {
                match v.iptv.load_source(i) {
                    Ok(()) => v.status = "已刷新".into(),
                    Err(e) => v.status = format!("刷新失败: {e}"),
                }
                cx.notify();
            });
        }))
        .child(widgets::icon_btn(theme::icon::EDIT, move |_, _, cx| {
            let _ = e_edit.update(cx, |v, cx| {
                if let Some(s) = v.iptv.sources.get(i).cloned() {
                    v.iptv_edit_idx = Some(i);
                    v.iptv_name = s.name;
                    v.iptv_url = s.url;
                    v.iptv_remote = s.type_ == "network";
                    v.iptv_auto = s.auto_update;
                    v.iptv_dialog = true;
                    v.input = InputTarget::None;
                }
                cx.notify();
            });
        }))
        .child(widgets::danger_icon_btn(theme::icon::DELETE, move |_, _, cx| {
            let _ = e_del.update(cx, |v, cx| {
                let _ = v.iptv.remove_source(i);
                cx.notify();
            });
        }))
        .on_click(move |_, _, cx| {
            let _ = e_load.update(cx, |v, cx| {
                match v.iptv.load_source(i) {
                    Ok(()) => {
                        v.iptv_active_src = Some(i);
                        v.iptv_group = v.iptv.playlist.group_names().first().cloned().unwrap_or_default();
                        let g = v.iptv_group.clone();
                        v.sync_iptv_channels(&g);
                        v.nav = Nav::Iptv;
                        v.status = format!(
                            "已加载 · {} 组 · {} 频道",
                            v.iptv.playlist.groups.len(),
                            v.iptv.playlist.total_channels()
                        );
                        // 有频道则直接进入播放页（与 Nexus 单击源一致）
                        if !v.iptv_channels.is_empty() {
                            let title = v.iptv_channels[0].name.clone();
                            v.play_channel(0, 0);
                            v.iptv_title = title;
                        }
                    }
                    Err(e) => v.status = format!("加载失败: {e}"),
                }
                cx.notify();
            });
        })
        .into_any_element()
}

// ============================ 音乐库 ============================

impl AppView {
    pub(crate) fn sync_iptv_channels(&mut self, group: &str) {
        self.iptv_channels = self
            .iptv
            .playlist
            .channels_in(group)
            .iter()
            .cloned()
            .collect();
    }

    pub(crate) fn render_music_page(&mut self, entity: Entity<Self>) -> AnyElement {
        let ms = self.music.state();
        let roots = ms.roots.clone();
        let category = ms.category;
        let count_label = ms.count_label();
        let drill = ms.drill.clone();
        let search = self.music_search.clone();
        let active_input = self.input == InputTarget::MusicSearch;
        let visible = ms.visible_tracks();

        // ---- 左侧路径栏 ----
        let mut side = div()
            .w(px(theme::SUBNAV_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .py_3()
            .border_r_1()
            .border_color(rgb(theme::border()))
            .bg(rgb(theme::bg()));

        side = side.child(
            div()
                .px_3()
                .pb_3()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.))
                        .text_color(rgb(theme::text()))
                        .child("音乐库"),
                )
                .child(widgets::icon_btn(theme::icon::FOLDER_ADD, {
                    let e = entity.clone();
                    move |_, _, cx| {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            let _ = e.update(cx, |v, cx| {
                                v.music.add_root(path);
                                v.status = "已添加音乐路径".into();
                                cx.notify();
                            });
                        }
                    }
                })),
        );
        side = side.child(side_item(
            "all-music",
            theme::icon::MUSIC,
            "全部音乐",
            Some(ms.all_count()),
            ms.selected_root.is_none() && drill.is_none(),
            {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.music.set_selected_root(None);
                        v.music.set_drill(None);
                        cx.notify();
                    });
                }
            },
        ));
        for (i, r) in roots.iter().enumerate() {
            let key = r.to_string_lossy().into_owned();
            let label = r
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| key.clone());
            let sel = ms.selected_root.as_deref() == Some(key.as_str());
            let e_sel = entity.clone();
            let e_del = entity.clone();
            let key_del = key.clone();
            side = side.child(
                div()
                    .id(SharedString::from(format!("mroot-{i}")))
                    .mx_2()
                    .px_2()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .bg(rgb(if sel { theme::selected() } else { theme::bg() }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::hover())))
                    .child(
                        div().text_size(px(13.)).text_color(rgb(theme::text_dim())).child(theme::icon::FOLDER),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(rgb(if sel { theme::primary() } else { theme::text() }))
                            .overflow_hidden()
                            .child(SharedString::from(label)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("mroot-del-{i}")))
                            .px_1()
                            .text_size(px(11.))
                            .child(theme::icon::CLOSE)
                            .on_click(move |_, _, cx| {
                                let _ = e_del.update(cx, |v, cx| {
                                    v.music.remove_root(&key_del);
                                    cx.notify();
                                });
                            }),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = e_sel.update(cx, |v, cx| {
                            v.music.set_selected_root(Some(key.clone()));
                            v.music.set_drill(None);
                            cx.notify();
                        });
                    }),
            );
        }
        side = side.child(div().flex_1());

        // ---- 工具栏 ----
        let crumb = drill.clone().unwrap_or_else(|| "全部音乐".into());
        let show_search = category == Category::Songs || drill.is_some();
        let mut toolbar = div()
            .w_full()
            .h(px(56.))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .gap_2()
            .child(
                div()
                    .id("music-crumb")
                    .text_size(px(14.))
                    .text_color(rgb(theme::text()))
                    .cursor_pointer()
                    .child(SharedString::from(crumb))
                    .on_click({
                        let e = entity.clone();
                        move |_, _, cx| {
                            let _ = e.update(cx, |v, cx| {
                                v.music.set_drill(None);
                                cx.notify();
                            });
                        }
                    }),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::text_dim()))
                    .child(SharedString::from(count_label)),
            )
            .child(div().flex_1());

        if show_search {
            toolbar = toolbar.child(widgets::search_field(
                "music",
                &search,
                "搜索音乐...",
                active_input,
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.activate_input(InputTarget::MusicSearch);
                            cx.notify();
                        });
                    }
                },
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.activate_input(InputTarget::MusicSearch);
                            v.paste_to_input(cx);
                            cx.notify();
                        });
                    }
                },
            ));
        }
        toolbar = toolbar
            .child(widgets::icon_btn(theme::icon::REFRESH, {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.music.rescan();
                        v.status = "已扫描音乐".into();
                        cx.notify();
                    });
                }
            }))
            .child(widgets::icon_btn(theme::icon::SORT, {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.dropdown = if v.dropdown == Some(Dropdown::MusicCategory) {
                            None
                        } else {
                            Some(Dropdown::MusicCategory)
                        };
                        cx.notify();
                    });
                }
            }))
            .child(widgets::text_btn(ms.sort.label(), {
                let e = entity.clone();
                move |_, _, cx| {
                    let _ = e.update(cx, |v, cx| {
                        v.dropdown = if v.dropdown == Some(Dropdown::MusicSort) {
                            None
                        } else {
                            Some(Dropdown::MusicSort)
                        };
                        cx.notify();
                    });
                }
            }));

        // ---- 内容列表 ----
        let mut list = div()
            .id("music-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_col();

        if category != Category::Songs && drill.is_none() {
            let groups = ms.group_entries();
            if groups.is_empty() {
                list = list.child(widgets::empty_state(
                    theme::icon::MUSIC,
                    "没有音乐库路径",
                    "在左侧点击 + 添加本地文件夹",
                    None,
                ));
            } else {
                for (i, (g, n)) in groups.iter().enumerate() {
                    let e = entity.clone();
                    let gname = g.clone();
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("mgrp-{i}")))
                            .w_full()
                            .h(px(56.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .px_4()
                            .gap_3()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::hover())))
                            .child(
                                div()
                                    .w(px(40.))
                                    .h(px(40.))
                                    .rounded_md()
                                    .bg(rgb(theme::surface()))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(18.))
                                    .text_color(rgb(theme::primary()))
                                    .child(if category == Category::Albums {
                                        theme::icon::ALBUM
                                    } else {
                                        theme::icon::ARTIST
                                    }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.))
                                    .text_color(rgb(theme::text()))
                                    .child(SharedString::from(g.clone())),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::text_dim()))
                                    .child(SharedString::from(format!("{n} 首"))),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(rgb(theme::text_faint()))
                                    .child(theme::icon::CHEVRON),
                            )
                            .on_click(move |_, _, cx| {
                                let _ = e.update(cx, |v, cx| {
                                    v.music.set_drill(Some(gname.clone()));
                                    cx.notify();
                                });
                            }),
                    );
                }
            }
        } else {
            if visible.is_empty() {
                list = list.child(widgets::empty_state(
                    theme::icon::MUSIC,
                    "未找到音乐文件",
                    "试试添加路径或清空搜索",
                    None,
                ));
            } else {
                for (row, idx) in visible.iter().take(MAX_RENDER).enumerate() {
                    let t = ms.tracks[*idx].clone();
                    let cover = t.cover.as_ref().and_then(|b| {
                        self.images.from_bytes(&t.key(), b.as_slice())
                    });
                    let cover_node = {
                        let mut node = div()
                            .w(px(36.))
                            .h(px(36.))
                            .flex_none()
                            .rounded_md()
                            .overflow_hidden()
                            .bg(rgb(theme::placeholder()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(16.))
                            .text_color(rgb(theme::text_faint()));
                        node = if let Some(ri) = cover.clone() {
                            node.child(img(ri).w(px(36.)).h(px(36.)).object_fit(ObjectFit::Cover))
                        } else {
                            node.child(theme::icon::NOTE)
                        };
                        node
                    };
                    let is_current = self
                        .current_track
                        .as_ref()
                        .map(|c| c.path == t.path)
                        .unwrap_or(false);
                    let e = entity.clone();
                    let t2 = t.clone();
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("mtrack-{row}")))
                            .w_full()
                            .h(px(52.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .px_4()
                            .gap_3()
                            .bg(rgb(if is_current { theme::selected() } else { theme::bg() }))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::hover())))
                            .child(
                                div()
                                    .w(px(24.))
                                    .text_size(px(11.))
                                    .text_color(rgb(if is_current { theme::primary() } else { theme::text_faint() }))
                                    .child(if is_current {
                                        theme::icon::PLAY
                                    } else {
                                        ""
                                    }),
                            )
                            .child(cover_node)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(rgb(if is_current { theme::primary() } else { theme::text() }))
                                            .overflow_hidden()
                                            .child(SharedString::from(t.display_title().to_string())),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(theme::text_dim()))
                                            .overflow_hidden()
                                            .child(SharedString::from(t.subtitle())),
                                    ),
                            )
                            .child(widgets::soft_badge(&t.ext()))
                            .on_click(move |_, _, cx| {
                                let _ = e.update(cx, |v, cx| {
                                    let st = v.music.state();
                                    let list: Vec<_> = st.visible_tracks().iter().map(|i| st.tracks[*i].clone()).collect();
                                    v.play_track(t2.clone(), list, row);
                                    cx.notify();
                                });
                            }),
                    );
                }
            }
        }

        div()
            .flex_1()
            .h_full()
            .flex()
            .child(side)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(toolbar)
                    .child(widgets::h_line())
                    .child(list),
            )
            .into_any_element()
    }
}

// ============================ 设置页 ============================

impl AppView {
    pub(crate) fn render_settings_page(&mut self, entity: Entity<Self>) -> AnyElement {
        let mut col = div()
            .id("settings-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px_6()
            .py_4();

        // 外观
        col = col
            .child(widgets::section_title("外观"))
            .child(widgets::radio_row("浅色", true, |_, _, _| {}))
            .child(
                div()
                    .py_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(14.))
                            .text_color(rgb(theme::text()))
                            .child("主题颜色"),
                    )
                    .child({
                        let mut row = div().flex().gap_2();
                        for (i, c) in theme::THEME_COLORS.iter().enumerate() {
                            let e = entity.clone();
                            row = row.child(widgets::color_swatch(
                                *c,
                                i == self.settings.theme_index,
                                move |_, _, cx| {
                                    let _ = e.update(cx, |v, cx| {
                                        v.set_theme(i);
                                        cx.notify();
                                    });
                                },
                            ));
                        }
                        row
                    }),
            )
            .child(div().h(px(12.)))
            .child(widgets::section_title("播放设置"));

        // 只用 mpv：后端固定显示，硬软解可切换
        col = col
            .child(widgets::setting_row(
                "视频播放后端",
                "MPV — 画质调优丰富，seek 精准（本项目唯一后端）",
                "MPV",
                |_, _, _| {},
            ))
            .child(widgets::setting_row(
                "IPTV 播放后端",
                "MPV — HLS/TS/RTMP 直播流（本项目唯一后端）",
                "MPV",
                |_, _, _| {},
            ))
            .child(widgets::setting_row(
                "MPV 解码方式",
                self.settings.decode.desc(),
                self.settings.decode.label(),
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.set_decode(v.settings.decode.next());
                            cx.notify();
                        });
                    }
                },
            ))
            .child(widgets::setting_row(
                "MPV 画质预设",
                self.settings.quality.desc(),
                self.settings.quality.label(),
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.set_quality(v.settings.quality.next());
                            cx.notify();
                        });
                    }
                },
            ))
            .child(widgets::setting_row(
                "默认倍速",
                "播放页也可随时切换 0.5x ~ 2.0x",
                &format!("{:.2}x", self.settings.rate),
                {
                    let e = entity.clone();
                    move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.dropdown = Some(Dropdown::Rate);
                            cx.notify();
                        });
                    }
                },
            ))
            .child(div().h(px(12.)))
            .child(widgets::section_title("关于"))
            .child(
                div()
                    .py_2()
                    .text_size(px(14.))
                    .text_color(rgb(theme::text()))
                    .child("Nexus Player for Windows"),
            )
            .child(
                div()
                    .pb_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::text_dim()))
                    .child("版本 0.4.0 · 基于 GPUI + libmpv · 仅 MPV 后端"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::text_faint()))
                    .child("硬解使用 hwdec=auto-copy 保证窗口内嵌画面可见；软解使用 hwdec=no。"),
            );

        col.into_any_element()
    }
}
