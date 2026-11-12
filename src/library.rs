//! Local video library: scan, filter, sort, thumbnail cache (libmpv SW frame).

use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "ts", "m4v", "mpg", "mpeg", "m2ts", "3gp",
];

const THUMB_MAX_CONCURRENT: usize = 2;
const THUMB_SIZE: u32 = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Name,
    Size,
    Ext,
}

impl SortMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SortMode::Name => "name",
            SortMode::Size => "size",
            SortMode::Ext => "ext",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "size" => SortMode::Size,
            "ext" => SortMode::Ext,
            _ => SortMode::Name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Default)]
pub struct LibraryState {
    pub roots: Vec<PathBuf>,
    pub files_by_root: HashMap<String, Vec<VideoFile>>,
    pub selected_root: Option<String>,
    pub search: String,
    pub sort: SortMode,
    pub scanning: bool,
    /// video abs path -> thumb abs path; "" means tried & failed
    pub thumbnails: HashMap<String, String>,
}

impl LibraryState {
    pub fn all_files(&self) -> Vec<VideoFile> {
        self.files_by_root.values().flatten().cloned().collect()
    }

    pub fn source_files(&self) -> Vec<VideoFile> {
        match &self.selected_root {
            Some(r) => self.files_by_root.get(r).cloned().unwrap_or_default(),
            None => self.all_files(),
        }
    }

    pub fn filtered(&self) -> Vec<VideoFile> {
        let mut list = self.source_files();
        let q = self.search.to_lowercase();
        if !q.is_empty() {
            list.retain(|f| f.name.to_lowercase().contains(&q));
        }
        match self.sort {
            SortMode::Name => list.sort_by(|a, b| a.name.cmp(&b.name)),
            SortMode::Size => list.sort_by(|a, b| b.size.cmp(&a.size)),
            SortMode::Ext => list.sort_by(|a, b| {
                let ea = Path::new(&a.name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let eb = Path::new(&b.name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                ea.cmp(eb)
            }),
        }
        list
    }

    pub fn total_count(&self) -> usize {
        self.files_by_root.values().map(|v| v.len()).sum()
    }
}

pub struct Library {
    state: Arc<Mutex<LibraryState>>,
    config_path: PathBuf,
    thumbs_dir: PathBuf,
    thumb_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    thumb_running: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
}

impl Library {
    pub fn new() -> Result<Self> {
        let base = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mpv-gpui-player");
        fs::create_dir_all(&base)?;
        let thumbs = base.join("thumbs");
        fs::create_dir_all(&thumbs)?;
        let config_path = base.join("library.json");

        let mut state = LibraryState {
            sort: SortMode::Name,
            ..Default::default()
        };

        if let Ok(text) = fs::read_to_string(&config_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(arr) = v.get("roots").and_then(|x| x.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            state.roots.push(PathBuf::from(s));
                        }
                    }
                }
                if let Some(s) = v.get("sort").and_then(|x| x.as_str()) {
                    state.sort = SortMode::from_str(s);
                }
                if let Some(map) = v.get("thumbnails").and_then(|x| x.as_object()) {
                    for (k, val) in map {
                        if let Some(s) = val.as_str() {
                            state.thumbnails.insert(k.clone(), s.to_string());
                        }
                    }
                }
            }
        }

        let lib = Self {
            state: Arc::new(Mutex::new(state)),
            config_path,
            thumbs_dir: thumbs,
            thumb_queue: Arc::new(Mutex::new(VecDeque::new())),
            thumb_running: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        lib.rescan();
        Ok(lib)
    }

    pub fn state(&self) -> LibraryState {
        self.state.lock().clone()
    }

    fn save_config(&self) {
        let st = self.state.lock();
        let roots: Vec<String> = st
            .roots
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let thumbs: HashMap<String, String> = st.thumbnails.clone();
        let v = serde_json::json!({
            "roots": roots,
            "sort": st.sort.as_str(),
            "thumbnails": thumbs,
        });
        let _ = fs::write(
            &self.config_path,
            serde_json::to_string_pretty(&v).unwrap_or_default(),
        );
    }

    pub fn add_root(&self, path: PathBuf) {
        {
            let mut st = self.state.lock();
            if st.roots.iter().any(|r| r == &path) {
                return;
            }
            st.roots.push(path.clone());
            st.scanning = true;
        }
        let key = path.to_string_lossy().into_owned();
        let files = scan_path(&path);
        {
            let mut st = self.state.lock();
            st.files_by_root.insert(key, files.clone());
            st.scanning = false;
        }
        self.save_config();
        self.enqueue_missing_thumbs(&files);
    }

