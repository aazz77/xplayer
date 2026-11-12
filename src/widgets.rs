//! 自建控件库。GPUI 0.2.2 只有原子元素，没有 Material 控件，
//! 这里按 Nexus 截图用 div 拼出按钮/输入框/进度条/角标/对话框等。
//!
//! 全部只用 nyaterm（同一 zed fork）实际编译过的 Styled 方法，
//! 返回 AnyElement 以便任意嵌套。

use gpui::*;
use gpui::prelude::FluentBuilder;

use crate::theme;

pub fn h_line() -> AnyElement {
    div().h(px(1.)).bg(rgb(theme::border())).into_any_element()
}
pub fn v_line() -> AnyElement {
    div().w(px(1.)).h_full().bg(rgb(theme::border())).into_any_element()
}

/// 方形图标按钮（工具栏、卡片操作）
pub fn icon_btn(
    glyph: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let g: SharedString = glyph.into();
    div()
        .id(SharedString::from(format!("ib-{g}")))
        .w(px(32.))
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(15.))
        .text_color(rgb(theme::text_dim()))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::hover())))
        .child(g)
        .on_click(on_click)
        .into_any_element()
}

/// 危险操作图标按钮（删除）
pub fn danger_icon_btn(
    glyph: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let g: SharedString = glyph.into();
    div()
        .id(SharedString::from(format!("db-{g}")))
        .w(px(32.))
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(15.))
        .text_color(rgb(theme::danger()))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0xfbe9e7)))
        .child(g)
        .on_click(on_click)
        .into_any_element()
}

/// 文字小按钮
pub fn text_btn(
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let l: SharedString = label.into();
    div()
        .id(SharedString::from(format!("tb-{l}")))
        .px_3()
        .h(px(32.))
        .flex()
        .items_center()
        .rounded_md()
        .bg(rgb(theme::surface()))
        .text_size(px(12.))
        .text_color(rgb(theme::text()))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::hover())))
        .child(l)
        .on_click(on_click)
        .into_any_element()
}

/// 主色胶囊按钮（导入 / 播放 / 确定）
pub fn primary_btn(
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let l: SharedString = label.into();
    div()
        .id(SharedString::from(format!("pb-{l}")))
        .px_4()
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_xl()
        .bg(rgb(theme::primary()))
        .text_size(px(13.))
        .text_color(rgb(theme::on_primary()))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::primary_deep())))
        .child(l)
        .on_click(on_click)
        .into_any_element()
}

/// 边框按钮（取消）
pub fn outline_btn(
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let l: SharedString = label.into();
    div()
        .id(SharedString::from(format!("ob-{l}")))
        .px_4()
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_xl()
        .border_1()
        .border_color(rgb(theme::border()))
        .text_size(px(13.))
        .text_color(rgb(theme::text()))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::hover())))
        .child(l)
        .on_click(on_click)
        .into_any_element()
}

/// 搜索框。点击后由根容器的键盘处理接管输入；右侧「粘贴」按钮兜底中文。
pub fn search_field(
    id_tag: &str,
    text: &str,
    placeholder: &str,
    active: bool,
    on_activate: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_paste: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let shown = if text.is_empty() {
        SharedString::from(placeholder.to_string())
    } else {
        SharedString::from(text.to_string())
    };
    let fg = if text.is_empty() {
        rgb(theme::text_faint())
    } else {
        rgb(theme::text())
    };
    div()
        .id(SharedString::from(format!("sf-{id_tag}")))
        .w(px(theme::SEARCH_W))
        .h(px(theme::SEARCH_H))
        .flex()
        .items_center()
        .px_3()
        .gap_2()
        .rounded_xl()
        .border_1()
        .border_color(rgb(if active { theme::primary() } else { theme::border() }))
        .bg(rgb(theme::bg()))
        .cursor_pointer()
        .child(
            div()
                .text_size(px(14.))
                .text_color(rgb(theme::text_dim()))
                .child(theme::icon::SEARCH),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(12.))
                .text_color(fg)
                .overflow_hidden()
                .child(shown),
        )
        .when(active, |d| {
            d.child(
                div()
                    .w(px(1.))
                    .h(px(14.))
                    .bg(rgb(theme::primary())),
            )
        })
        .child(
            div()
                .id(SharedString::from(format!("paste-{id_tag}")))
                .px_1()
                .text_size(px(10.))
                .text_color(rgb(theme::text_dim()))
                .cursor_pointer()
                .child("贴")
                .on_click(on_paste),
        )
        .on_click(on_activate)
        .into_any_element()
}

