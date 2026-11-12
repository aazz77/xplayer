//! 主视图：4 页导航 + 迷你播放栏 + 页面路由 + 全局键盘/轮询。
//!
//! 页面渲染拆分在 pages.rs（4 个库页/设置页）与 players.rs（3 个播放页），
//! 都以 `impl AppView` 形式提供 render_* 方法。

use gpui::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crate::iptv::{IptvStore, M3uChannel};
use crate::library::{Library, SortMode, VideoFile};
use crate::media::ImageCache;
use crate::music::{Category, LyricLine, MusicLibrary, MusicSort, Track};
use crate::player::{PlaybackState, PlayerHandle};
use crate::settings::{DecodeMode, Quality, Settings};
use crate::{theme, widgets};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Video,
    Iptv,
    Music,
    Settings,
}

impl Nav {
    pub fn index(self) -> usize {
        match self {
            Nav::Video => 0,
            Nav::Iptv => 1,
            Nav::Music => 2,
            Nav::Settings => 3,
        }
    }
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Nav::Iptv,
            2 => Nav::Music,
            3 => Nav::Settings,
            _ => Nav::Video,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Browse,
    VideoPlayer,
    IptvPlayer,
    MusicPlayer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mini {
    None,
    Video,
    Iptv,
    Music,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    List,
    Shuffle,
    One,
}

impl RepeatMode {
    pub fn icon(self) -> &'static str {
        match self {
            RepeatMode::List => theme::icon::REPEAT,
            RepeatMode::Shuffle => theme::icon::SHUFFLE,
            RepeatMode::One => theme::icon::REPEAT_ONE,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            RepeatMode::List => "列表循环",
            RepeatMode::Shuffle => "随机播放",
            RepeatMode::One => "单曲循环",
        }
    }
    pub fn next(self) -> Self {
        match self {
            RepeatMode::List => RepeatMode::Shuffle,
            RepeatMode::Shuffle => RepeatMode::One,
            RepeatMode::One => RepeatMode::List,
        }
    }
}

/// 当前接收键盘字符的输入框
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    None,
    VideoSearch,
    MusicSearch,
    NetUrl,
    IptvName,
    IptvUrl,
}

/// 弹出式下拉/选项
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dropdown {
    VideoSort,
    MusicSort,
    MusicCategory,
    Rate,
    Decode,
    Quality,
}

pub struct AppView {
    // ---- 数据层 ----
    pub(crate) library: Arc<Library>,
    pub(crate) iptv: IptvStore,
    pub(crate) music: Arc<MusicLibrary>,
    pub(crate) settings: Settings,
    pub(crate) player: Arc<PlayerHandle>,
    pub(crate) images: Arc<ImageCache>,
    pub(crate) focus: FocusHandle,

    // ---- 框架 ----
    pub(crate) nav: Nav,
    pub(crate) page: Page,
    pub(crate) mini: Mini,
    pub(crate) status: String,
    pub(crate) fullscreen: bool,

    // ---- 播放状态 ----
    pub(crate) state: PlaybackState,
    pub(crate) frame_seq: u64,

    // ---- 视频帧 / GPUI sprite atlas 生命周期 ----
    // GPUI 的 img() 以 (RenderImage.id, frame_index) 为 key 把每帧塞进
    // sprite_atlas 且不会自动移除；而 RenderImage::new() 每次生成新 id。
    // 必须显式 window.drop_image() 释放旧帧，否则播放时纹理逐帧累积到数 GB。
    /// 本帧实际交给 img() 显示的图像（render 阶段统一取一次）
    pub(crate) current_frame: Option<Arc<RenderImage>>,
    /// 最近两帧，render 时把更早的帧从 atlas 释放（延迟一帧保证 GPU 已用完）
    pub(crate) frame_history: VecDeque<Arc<RenderImage>>,
    pub(crate) last_frame_id: Option<ImageId>,
    pub(crate) rate: f64,
    pub(crate) volume: i64,
    pub(crate) show_controls: bool,