    pub fn remove_root(&self, path: &str) {
        {
            let mut st = self.state.lock();
            st.roots.retain(|r| r.to_string_lossy() != path);
            st.files_by_root.remove(path);
            if st.selected_root.as_deref() == Some(path) {
                st.selected_root = None;
            }
        }
        self.save_config();
    }

    pub fn set_selected_root(&self, path: Option<String>) {
        self.state.lock().selected_root = path;
    }

    pub fn set_search(&self, q: String) {
        self.state.lock().search = q;
    }

    pub fn set_sort(&self, mode: SortMode) {
        self.state.lock().sort = mode;
        self.save_config();
    }

    pub fn rescan(&self) {
        let roots = self.state.lock().roots.clone();
        {
            let mut st = self.state.lock();
            st.scanning = true;
            st.files_by_root.clear();
        }
        let mut all = Vec::new();
        for root in roots {
            let key = root.to_string_lossy().into_owned();
            let files = scan_path(&root);
            all.extend(files.iter().cloned());
            self.state.lock().files_by_root.insert(key, files);
        }
        self.state.lock().scanning = false;
        self.enqueue_missing_thumbs(&all);
    }

    pub fn thumbs_dir(&self) -> &Path {
        &self.thumbs_dir
    }

    fn enqueue_missing_thumbs(&self, files: &[VideoFile]) {
        let mut q = self.thumb_queue.lock();
        for f in files {
            let key = f.path.to_string_lossy().into_owned();
            let st = self.state.lock();
            if let Some(cached) = st.thumbnails.get(&key) {
                if cached.is_empty() {
                    continue;
                }
                if Path::new(cached).is_file() {
                    continue;
                }
            }
            drop(st);
            if q.iter().any(|p| p == &f.path) {
                continue;
            }
            q.push_back(f.path.clone());
        }
        drop(q);
        self.pump_thumbs();
    }