/// 通用单行文本输入（对话框用）
pub fn input_field(
    id_tag: &str,
    text: &str,
    placeholder: &str,
    active: bool,
    on_activate: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_paste: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let shown = if text.is_empty() {
        SharedString::from(placeholder.to_string())
    } else {
        SharedString::from(text.to_string())
    };
    let fg = if text.is_empty() {
        rgb(theme::text_faint())
    } else {
        rgb(theme::text())
    };
    div()
        .id(SharedString::from(format!("if-{id_tag}")))
        .w_full()
        .h(px(40.))
        .flex()
        .items_center()
        .px_3()
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(if active { theme::primary() } else { theme::border() }))
        .bg(rgb(theme::bg()))
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .text_size(px(13.))
                .text_color(fg)
                .overflow_hidden()
                .child(shown),
        )
        .when(active, |d| {
            d.child(div().w(px(1.)).h(px(14.)).bg(rgb(theme::primary())))
        })
        .child(
            div()
                .id(SharedString::from(format!("ipaste-{id_tag}")))
                .px_1()
                .text_size(px(10.))
                .text_color(rgb(theme::text_dim()))
                .cursor_pointer()
                .child("贴")
                .on_click(on_paste),
        )
        .on_click(on_activate)
        .into_any_element()
}

/// 右上角格式角标（MP4 / TS / MP3）
pub fn ext_badge(ext: &str) -> AnyElement {
    div()
        .px_2()
        .py_0()
        .rounded_sm()
        .bg(rgb(theme::badge_bg()))
        .text_size(px(10.))
        .text_color(rgb(0xffffff))
        .child(SharedString::from(ext.to_string()))
        .into_any_element()
}

/// 灰色小角标（列表右侧格式）
pub fn soft_badge(text: &str) -> AnyElement {
    div()
        .px_2()
        .py_0()
        .rounded_sm()
        .bg(rgb(theme::surface_alt()))
        .text_size(px(10.))
        .text_color(rgb(theme::text_dim()))
        .child(SharedString::from(text.to_string()))
        .into_any_element()
}

/// 绿色「自动更新」角标
pub fn green_badge(text: &str) -> AnyElement {
    div()
        .px_2()
        .py_0()
        .rounded_sm()
        .bg(rgb(theme::green_bg()))
        .text_size(px(10.))
        .text_color(rgb(theme::green_fg()))
        .child(SharedString::from(text.to_string()))
        .into_any_element()
}

/// 红色 LIVE 角标
pub fn live_badge() -> AnyElement {
    div()
        .px_2()
        .py_0()
        .rounded_sm()
        .bg(rgb(theme::live()))
        .text_size(px(10.))
        .text_color(rgb(0xffffff))
        .child("LIVE")
        .into_any_element()
}

/// 分组标题（设置页）
pub fn section_title(text: &str) -> AnyElement {
    div()
        .pb_2()
        .text_size(px(13.))
        .text_color(rgb(theme::primary()))
        .child(SharedString::from(text.to_string()))
        .into_any_element()
}

/// 设置行：标题 + 说明 + 右侧值（点击切换）
pub fn setting_row(
    title: &str,
    desc: &str,
    value: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("sr-{title}")))
        .w_full()
        .py_3()
        .flex()
        .items_center()
        .gap_3()
        .cursor_pointer()
        .hover(|s| s.bg(rgb(theme::hover())))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(theme::text()))
                        .child(SharedString::from(title.to_string())),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(theme::text_dim()))
                        .child(SharedString::from(desc.to_string())),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(13.))
                .text_color(rgb(theme::text()))
                .child(SharedString::from(value.to_string()))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(theme::text_dim()))
                        .child("▾"),
                ),
        )
        .on_click(on_click)
        .into_any_element()
}

/// 单选行（外观模式）
pub fn radio_row(
    label: &str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("rr-{label}")))
        .flex()
        .items_center()
        .gap_2()
        .py_1()
        .cursor_pointer()
        .child(
            div()
                .w(px(16.))
                .h(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(rgb(if selected { theme::primary() } else { theme::text_faint() }))
                .child(
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .rounded_full()
                        .bg(rgb(if selected { theme::primary() } else { theme::bg() })),
                ),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(theme::text()))
                .child(SharedString::from(label.to_string())),
        )
        .on_click(on_click)
        .into_any_element()
}

/// 主题色圆点
pub fn color_swatch(
    color: u32,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("cs-{color}")))
        .w(px(28.))
        .h(px(28.))
        .rounded_full()
        .bg(rgb(color))
        .border_1()
        .border_color(rgb(if selected { 0xffffff } else { color }))
        .cursor_pointer()
        .on_click(on_click)
        .into_any_element()
}