    // ---- 视频页 ----
    pub(crate) video_search: String,
    pub(crate) video_sort_open: bool,
    pub(crate) net_dialog: bool,
    pub(crate) net_url: String,

    // ---- IPTV 页 ----
    pub(crate) iptv_dialog: bool,
    pub(crate) iptv_edit_idx: Option<usize>,
    pub(crate) iptv_name: String,
    pub(crate) iptv_url: String,
    pub(crate) iptv_remote: bool,
    pub(crate) iptv_auto: bool,
    pub(crate) iptv_group: String,
    pub(crate) iptv_ch_idx: usize,
    pub(crate) iptv_src_idx: usize,
    pub(crate) iptv_active_src: Option<usize>,
    pub(crate) iptv_title: String,
    pub(crate) iptv_channels: Vec<M3uChannel>,

    // ---- 音乐页 ----
    pub(crate) music_search: String,
    pub(crate) music_sort_open: bool,
    pub(crate) music_queue: Vec<Track>,
    pub(crate) queue_idx: usize,
    pub(crate) repeat: RepeatMode,
    pub(crate) current_track: Option<Track>,
    pub(crate) lyrics: Vec<LyricLine>,
    pub(crate) lyric_idx: usize,
    pub(crate) show_queue: bool,

    // ---- 视频播放列表 ----
    pub(crate) playlist: Vec<VideoFile>,
    pub(crate) playlist_idx: usize,
    pub(crate) show_playlist: bool,

    // ---- 输入 / 下拉 ----
    pub(crate) input: InputTarget,
    pub(crate) edit: String,
    pub(crate) dropdown: Option<Dropdown>,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        theme::set_primary(settings.theme_color());
        let library = Arc::new(Library::new().expect("library"));
        let iptv = IptvStore::new().expect("iptv store");
        let music = Arc::new(MusicLibrary::new().expect("music library"));
        let player = Arc::new(PlayerHandle::new(&settings).expect("mpv"));
        let images = Arc::new(ImageCache::new());
        let focus = cx.focus_handle();
        let nav = Nav::from_index(settings.last_nav);
        let rate = settings.rate;
        let volume = settings.volume;