    fn pump_thumbs(&self) {
        while self.thumb_running.load(Ordering::SeqCst) < THUMB_MAX_CONCURRENT {
            let next = self.thumb_queue.lock().pop_front();
            let Some(path) = next else { break };
            self.thumb_running.fetch_add(1, Ordering::SeqCst);

            let state = Arc::clone(&self.state);
            let thumbs_dir = self.thumbs_dir.clone();
            let running = Arc::clone(&self.thumb_running);
            let queue = Arc::clone(&self.thumb_queue);
            let shutdown = Arc::clone(&self.shutdown);
            let config_path = self.config_path.clone();

            thread::Builder::new()
                .name("thumb".into())
                .spawn(move || {
                    let key = path.to_string_lossy().into_owned();
                    let result = generate_thumbnail_mpv(&path, &thumbs_dir);
                    {
                        let mut st = state.lock();
                        match result {
                            Ok(p) => {
                                st.thumbnails
                                    .insert(key, p.to_string_lossy().into_owned());
                            }
                            Err(_) => {
                                st.thumbnails.insert(key, String::new());
                            }
                        }
                        // persist thumbs map
                        let roots: Vec<String> = st
                            .roots
                            .iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect();
                        let v = serde_json::json!({
                            "roots": roots,
                            "sort": st.sort.as_str(),
                            "thumbnails": st.thumbnails,
                        });
                        let _ = fs::write(
                            &config_path,
                            serde_json::to_string_pretty(&v).unwrap_or_default(),
                        );
                    }
                    running.fetch_sub(1, Ordering::SeqCst);
                    if !shutdown.load(Ordering::Relaxed) {
                        // continue queue
                        while running.load(Ordering::SeqCst) < THUMB_MAX_CONCURRENT {
                            let n = queue.lock().pop_front();
                            let Some(p2) = n else { break };
                            running.fetch_add(1, Ordering::SeqCst);
                            let state2 = Arc::clone(&state);
                            let thumbs2 = thumbs_dir.clone();
                            let running2 = Arc::clone(&running);
                            let config2 = config_path.clone();
                            thread::spawn(move || {
                                let key = p2.to_string_lossy().into_owned();
                                let result = generate_thumbnail_mpv(&p2, &thumbs2);
                                {
                                    let mut st = state2.lock();
                                    match result {
                                        Ok(p) => {
                                            st.thumbnails
                                                .insert(key, p.to_string_lossy().into_owned());
                                        }
                                        Err(_) => {
                                            st.thumbnails.insert(key, String::new());
                                        }
                                    }
                                    let roots: Vec<String> = st
                                        .roots
                                        .iter()
                                        .map(|p| p.to_string_lossy().into_owned())
                                        .collect();
                                    let v = serde_json::json!({
                                        "roots": roots,
                                        "sort": st.sort.as_str(),
                                        "thumbnails": st.thumbnails,
                                    });
                                    let _ = fs::write(
                                        &config2,
                                        serde_json::to_string_pretty(&v).unwrap_or_default(),
                                    );
                                }
                                running2.fetch_sub(1, Ordering::SeqCst);
                            });
                        }
                    }
                })
                .ok();
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

/// Grab one frame via a short-lived libmpv instance + SW render, save PNG.
fn generate_thumbnail_mpv(video: &Path, thumbs_dir: &Path) -> Result<PathBuf> {
    use libmpv2::Mpv;
    use libmpv2_sys as sys;
    use std::ffi::CString;
    use std::ptr;

    let hash = stable_hash(&video.to_string_lossy());
    let out = thumbs_dir.join(format!("{hash}.png"));
    if out.is_file() {
        return Ok(out);
    }

    let path_str = video
        .to_str()
        .context("path utf-8")?
        .to_string();

    let mpv = Mpv::with_initializer(|init| {
        init.set_property("vo", "libmpv")?;
        init.set_property("aid", "no")?;
        init.set_property("sid", "no")?;
        init.set_property("pause", true)?;
        init.set_property("terminal", "no")?;
        init.set_property("msg-level", "all=no")?;
        init.set_property("idle", "yes")?;
        Ok(())
    })?;

    mpv.command("loadfile", &[&path_str, "replace"])?;

    // Wait for duration / demux a bit
    let mut duration = 0.0f64;
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(50));
        if let Ok(d) = mpv.get_property::<f64>("duration") {
            if d > 0.0 {
                duration = d;
                break;
            }
        }
    }
    let seek_to = if duration > 0.0 { duration * 0.1 } else { 0.0 };
    if seek_to > 0.0 {
        let seek_s = format!("{seek_to}");
        let _ = mpv.command("seek", &[&seek_s, "absolute"]);
        thread::sleep(Duration::from_millis(200));
    }

    let handle = mpv.ctx.as_ptr();
    let api_type = CString::new("sw").unwrap();
    let mut params = [
        sys::mpv_render_param {
            type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            data: api_type.as_ptr() as *mut _,
        },
        sys::mpv_render_param {
            type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        },
    ];
    let mut ctx: *mut sys::mpv_render_context = ptr::null_mut();
    let err = unsafe { sys::mpv_render_context_create(&mut ctx, handle, params.as_mut_ptr()) };
    if err < 0 || ctx.is_null() {
        anyhow::bail!("thumb render context failed: {err}");
    }

    // Force a few update/render cycles
    let w = THUMB_SIZE;
    let h = THUMB_SIZE;
    let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
    let size = [w as i32, h as i32];
    let format = CString::new("rgb0").unwrap();
    let stride: usize = (w as usize) * 4;

    for _ in 0..10 {
        unsafe {
            sys::mpv_render_context_update(ctx);
        }
        let mut rparams = [
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                data: size.as_ptr() as *mut _,
            },
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                data: format.as_ptr() as *mut _,
            },
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                data: &stride as *const _ as *mut _,
            },
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                data: pixels.as_mut_ptr() as *mut _,
            },
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];
        let e = unsafe { sys::mpv_render_context_render(ctx, rparams.as_mut_ptr()) };
        if e >= 0 {
            // Check non-black-ish
            let sample = pixels.iter().step_by(16).filter(|b| **b > 8).count();
            if sample > 4 {
                break;
            }
        }
        thread::sleep(Duration::from_millis(40));
    }

    unsafe {
        sys::mpv_render_context_free(ctx);
    }

    // BGRA -> RGBA PNG
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) as usize) * 4;
            let b = pixels[i];
            let g = pixels[i + 1];
            let r = pixels[i + 2];
            let a = 255u8;
            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
        }
    }
    img.save(&out).context("save thumb png")?;
    Ok(out)
}

fn stable_hash(input: &str) -> String {
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash: u32 = 0x811c_9dc5;
    for b in input.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:08x}")
}

fn scan_path(dir: &Path) -> Vec<VideoFile> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = match fs::read_dir(&d) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let p = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                let ext = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                if !VIDEO_EXTS.contains(&ext.as_str()) {
                    continue;
                }
                let meta = match fs::metadata(&p) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                out.push(VideoFile {
                    path: p,
                    name,
                    size: meta.len(),
                    modified: meta.modified().ok(),
                });
            }
        }
    }
    out
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
