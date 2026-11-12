//! 本地音乐库：扫描、标签/封面解析、LRC 歌词。
//! 纯 std 实现，不引入额外 crate：
//!   MP3  -> ID3v2.2/2.3/2.4 (TIT2/TPE1/TALB/APIC/USLT) + ID3v1
//!   FLAC -> VORBIS_COMMENT + PICTURE
//!   OGG/M4A/WAV -> 文件名兜底
//! 封面以原始压缩字节保存在内存，UI 层再解码成 RenderImage。

use anyhow::Result;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MUSIC_EXTS: &[&str] = &[
    "mp3", "flac", "aac", "wav", "ogg", "m4a", "wma", "opus", "ape", "aiff",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    #[default]
    Songs,
    Albums,
    Artists,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Songs => "按歌曲",
            Category::Albums => "按专辑",
            Category::Artists => "按歌手",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Category::Songs => Category::Albums,
            Category::Albums => Category::Artists,
            Category::Artists => Category::Songs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicSort {
    #[default]
    NameAsc,
    NameDesc,
    ArtistAsc,
    TimeDesc,
}

impl MusicSort {
    pub fn label(self) -> &'static str {
        match self {
            MusicSort::NameAsc => "名称 ↑",
            MusicSort::NameDesc => "名称 ↓",
            MusicSort::ArtistAsc => "艺术家 ↑",
            MusicSort::TimeDesc => "时间 ↓",
        }
    }
    pub fn next(self) -> Self {
        match self {
            MusicSort::NameAsc => MusicSort::NameDesc,
            MusicSort::NameDesc => MusicSort::ArtistAsc,
            MusicSort::ArtistAsc => MusicSort::TimeDesc,
            MusicSort::TimeDesc => MusicSort::NameAsc,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub path: PathBuf,
    pub name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// APIC / PICTURE 原始字节（JPEG/PNG）
    pub cover: Option<Arc<Vec<u8>>>,
    /// 内嵌歌词（USLT）
    pub embedded_lyrics: Option<String>,
    pub size: u64,
    pub modified_secs: u64,
}

impl Track {
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.name
        } else {
            &self.title
        }
    }
    pub fn subtitle(&self) -> String {
        let a = if self.artist.is_empty() { "未知艺术家" } else { &self.artist };
        let b = if self.album.is_empty() { "未知专辑" } else { &self.album };
        format!("{a} · {b}")
    }
    pub fn ext(&self) -> String {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_uppercase()
    }
    pub fn key(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MusicState {
    pub roots: Vec<PathBuf>,
    pub selected_root: Option<String>,
    pub tracks: Vec<Track>,
    pub groups: BTreeMap<String, Vec<usize>>,
    /// 下钻的分组名
    pub drill: Option<String>,
    pub search: String,
    pub category: Category,
    pub sort: MusicSort,
    pub scanning: bool,
}

impl MusicState {
    pub fn all_count(&self) -> usize {
        self.tracks.len()
    }
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
    pub fn count_label(&self) -> String {
        let n = if self.drill.is_some() {
            self.visible_tracks().len()
        } else {
            match self.category {
                Category::Songs => self.tracks.len(),
                _ => self.groups.len(),
            }
        };
        let unit = match self.category {
            Category::Songs => "首",
            Category::Albums => "张专辑",
            Category::Artists => "位歌手",
        };
        format!("{n} {unit}")
    }

    /// 当前视图下的曲目索引列表（受路径 / 分组 / 搜索 / 排序影响）
    pub fn visible_tracks(&self) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..self.tracks.len()).collect();
        if let Some(root) = &self.selected_root {
            idx.retain(|i| {
                self.tracks[*i]
                    .path
                    .to_string_lossy()
                    .starts_with(root.as_str())
            });
        }
        if let Some(g) = &self.drill {
            if let Some(list) = self.groups.get(g) {
                idx.retain(|i| list.contains(i));
            }
        }
        let q = self.search.to_lowercase();
        if !q.is_empty() {
            idx.retain(|i| {
                let t = &self.tracks[*i];
                t.display_title().to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
            });
        }
        match self.sort {
            MusicSort::NameAsc => idx.sort_by(|a, b| {
                self.tracks[*a]
                    .display_title()
                    .cmp(self.tracks[*b].display_title())
            }),
            MusicSort::NameDesc => idx.sort_by(|a, b| {
                self.tracks[*b]
                    .display_title()
                    .cmp(self.tracks[*a].display_title())
            }),
            MusicSort::ArtistAsc => {
                idx.sort_by(|a, b| self.tracks[*a].artist.cmp(&self.tracks[*b].artist))
            }
            MusicSort::TimeDesc => {
                idx.sort_by(|a, b| self.tracks[*b].modified_secs.cmp(&self.tracks[*a].modified_secs))
            }
        }
        idx
    }

    pub fn group_entries(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = self
            .groups
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

pub struct MusicLibrary {
    state: Arc<Mutex<MusicState>>,
    config_path: PathBuf,
}

impl MusicLibrary {
    pub fn new() -> Result<Self> {
        let base = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("xplayer");
        fs::create_dir_all(&base)?;
        let config_path = base.join("music_library.json");

        let mut st = MusicState::default();
        if let Ok(text) = fs::read_to_string(&config_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(arr) = v.get("roots").and_then(|x| x.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            st.roots.push(PathBuf::from(s));
                        }
                    }
                }
            }
        }
        let lib = Self {
            state: Arc::new(Mutex::new(st)),
            config_path,
        };
        lib.rescan();
        Ok(lib)
    }

    pub fn state(&self) -> MusicState {
        self.state.lock().clone()
    }

    pub fn add_root(&self, path: PathBuf) {
        {
            let mut st = self.state.lock();
            if st.roots.iter().any(|r| r == &path) {
                return;
            }
            st.roots.push(path);
        }
        self.rescan();
        self.save_config();
    }

    pub fn remove_root(&self, path: &str) {
        {
            let mut st = self.state.lock();
            st.roots.retain(|r| r.to_string_lossy() != path);
            if st.selected_root.as_deref() == Some(path) {
                st.selected_root = None;
            }
        }
        self.rescan();
        self.save_config();
    }

    pub fn set_selected_root(&self, p: Option<String>) {
        self.state.lock().selected_root = p;
    }
    pub fn set_search(&self, q: String) {
        self.state.lock().search = q;
    }
    pub fn set_category(&self, c: Category) {
        let mut st = self.state.lock();
        st.category = c;
        st.drill = None;
        drop(st);
        self.rebuild_groups();
    }
    pub fn set_sort(&self, s: MusicSort) {
        self.state.lock().sort = s;
    }
    pub fn set_drill(&self, g: Option<String>) {
        self.state.lock().drill = g;
    }

    pub fn rescan(&self) {
        let roots = self.state.lock().roots.clone();
        self.state.lock().scanning = true;
        let mut tracks = Vec::new();
        for root in &roots {
            scan_dir(root, &mut tracks);
        }
        {
            let mut st = self.state.lock();
            st.tracks = tracks;
            st.scanning = false;
        }
        self.rebuild_groups();
    }

    fn rebuild_groups(&self) {
        let st = self.state.lock();
        let category = st.category;
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, t) in st.tracks.iter().enumerate() {
            let key = match category {
                Category::Songs => continue,
                Category::Albums => {
                    if t.album.is_empty() { "未知专辑" } else { &t.album }.to_string()
                }
                Category::Artists => {
                    if t.artist.is_empty() { "未知歌手" } else { &t.artist }.to_string()
                }
            };
            groups.entry(key).or_default().push(i);
        }
        drop(st);
        self.state.lock().groups = groups;
    }

    fn save_config(&self) {
        let st = self.state.lock();
        let roots: Vec<String> = st.roots.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        let v = serde_json::json!({ "roots": roots });
        let _ = fs::write(&self.config_path, serde_json::to_string_pretty(&v).unwrap_or_default());
    }
}