        // 轮询：播放页 30fps 刷新画面，浏览页低频刷新
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(33)).await;
                let _ = this.update(cx, |view, cx| {
                    let seq = view.player.frame_seq();
                    let st = view.player.state();
                    let changed_frame = seq != view.frame_seq;
                    view.frame_seq = seq;
                    view.state = st;

                    // 音乐：自动下一首 + 歌词滚动
                    if view.page == Page::MusicPlayer && !view.lyrics.is_empty() {
                        let idx = view
                            .lyrics
                            .iter()
                            .position(|l| l.time > view.state.position)
                            .map(|i| i.saturating_sub(1))
                            .unwrap_or(view.lyrics.len().saturating_sub(1));
                        view.lyric_idx = idx;
                    }
                    // 视频/IPTV 播放结束自动切下一个
                    if view.state.eof {
                        if view.page == Page::MusicPlayer {
                            view.state.eof = false;
                            view.next_track();
                        } else if view.page == Page::VideoPlayer {
                            view.state.eof = false;
                            view.next_playlist();
                        }
                    }

                    let in_player = view.page != Page::Browse;
                    if in_player || changed_frame {
                        cx.notify();
                    } else {
                        // 浏览页每 ~250ms 才刷一次状态栏/迷你栏
                        cx.notify();
                    }
                });
            }
        })
        .detach();

        Self {
            library,
            iptv,
            music,
            settings,
            player,
            images,
            focus,
            nav,
            page: Page::Browse,
            mini: Mini::None,
            status: String::new(),
            fullscreen: false,
            state: PlaybackState::default(),
            frame_seq: 0,
            current_frame: None,
            frame_history: VecDeque::new(),
            last_frame_id: None,
            rate,
            volume,
            show_controls: true,
            video_search: String::new(),
            video_sort_open: false,
            net_dialog: false,
            net_url: String::new(),
            iptv_dialog: false,
            iptv_edit_idx: None,
            iptv_name: String::new(),
            iptv_url: String::new(),
            iptv_remote: true,
            iptv_auto: false,
            iptv_group: String::new(),
            iptv_ch_idx: 0,
            iptv_src_idx: 0,
            iptv_active_src: None,
            iptv_title: String::new(),
            iptv_channels: Vec::new(),
            music_search: String::new(),
            music_sort_open: false,
            music_queue: Vec::new(),
            queue_idx: 0,
            repeat: RepeatMode::List,
            current_track: None,
            lyrics: Vec::new(),
            lyric_idx: 0,
            show_queue: false,
            playlist: Vec::new(),
            playlist_idx: 0,
            show_playlist: true,
            input: InputTarget::None,
            edit: String::new(),
            dropdown: None,
        }
    }

    // ---------------- 通用操作 ----------------

    pub(crate) fn set_nav(&mut self, n: Nav) {
        self.nav = n;
        self.page = Page::Browse;
        self.dropdown = None;
        self.input = InputTarget::None;
        self.settings.last_nav = n.index();
        self.settings.save();
    }

    pub(crate) fn go_browse(&mut self) {
        // 视频/IPTV 返回时停止画面但保留迷你栏恢复信息；音乐不停止
        if self.page != Page::MusicPlayer {
            let _ = self.player.stop();
        }
        self.page = Page::Browse;
        self.fullscreen = false;
        self.dropdown = None;
        self.input = InputTarget::None;
    }

    pub(crate) fn fmt_time(secs: f64) -> String {
        if !secs.is_finite() || secs < 0.0 {
            return "00:00".into();
        }
        let t = secs as u64;
        let h = t / 3600;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, (t / 60) % 60, t % 60)
        } else {
            format!("{:02}:{:02}", t / 60, t % 60)
        }
    }

    pub(crate) fn ratio(&self) -> f64 {
        if self.state.duration > 0.0 {
            (self.state.position / self.state.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub(crate) fn pause_toggle(&mut self) {
        let _ = self.player.toggle_pause();
    }
    pub(crate) fn seek_rel(&mut self, d: f64) {
        let _ = self.player.seek_relative(d);
    }
    pub(crate) fn change_volume(&mut self, d: i64) {
        let v = (self.volume + d).clamp(0, 150);
        self.volume = v;
        let _ = self.player.set_volume(v);
        self.settings.volume = v;
        self.settings.save();
    }
    pub(crate) fn set_rate(&mut self, r: f64) {
        self.rate = r;
        let _ = self.player.set_rate(r);
        self.settings.rate = r;
        self.settings.save();
    }

    // ---- 视频 ----
    pub(crate) fn play_video(&mut self, f: &VideoFile, list: Vec<VideoFile>, idx: usize) {
        self.playlist = list;
        self.playlist_idx = idx;
        match self.player.load(&f.path) {
            Ok(()) => {
                self.page = Page::VideoPlayer;
                self.mini = Mini::Video;
                self.status = format!("播放 {}", f.name);
            }
            Err(e) => self.status = format!("失败: {e}"),
        }
    }
    pub(crate) fn play_url(&mut self, url: &str, title: &str) {
        match self.player.load_str(url) {
            Ok(()) => {
                self.page = Page::VideoPlayer;
                self.mini = Mini::Video;
                self.status = format!("播放 {title}");
            }
            Err(e) => self.status = format!("失败: {e}"),
        }
    }
    pub(crate) fn next_playlist(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        self.playlist_idx = (self.playlist_idx + 1) % self.playlist.len();
        let f = self.playlist[self.playlist_idx].clone();
        let _ = self.player.load(&f.path);
    }
    pub(crate) fn prev_playlist(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        self.playlist_idx = (self.playlist_idx + self.playlist.len() - 1) % self.playlist.len();
        let f = self.playlist[self.playlist_idx].clone();
        let _ = self.player.load(&f.path);
    }

    // ---- IPTV ----
    pub(crate) fn play_channel(&mut self, ch_idx: usize, src_idx: usize) {
        if let Some(ch) = self.iptv_channels.get(ch_idx) {
            let url = ch
                .urls
                .get(src_idx)
                .or_else(|| ch.urls.first())
                .cloned()
                .unwrap_or_default();
            if url.is_empty() {
                return;
            }
            self.iptv_ch_idx = ch_idx;
            self.iptv_src_idx = src_idx;
            self.iptv_title = ch.name.clone();
            match self.player.load_str(&url) {
                Ok(()) => {
                    self.page = Page::IptvPlayer;
                    self.mini = Mini::Iptv;
                }
                Err(e) => self.status = format!("失败: {e}"),
            }
        }
    }

    // ---- 音乐 ----
    pub(crate) fn play_track(&mut self, track: Track, queue: Vec<Track>, idx: usize) {
        self.music_queue = queue;
        self.queue_idx = idx;
        self.current_track = Some(track.clone());
        self.lyrics = crate::music::load_lyrics(&track);
        self.lyric_idx = 0;
        match self.player.load(&track.path) {
            Ok(()) => {
                self.page = Page::MusicPlayer;
                self.mini = Mini::Music;
                self.status = format!("播放 {}", track.display_title());
            }
            Err(e) => self.status = format!("失败: {e}"),
        }
    }
    pub(crate) fn next_track(&mut self) {
        if self.music_queue.is_empty() {
            return;
        }
        let idx = match self.repeat {
            RepeatMode::One => self.queue_idx,
            RepeatMode::Shuffle => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let n = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.subsec_nanos() as usize)
                    .unwrap_or(0);
                n % self.music_queue.len()
            }
            RepeatMode::List => (self.queue_idx + 1) % self.music_queue.len(),
        };
        self.queue_idx = idx;
        let t = self.music_queue[idx].clone();
        self.current_track = Some(t.clone());
        self.lyrics = crate::music::load_lyrics(&t);
        self.lyric_idx = 0;
        let _ = self.player.load(&t.path);
    }
    pub(crate) fn prev_track(&mut self) {
        if self.music_queue.is_empty() {
            return;
        }
        self.queue_idx = (self.queue_idx + self.music_queue.len() - 1) % self.music_queue.len();
        let t = self.music_queue[self.queue_idx].clone();
        self.current_track = Some(t.clone());
        self.lyrics = crate::music::load_lyrics(&t);
        self.lyric_idx = 0;
        let _ = self.player.load(&t.path);
    }

    // ---- 设置 ----
    pub(crate) fn set_theme(&mut self, i: usize) {
        self.settings.theme_index = i.min(theme::THEME_COLORS.len() - 1);
        theme::set_primary(self.settings.theme_color());
        self.settings.save();
    }
    pub(crate) fn set_decode(&mut self, d: DecodeMode) {
        self.settings.decode = d;
        self.settings.save();
        let _ = self.player.set_decode(d);
        self.status = format!("解码方式：{}（重新打开文件生效）", d.label());
    }
    pub(crate) fn set_quality(&mut self, q: Quality) {
        self.settings.quality = q;
        self.settings.save();
        let _ = self.player.set_quality(q);
        self.status = format!("画质预设：{}", q.label());
    }

    // ---- 输入 ----
    pub(crate) fn activate_input(&mut self, t: InputTarget) {
        self.input = t;
        self.edit = match t {
            InputTarget::VideoSearch => self.video_search.clone(),
            InputTarget::MusicSearch => self.music_search.clone(),
            InputTarget::NetUrl => self.net_url.clone(),
            InputTarget::IptvName => self.iptv_name.clone(),
            InputTarget::IptvUrl => self.iptv_url.clone(),
            InputTarget::None => String::new(),
        };
    }
    pub(crate) fn sync_input(&mut self) {
        match self.input {
            InputTarget::VideoSearch => {
                self.video_search = self.edit.clone();
                self.library.set_search(self.edit.clone());
            }
            InputTarget::MusicSearch => {
                self.music_search = self.edit.clone();
                self.music.set_search(self.edit.clone());
            }
            InputTarget::NetUrl => self.net_url = self.edit.clone(),
            InputTarget::IptvName => self.iptv_name = self.edit.clone(),
            InputTarget::IptvUrl => self.iptv_url = self.edit.clone(),
            InputTarget::None => {}
        }
    }
    pub(crate) fn paste_to_input(&mut self, cx: &mut App) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.edit.push_str(&text);
                self.sync_input();
            }
        }
    }

    // ---- 下拉 ----
    pub(crate) fn dropdown_items(&self, d: Dropdown) -> Vec<String> {
        match d {
            Dropdown::VideoSort => vec!["按名称", "按大小", "按格式"]
                .into_iter()
                .map(String::from)
                .collect(),
            Dropdown::MusicSort => vec!["名称 ↑", "名称 ↓", "艺术家 ↑", "时间 ↓"]
                .into_iter()
                .map(String::from)
                .collect(),
            Dropdown::MusicCategory => vec!["按歌曲", "按专辑", "按歌手"]
                .into_iter()
                .map(String::from)
                .collect(),
            Dropdown::Rate => vec!["0.5x", "0.75x", "1.0x", "1.25x", "1.5x", "2.0x"]
                .into_iter()
                .map(String::from)
                .collect(),
            Dropdown::Decode => vec!["硬解", "软解"].into_iter().map(String::from).collect(),
            Dropdown::Quality => vec!["性能优先", "均衡", "画质优先"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
    pub(crate) fn dropdown_selected(&self, d: Dropdown) -> usize {
        match d {
            Dropdown::VideoSort => match self.library.state().sort {
                SortMode::Name => 0,
                SortMode::Size => 1,
                SortMode::Ext => 2,
            },
            Dropdown::MusicSort => match self.music.state().sort {
                MusicSort::NameAsc => 0,
                MusicSort::NameDesc => 1,
                MusicSort::ArtistAsc => 2,
                MusicSort::TimeDesc => 3,
            },
            Dropdown::MusicCategory => match self.music.state().category {
                Category::Songs => 0,
                Category::Albums => 1,
                Category::Artists => 2,
            },
            Dropdown::Rate => match self.rate {
                r if (r - 0.5).abs() < 0.01 => 0,
                r if (r - 0.75).abs() < 0.01 => 1,
                r if (r - 1.25).abs() < 0.01 => 3,
                r if (r - 1.5).abs() < 0.01 => 4,
                r if (r - 2.0).abs() < 0.01 => 5,
                _ => 2,
            },
            Dropdown::Decode => match self.settings.decode {
                DecodeMode::Hardware => 0,
                DecodeMode::Software => 1,
            },
            Dropdown::Quality => match self.settings.quality {
                Quality::Performance => 0,
                Quality::Balanced => 1,
                Quality::Quality => 2,
            },
        }
    }
    pub(crate) fn apply_dropdown(&mut self, d: Dropdown, i: usize) {
        match d {
            Dropdown::VideoSort => {
                let m = match i {
                    1 => SortMode::Size,
                    2 => SortMode::Ext,
                    _ => SortMode::Name,
                };
                self.library.set_sort(m);
            }
            Dropdown::MusicSort => {
                let m = match i {
                    1 => MusicSort::NameDesc,
                    2 => MusicSort::ArtistAsc,
                    3 => MusicSort::TimeDesc,
                    _ => MusicSort::NameAsc,
                };
                self.music.set_sort(m);
            }
            Dropdown::MusicCategory => {
                let c = match i {
                    1 => Category::Albums,
                    2 => Category::Artists,
                    _ => Category::Songs,
                };
                self.music.set_category(c);
            }
            Dropdown::Rate => {
                let r = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0].get(i).copied().unwrap_or(1.0);
                self.set_rate(r);
            }
            Dropdown::Decode => self.set_decode(if i == 1 {
                DecodeMode::Software
            } else {
                DecodeMode::Hardware
            }),
            Dropdown::Quality => self.set_quality(match i {
                0 => Quality::Performance,
                2 => Quality::Quality,
                _ => Quality::Balanced,
            }),
        }
        self.dropdown = None;
    }

    /// 键盘事件：输入框字符 / 快捷键
    fn on_key(&mut self, event: &KeyDownEvent, cx: &mut App) {
        let key = event.keystroke.key.as_str();
        let ch = event.keystroke.key_char.clone();

        if self.input != InputTarget::None {
            match key {
                "escape" => {
                    self.input = InputTarget::None;
                    self.edit.clear();
                }
                "enter" => {
                    self.sync_input();
                    self.input = InputTarget::None;
                    self.edit.clear();
                }
                "backspace" | "delete" => {
                    self.edit.pop();
                    self.sync_input();
                }
                "space" => {
                    self.edit.push(' ');
                    self.sync_input();
                }
                "v" if event.keystroke.modifiers.control => {
                    self.paste_to_input(cx);
                }
                _ => {
                    if let Some(s) = ch {
                        // 只接受可打印字符
                        if !s.chars().any(|c| c.is_control()) {
                            self.edit.push_str(&s);
                            self.sync_input();
                        }
                    } else if key.chars().count() == 1 && !key.starts_with("f") {
                        self.edit.push_str(key);
                        self.sync_input();
                    }
                }
            }
            return;
        }

        // 播放页快捷键
        let in_player = self.page != Page::Browse;
        match key {
            "escape" => {
                if self.fullscreen {
                    self.fullscreen = false;
                } else if in_player {
                    self.go_browse();
                } else {
                    self.dropdown = None;
                    self.net_dialog = false;
                    self.iptv_dialog = false;
                }
            }
            "space" if in_player => self.pause_toggle(),
            "left" | "arrowleft" if in_player => {
                if self.page == Page::MusicPlayer {
                    self.prev_track()
                } else {
                    self.seek_rel(-5.0)
                }
            }
            "right" | "arrowright" if in_player => {
                if self.page == Page::MusicPlayer {
                    self.next_track()
                } else {
                    self.seek_rel(5.0)
                }
            }
            "up" | "arrowup" if in_player => self.change_volume(5),
            "down" | "arrowdown" if in_player => self.change_volume(-5),
            "f" if in_player => self.fullscreen = !self.fullscreen,
            _ => {}
        }
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let hide_chrome = self.fullscreen;

        // 只保留最近两帧的纹理：RenderImage::new() 的 id 永不复用，
        // 每帧都会往 GPUI sprite atlas 插入一张新纹理。这里在 paint 之前
        // 把前前一帧显式释放，atlas 恒定 2 张，内存不再随时间增长。
        let cur = self.player.latest_image();
        match &cur {
            Some(ri) if self.last_frame_id != Some(ri.id) => {
                self.last_frame_id = Some(ri.id);
                self.frame_history.push_back(ri.clone());
                while self.frame_history.len() > 2 {
                    if let Some(old) = self.frame_history.pop_front() {
                        let _ = window.drop_image(old);
                    }
                }
            }
            None => {
                self.last_frame_id = None;
                for old in self.frame_history.drain(..) {
                    let _ = window.drop_image(old);
                }
            }
            _ => {}
        }
        // 页面渲染统一使用这一帧，避免与 worker 线程的替换产生竞态
        self.current_frame = cur;

        // 播放区尺寸跟随窗口（估算 SW render 目标分辨率）
        let (vw, vh) = if self.page == Page::VideoPlayer {
            (1000u32, 620u32)
        } else if self.page == Page::IptvPlayer {
            (940u32, 620u32)
        } else {
            (960u32, 540u32)
        };
        self.player.set_target_size(vw, vh);

        let mut root = div()
            .id("app-root")
            .relative()
            .flex()
            .size_full()
            .bg(rgb(theme::bg()))
            .text_color(rgb(theme::text()))
            .font_family(theme::FONT)
            .on_key_down({
                let entity = entity.clone();
                move |event: &KeyDownEvent, _window, cx: &mut App| {
                    let _ = entity.update(cx, |v, cx| v.on_key(event, cx));
                }
            });

        if !hide_chrome {
            root = root.child(self.render_nav_rail(entity.clone())).child(widgets::v_line());
        }

        let mut content = div().flex().flex_col().flex_1().h_full();

        let body = match self.page {
            Page::Browse => match self.nav {
                Nav::Video => self.render_video_page(entity.clone()),
                Nav::Iptv => self.render_iptv_page(entity.clone()),
                Nav::Music => self.render_music_page(entity.clone()),
                Nav::Settings => self.render_settings_page(entity.clone()),
            },
            Page::VideoPlayer => self.render_video_player(entity.clone()),
            Page::IptvPlayer => self.render_iptv_player(entity.clone()),
            Page::MusicPlayer => self.render_music_player(entity.clone()),
        };
        content = content.child(body);

        if !hide_chrome && self.mini != Mini::None {
            content = content.child(self.render_mini_bar(entity.clone()));
        }

        root = root.child(content);

        // 浮层：下拉 / 对话框
        if let Some(d) = self.dropdown {
            let items = self.dropdown_items(d);
            let sel = self.dropdown_selected(d);
            let e2 = entity.clone();
            let list = widgets::dropdown_list("dd", &items, sel, move |i, _w, cx| {
                let _ = e2.update(cx, |v, cx| {
                    v.apply_dropdown(d, i);
                    cx.notify();
                });
            });
            root = root.child(
                div()
                    .absolute()
                    .top(px(56.))
                    .right(px(90.))
                    .child(list),
            );
        }

        if self.net_dialog {
            root = root.child(widgets::dialog_overlay(
                self.render_net_dialog(entity.clone()),
            ));
        }
        if self.iptv_dialog {
            root = root.child(widgets::dialog_overlay(
                self.render_iptv_dialog(entity.clone()),
            ));
        }

        root
    }
}

impl AppView {
    fn render_nav_rail(&self, entity: Entity<Self>) -> AnyElement {
        let rail = div()
            .w(px(theme::NAV_W))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .py_2()
            .gap_1()
            .bg(rgb(theme::nav_bg()));
        let items = [
            (theme::icon::VIDEO, "视频", Nav::Video),
            (theme::icon::IPTV, "IPTV", Nav::Iptv),
            (theme::icon::MUSIC, "音乐", Nav::Music),
            (theme::icon::SETTINGS, "设置", Nav::Settings),
        ];
        let mut r = rail;
        for (icon, label, nav) in items {
            let selected = self.nav == nav && self.page == Page::Browse;
            let e = entity.clone();
            r = r.child(
                div()
                    .id(SharedString::from(format!("nav-{label}")))
                    .w(px(60.))
                    .h(px(56.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .rounded_lg()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::hover())))
                    .child(
                        div()
                            .w(px(26.))
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .bg(rgb(if selected { theme::primary() } else { theme::surface() }))
                            .text_size(px(14.))
                            .text_color(rgb(if selected { theme::on_primary() } else { theme::text_dim() }))
                            .child(icon),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(if selected { theme::primary() } else { theme::text_dim() }))
                            .child(label),
                    )
                    .on_click(move |_, _, cx| {
                        let _ = e.update(cx, |v, cx| {
                            v.set_nav(nav);
                            cx.notify();
                        });
                    }),
            );
        }
        r.into_any_element()
    }

    fn render_mini_bar(&self, entity: Entity<Self>) -> AnyElement {
        let st = &self.state;
        let title = match self.mini {
            Mini::Music => self
                .current_track
                .as_ref()
                .map(|t| t.display_title().to_string())
                .unwrap_or_else(|| "音乐".into()),
            Mini::Iptv => self.iptv_title.clone(),
            _ => st.title.clone(),
        };
        let subtitle = match self.mini {
            Mini::Music => self
                .current_track
                .as_ref()
                .map(|t| t.artist.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "音乐".into()),
            Mini::Iptv => self.iptv_group.clone(),
            _ => "视频".into(),
        };
        let pos = Self::fmt_time(st.position);
        let dur = Self::fmt_time(st.duration);
        let paused = st.paused;
        let ratio = self.ratio();
        let is_music = self.mini == Mini::Music;

        let e_play = entity.clone();
        let e_prev = entity.clone();
        let e_next = entity.clone();
        let e_click = entity.clone();

        let mut bar = div()
            .id("mini-bar")
            .w_full()
            .h(px(theme::MINI_H))
            .flex()
            .items_center()
            .px_4()
            .gap_3()
            .border_t_1()
            .border_color(rgb(theme::border()))
            .bg(rgb(theme::surface()))
            .cursor_pointer()
            .on_click(move |_, _, cx| {
                let _ = e_click.update(cx, |v, cx| {
                    match v.mini {
                        Mini::Video => v.page = Page::VideoPlayer,
                        Mini::Iptv => v.page = Page::IptvPlayer,
                        Mini::Music => v.page = Page::MusicPlayer,
                        Mini::None => {}
                    }
                    cx.notify();
                });
            });

        // 左侧图标/封面
        bar = bar.child(
            div()
                .w(px(40.))
                .h(px(40.))
                .flex_none()
                .rounded_md()
                .bg(rgb(theme::selected()))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.))
                .text_color(rgb(theme::primary()))
                .child(match self.mini {
                    Mini::Music => theme::icon::MUSIC,
                    Mini::Iptv => theme::icon::IPTV,
                    _ => theme::icon::VIDEO,
                }),
        );

        bar = bar.child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(theme::text()))
                        .overflow_hidden()
                        .child(SharedString::from(title)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(theme::text_dim()))
                                .child(SharedString::from(pos)),
                        )
                        .child(
                            div()
                                .w(px(120.))
                                .h(px(3.))
                                .rounded_full()
                                .bg(rgb(theme::surface_alt()))
                                .child(
                                    div()
                                        .w(px(120.0_f32 * ratio as f32))
                                        .h_full()
                                        .rounded_full()
                                        .bg(rgb(theme::primary())),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(theme::text_dim()))
                                .child(SharedString::from(dur)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(theme::text_faint()))
                        .child(SharedString::from(subtitle)),
                ),
        );

        if is_music {
            bar = bar
                .child(widgets::icon_btn(theme::icon::PREV, move |_, _, cx| {
                    let _ = e_prev.update(cx, |v, cx| {
                        v.prev_track();
                        cx.notify();
                    });
                }))
                .child(widgets::icon_btn(
                    if paused { theme::icon::PLAY } else { theme::icon::PAUSE },
                    move |_, _, cx| {
                        let _ = e_play.update(cx, |v, cx| {
                            v.pause_toggle();
                            cx.notify();
                        });
                    },
                ))
                .child(widgets::icon_btn(theme::icon::NEXT, move |_, _, cx| {
                    let _ = e_next.update(cx, |v, cx| {
                        v.next_track();
                        cx.notify();
                    });
                }));
        } else {
            bar = bar.child(widgets::icon_btn(
                if paused { theme::icon::PLAY } else { theme::icon::PAUSE },
                move |_, _, cx| {
                    let _ = e_play.update(cx, |v, cx| {
                        v.pause_toggle();
                        cx.notify();
                    });
                },
            ));
        }
        bar.into_any_element()
    }
}

/// 供 main.rs 注册窗口标题等
pub const APP_TITLE: &str = "Nexus Player";