/// 空状态
pub fn empty_state(
    glyph: &str,
    title: &str,
    hint: &str,
    action: Option<(&str, Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>)>,
) -> AnyElement {
    let mut col = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .text_size(px(52.))
                .text_color(rgb(theme::text_faint()))
                .child(SharedString::from(glyph.to_string())),
        )
        .child(
            div()
                .text_size(px(15.))
                .text_color(rgb(theme::text_dim()))
                .child(SharedString::from(title.to_string())),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::text_faint()))
                .child(SharedString::from(hint.to_string())),
        );
    if let Some((label, cb)) = action {
        let label = SharedString::from(label.to_string());
        col = col.child(div().pt_3().child(primary_btn(label, cb)));
    }
    col.into_any_element()
}

/// 分段进度条 / 滑块。点击第 i 段 -> on_seek(i/count)。
/// 用分段避免依赖 ClickEvent 坐标计算，视觉仍是一条连续进度条。
pub fn segmented_bar(
    id_tag: &str,
    ratio: f64,
    count: usize,
    height: f32,
    on_seek: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> AnyElement {
    let ratio = ratio.clamp(0.0, 1.0);
    let mut row = div().id(SharedString::from(id_tag.to_string())).w_full().flex();
    for i in 0..count {
        let filled = ((i as f64 + 1.0) / count as f64) <= ratio;
        let cb = on_seek.clone();
        row = row.child(
            div()
                .id(SharedString::from(format!("{id_tag}-seg-{i}")))
                .flex_1()
                .h(px(height))
                .bg(rgb(theme::surface_alt()))
                .cursor_pointer()
                .child(
                    div()
                        .w_full()
                        .h_full()
                        .bg(rgb(if filled { theme::primary() } else { theme::surface_alt() })),
                )
                .on_click(move |_, w, cx| cb(i, w, cx)),
        );
    }
    row.into_any_element()
}

/// 播放页黑底进度条（白/主色）
pub fn player_bar(
    id_tag: &str,
    ratio: f64,
    count: usize,
    on_seek: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> AnyElement {
    let ratio = ratio.clamp(0.0, 1.0);
    let mut row = div().id(SharedString::from(id_tag.to_string())).w_full().flex();
    for i in 0..count {
        let filled = ((i as f64 + 1.0) / count as f64) <= ratio;
        let cb = on_seek.clone();
        row = row.child(
            div()
                .id(SharedString::from(format!("{id_tag}-s-{i}")))
                .flex_1()
                .h(px(4.))
                .bg(rgb(0x555555))
                .cursor_pointer()
                .child(
                    div()
                        .w_full()
                        .h_full()
                        .bg(rgb(if filled { theme::primary() } else { 0x555555 })),
                )
                .on_click(move |_, w, cx| cb(i, w, cx)),
        );
    }
    row.into_any_element()
}

/// 弹出层遮罩（对话框背景）。遮罩半透明，卡片保持不透明。
pub fn dialog_overlay(card: AnyElement) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .absolute()
                .inset_0()
                .bg(rgb(0x000000))
                .opacity(0.35),
        )
        .child(
            div()
                .relative()
                .w(px(420.))
                .rounded_xl()
                .bg(rgb(theme::bg()))
                .shadow_lg()
                .p_5()
                .child(card),
        )
        .into_any_element()
}

/// 简单下拉选项列表（点击即选中）
pub fn dropdown_list(
    id_tag: &str,
    items: &[String],
    selected: usize,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static + Clone,
) -> AnyElement {
    let mut col = div()
        .id(SharedString::from(format!("dd-{id_tag}")))
        .w(px(200.))
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(rgb(theme::border()))
        .bg(rgb(theme::bg()))
        .shadow_lg();
    for (i, it) in items.iter().enumerate() {
        let cb = on_pick.clone();
        let label = it.clone();
        col = col.child(
            div()
                .id(SharedString::from(format!("dd-{id_tag}-{i}")))
                .px_3()
                .py_2()
                .text_size(px(12.))
                .bg(rgb(if i == selected { theme::selected() } else { theme::bg() }))
                .text_color(rgb(if i == selected { theme::primary() } else { theme::text() }))
                .cursor_pointer()
                .hover(|s| s.bg(rgb(theme::hover())))
                .child(SharedString::from(label))
                .on_click(move |_, w, cx| cb(i, w, cx)),
        );
    }
    col.into_any_element()
}