/// 歌词行：[mm:ss.xx]文本
#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
}

/// 读取歌词：优先内嵌 USLT，其次同名 .lrc / .LRC
pub fn load_lyrics(track: &Track) -> Vec<LyricLine> {
    if let Some(embedded) = &track.embedded_lyrics {
        let p = parse_lrc(embedded);
        if !p.is_empty() {
            return p;
        }
        // 内嵌纯文本歌词（无时间轴）：整体作为一行
        return embedded
            .lines()
            .map(|l| LyricLine { time: 0.0, text: l.to_string() })
            .collect();
    }
    let stem = track.path.with_extension("");
    for ext in ["lrc", "LRC"] {
        let p = stem.with_extension(ext);
        if let Ok(text) = fs::read_to_string(&p) {
            let lines = parse_lrc(&text);
            if !lines.is_empty() {
                return lines;
            }
        }
    }
    Vec::new()
}

pub fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // 允许一行多个时间标签
        let mut stamps = Vec::new();
        let mut rest = line;
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let tag = &rest[1..end];
            if let Some(t) = parse_stamp(tag) {
                stamps.push(t);
            }
            rest = &rest[end + 1..];
        }
        if stamps.is_empty() {
            // 元数据标签 [ar:] [ti:] 等，跳过
            continue;
        }
        let txt = rest.trim().to_string();
        for t in stamps {
            out.push(LyricLine { time: t, text: txt.clone() });
        }
    }
    out.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn parse_stamp(tag: &str) -> Option<f64> {
    let (m, s) = tag.split_once(':')?;
    let m: f64 = m.trim().parse().ok()?;
    let s: f64 = s.trim().parse().ok()?;
    Some(m * 60.0 + s)
}

// ---------------- 文件扫描与标签解析 ----------------

fn scan_dir(dir: &Path, out: &mut Vec<Track>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                if !MUSIC_EXTS.contains(&ext.as_str()) {
                    continue;
                }
                let Ok(meta) = fs::metadata(&p) else { continue };
                let name = p
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut t = Track {
                    path: p.clone(),
                    name: name.clone(),
                    title: name,
                    size: meta.len(),
                    modified_secs: meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    ..Default::default()
                };
                // 只读前 512KB，足够覆盖 ID3v2/FLAC 头与封面
                if let Ok(bytes) = read_prefix(&p, 512 * 1024) {
                    parse_tags(&bytes, &ext, &mut t);
                }
                // ID3v1 兜底（在文件尾部）
                if t.artist.is_empty() {
                    if let Ok(tail) = read_tail(&p, 128) {
                        parse_id3v1(&tail, &mut t);
                    }
                }
                out.push(t);
            }
        }
    }
}

fn read_prefix(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = fs::File::open(path)?;
    let mut buf = vec![0u8; max];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

fn read_tail(path: &Path, n: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path)?;
    let len = f.metadata()?.len();
    if len < n as u64 {
        return Ok(Vec::new());
    }
    f.seek(SeekFrom::End(-(n as i64)))?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn parse_tags(bytes: &[u8], ext: &str, t: &mut Track) {
    if bytes.starts_with(b"ID3") {
        parse_id3v2(bytes, t);
    } else if bytes.starts_with(b"fLaC") {
        parse_flac(bytes, t);
    } else if ext == "m4a" || ext == "aac" {
        // M4A 简易：直接搜 ASCII 元数据名（不严格解析 atom）
        parse_loose_ascii(bytes, t);
    }
    // 文件名兜底：把 "Artist - Title" 拆开
    if t.artist.is_empty() {
        if let Some((a, b)) = t.name.split_once(" - ") {
            t.artist = a.trim().to_string();
            t.title = b.trim().to_string();
        }
    }
}

fn parse_id3v2(b: &[u8], t: &mut Track) {
    if b.len() < 10 {
        return;
    }
    let ver = b[3]; // 2 / 3 / 4
    let size = synchsafe(&b[6..10]) as usize;
    let mut pos = 10usize;
    let end = (10 + size).min(b.len());
    // 跳过扩展头（v2.3/2.4）
    if b[5] & 0x40 != 0 && ver >= 3 && pos + 4 <= b.len() {
        let ext_size = if ver == 4 {
            synchsafe(&b[pos..pos + 4]) as usize
        } else {
            be_u32(&b[pos..pos + 4]) as usize
        };
        pos = (pos + 4 + ext_size.saturating_sub(4)).min(end);
    }
    while pos + 10 <= end {
        let id = &b[pos..pos + 4];
        let id3 = id.len() >= 3 && id[0] == 0 && id[1] == 0 && id[2] == 0;
        if id3 {
            break;
        }
        let fsize = if ver == 4 {
            synchsafe(&b[pos + 4..pos + 8]) as usize
        } else {
            be_u32(&b[pos + 4..pos + 8]) as usize
        };
        let body = pos + 10;
        let body_end = (body + fsize).min(end);
        if body >= body_end {
            break;
        }
        let data = &b[body..body_end];
        match &id[..3] {
            b"TIT" => t.title = decode_text_frame(data),
            b"TPE" => {
                if t.artist.is_empty() {
                    t.artist = decode_text_frame(data)
                }
            }
            b"TAL" => t.album = decode_text_frame(data),
            b"API" => {
                if t.cover.is_none() {
                    t.cover = parse_apic(data, ver).map(Arc::new);
                }
            }
            b"USL" | b"ULT" => {
                if t.embedded_lyrics.is_none() {
                    t.embedded_lyrics = Some(parse_uslt(data));
                }
            }
            _ => {}
        }
        pos = body_end;
        if fsize == 0 {
            break;
        }
    }
}

fn decode_text_frame(d: &[u8]) -> String {
    if d.is_empty() {
        return String::new();
    }
    decode_encoded(&d[1..], d[0])
}

fn decode_encoded(d: &[u8], enc: u8) -> String {
    let raw: Vec<u8> = match enc {
        0 => d.to_vec(), // ISO-8859-1 -> 近似按 UTF-8 处理（中文标签多为 UTF-16/8）
        1 | 2 => decode_utf16(d, enc == 2),
        3 => d.to_vec(),
        _ => d.to_vec(),
    };
    let s = String::from_utf8_lossy(&raw);
    s.split('\0')
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| c == '\0' || c.is_control())
        .to_string()
}

fn decode_utf16(d: &[u8], big_endian: bool) -> Vec<u8> {
    let mut be = big_endian;
    let mut start = 0;
    if d.len() >= 2 {
        if d[0] == 0xff && d[1] == 0xfe {
            be = false;
            start = 2;
        } else if d[0] == 0xfe && d[1] == 0xff {
            be = true;
            start = 2;
        }
    }
    let mut units = Vec::new();
    let mut i = start;
    while i + 1 < d.len() {
        let u = if be {
            ((d[i] as u16) << 8) | d[i + 1] as u16
        } else {
            ((d[i + 1] as u16) << 8) | d[i] as u16
        };
        if u == 0 {
            break;
        }
        units.push(u);
        i += 2;
    }
    String::from_utf16_lossy(&units).into_bytes()
}

/// APIC: enc, mime\0, picType, desc\0, image data
fn parse_apic(d: &[u8], ver: u8) -> Option<Vec<u8>> {
    if d.len() < 4 {
        return None;
    }
    let enc = d[0];
    let mut i = 1;
    // mime 直到 0（v2.2 是 3 字节 "JPG"/"PNG"）
    let mime_end = if ver == 2 {
        (i + 3).min(d.len())
    } else {
        d[i..].iter().position(|&x| x == 0).map(|p| i + p).unwrap_or(d.len())
    };
    i = mime_end + if ver == 2 { 0 } else { 1 };
    if i >= d.len() {
        return None;
    }
    i += 1; // picture type
    // description：enc=1/2 时是 UTF-16，以 00 00 结束
    let desc_end = match enc {
        1 | 2 => {
            let mut j = i;
            while j + 1 < d.len() && !(d[j] == 0 && d[j + 1] == 0) {
                j += 2;
            }
            (j + 2).min(d.len())
        }
        _ => d[i..]
            .iter()
            .position(|&x| x == 0)
            .map(|p| i + p + 1)
            .unwrap_or(d.len()),
    };
    i = desc_end;
    if i >= d.len() {
        return None;
    }
    let img = d[i..].to_vec();
    if img.len() > 16 { Some(img) } else { None }
}

fn parse_uslt(d: &[u8]) -> String {
    if d.len() < 7 {
        return String::new();
    }
    let enc = d[0];
    let mut i = 4; // enc + 3 字节语言
    let desc_end = match enc {
        1 | 2 => {
            let mut j = i;
            while j + 1 < d.len() && !(d[j] == 0 && d[j + 1] == 0) {
                j += 2;
            }
            (j + 2).min(d.len())
        }
        _ => d[i..]
            .iter()
            .position(|&x| x == 0)
            .map(|p| i + p + 1)
            .unwrap_or(d.len()),
    };
    i = desc_end;
    decode_encoded(&d[i.min(d.len())..], enc)
}

/// FLAC: "fLaC" 后是一串 metadata block
fn parse_flac(b: &[u8], t: &mut Track) {
    let mut pos = 4usize;
    while pos + 4 <= b.len() {
        let last = b[pos] & 0x80 != 0;
        let kind = b[pos] & 0x7f;
        let len = be_u24(&b[pos + 1..pos + 4]) as usize;
        let body = pos + 4;
        let end = (body + len).min(b.len());
        if body >= end {
            break;
        }
        let d = &b[body..end];
        match kind {
            4 => parse_vorbis_comment(d, t), // VORBIS_COMMENT
            6 => {
                if t.cover.is_none() {
                    t.cover = parse_flac_picture(d).map(Arc::new);
                }
            }
            _ => {}
        }
        if last {
            break;
        }
        pos = end;
    }
}

fn parse_vorbis_comment(d: &[u8], t: &mut Track) {
    // little-endian 长度前缀
    let mut p = 0usize;
    fn read_le<'a>(d: &'a [u8], p: &mut usize) -> Option<&'a [u8]> {
        if *p + 4 > d.len() {
            return None;
        }
        let n = u32::from_le_bytes([d[*p], d[*p + 1], d[*p + 2], d[*p + 3]]) as usize;
        *p += 4;
        if *p + n > d.len() {
            return None;
        }
        let s = &d[*p..*p + n];
        *p += n;
        Some(s)
    }
    let Some(_vendor) = read_le(d, &mut p) else { return };
    if p + 4 > d.len() {
        return;
    }
    let count = u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]) as usize;
    p += 4;
    for _ in 0..count {
        let Some(entry) = read_le(d, &mut p) else { break };
        let s = String::from_utf8_lossy(entry);
        if let Some((k, v)) = s.split_once('=') {
            let v = v.trim().to_string();
            match k.to_uppercase().as_str() {
                "TITLE" => t.title = v,
                "ARTIST" => {
                    if t.artist.is_empty() {
                        t.artist = v
                    }
                }
                "ALBUM" => t.album = v,
                "LYRICS" | "UNSYNCEDLYRICS" => t.embedded_lyrics = Some(v),
                _ => {}
            }
        }
    }
}

fn parse_flac_picture(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 32 {
        return None;
    }
    let mut p = 0usize;
    p += 4; // picture type
    let mime_len = be_u32(&d[p..p + 4]) as usize;
    p += 4 + mime_len;
    let desc_len = be_u32(&d[p..p + 4]) as usize;
    p += 4 + desc_len;
    p += 16; // width/height/depth/colors
    let data_len = be_u32(&d[p..p + 4]) as usize;
    p += 4;
    if p + data_len > d.len() {
        return None;
    }
    let img = d[p..p + data_len].to_vec();
    if img.len() > 16 { Some(img) } else { None }
}

fn parse_loose_ascii(b: &[u8], t: &mut Track) {
    // M4A 兜底：在头部字节里找 ©nam / ©ART / ©alb 附近的可见文本
    for (tag, field) in [
        (&b"\xa9nam"[..], 0u8),
        (&b"\xa9ART"[..], 1u8),
        (&b"\xa9alb"[..], 2u8),
    ] {
        if let Some(i) = b.windows(tag.len()).position(|w| w == tag) {
            let start = i + tag.len();
            let end = (start + 120).min(b.len());
            let chunk = &b[start..end];
            let s: String = chunk
                .iter()
                .skip_while(|&&c| c < 0x20 || c > 0x7e && c < 0xc0)
                .take_while(|&&c| c >= 0x20)
                .map(|&c| c as char)
                .collect();
            let s = s.trim().to_string();
            if !s.is_empty() {
                match field {
                    0 => t.title = s,
                    1 => t.artist = s,
                    _ => t.album = s,
                }
            }
        }
    }
}

fn parse_id3v1(tail: &[u8], t: &mut Track) {
    if tail.len() < 128 || &tail[..3] != b"TAG" {
        return;
    }
    let get = |a: usize, b: usize| -> String {
        String::from_utf8_lossy(&tail[a..b])
            .trim_matches(|c: char| c == '\0' || c == ' ' || c.is_control())
            .to_string()
    };
    let title = get(3, 33);
    let artist = get(33, 63);
    let album = get(63, 93);
    if !title.is_empty() {
        t.title = title;
    }
    if !artist.is_empty() {
        t.artist = artist;
    }
    if !album.is_empty() {
        t.album = album;
    }
}

fn synchsafe(d: &[u8]) -> u32 {
    ((d[0] as u32 & 0x7f) << 21)
        | ((d[1] as u32 & 0x7f) << 14)
        | ((d[2] as u32 & 0x7f) << 7)
        | (d[3] as u32 & 0x7f)
}
fn be_u32(d: &[u8]) -> u32 {
    ((d[0] as u32) << 24) | ((d[1] as u32) << 16) | ((d[2] as u32) << 8) | d[3] as u32
}
fn be_u24(d: &[u8]) -> u32 {
    ((d[0] as u32) << 16) | ((d[1] as u32) << 8) | d[2] as u32
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
