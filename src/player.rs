//! libmpv2 播放器：软件渲染取帧 -> GPUI `img()` 显示。
//!
//! 关键链路（修复「只有声音没画面」）：
//!   1. `vo=libmpv`：mpv 不建窗口，画面必须由宿主取帧。
//!   2. 用 SW render API 把每帧渲染进 `rgb0` 缓冲，字节序为 **B,G,R,X**。
//!   3. GPUI 的 `RenderImage` 按 **BGRA** 解释 `image::Frame` 字节
//!      （nyaterm 的 `pixel.0.swap(0,2)` 已证实），所以只需把第 4 字节
//!      X(0) 补成 255(alpha)，即可零通道转换直接显示。
//!   4. UI 每帧 `cx.notify()` 后取 `latest_image()` 交给 `img()`。
//!
//! 硬/软解：全部走 mpv `hwdec`（auto-copy / no），不引入 VLC。
//! `auto-copy` 会把 GPU 解码帧拷回系统内存，保证 SW render 仍能取到帧。

use anyhow::{bail, Context, Result};
use gpui::RenderImage;
use libmpv2::{events::Event, Format, Mpv};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::settings::{DecodeMode, Quality, Settings};

#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub path: Option<PathBuf>,
    pub title: String,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: i64,
    pub mute: bool,
    pub eof: bool,
    pub error: Option<String>,
    pub frame_w: u32,
    pub frame_h: u32,
    /// 是否拿得到视频帧（纯音频为 false）
    pub has_video: bool,
}

pub struct PlayerHandle {
    mpv: Arc<Mutex<Option<Mpv>>>,
    state: Arc<Mutex<PlaybackState>>,
    /// 已转换好的 GPUI 图像，UI 直接 `img(arc)`
    image: Arc<Mutex<Option<Arc<RenderImage>>>>,
    /// 帧序号，UI 用它判断是否需要刷新
    frame_seq: Arc<AtomicU64>,
    target_size: Arc<Mutex<(u32, u32)>>,
    /// 当前加载的路径/URL，用于切换解码方式后重载
    current: Arc<Mutex<String>>,
    shutdown: Arc<AtomicBool>,
    _worker: Option<thread::JoinHandle<()>>,
}

impl PlayerHandle {
    pub fn new(settings: &Settings) -> Result<Self> {
        let mpv = Mpv::with_initializer(|init| {
            init.set_property("vo", "libmpv")?;
            // 解码方式由设置决定；默认软解保证一定能出画
            init.set_property("hwdec", settings.decode.hwdec())?;
            init.set_property("keep-open", "yes")?;
            init.set_property("idle", "yes")?;
            init.set_property("osc", "no")?;
            init.set_property("input-default-bindings", "no")?;
            init.set_property("input-vo-keyboard", "no")?;
            init.set_property("terminal", "no")?;
            init.set_property("msg-level", "all=error")?;
            init.set_property("video-timing-offset", 0)?;
            init.set_property("volume", settings.volume)?;
            init.set_property("speed", settings.rate)?;
            for (k, v) in settings.quality.props() {
                let _ = init.set_property(*k, *v);
            }
            Ok(())
        })
        .context("create libmpv")?;

        let mpv = Arc::new(Mutex::new(Some(mpv)));
        let state = Arc::new(Mutex::new(PlaybackState {
            volume: settings.volume,
            ..Default::default()
        }));
        let image = Arc::new(Mutex::new(None));
        let frame_seq = Arc::new(AtomicU64::new(0));
        let target_size = Arc::new(Mutex::new((960u32, 540u32)));
        let current = Arc::new(Mutex::new(String::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        {
            let mut guard = mpv.lock();
            if let Some(m) = guard.as_mut() {
                let _ = m.observe_property("time-pos", Format::Double, 0);
                let _ = m.observe_property("duration", Format::Double, 0);
                let _ = m.observe_property("pause", Format::Flag, 0);
                let _ = m.observe_property("volume", Format::Int64, 0);
                let _ = m.observe_property("mute", Format::Flag, 0);
                let _ = m.observe_property("media-title", Format::String, 0);
            }
        }

        let mpv_c = Arc::clone(&mpv);
        let state_c = Arc::clone(&state);
        let image_c = Arc::clone(&image);
        let seq_c = Arc::clone(&frame_seq);
        let size_c = Arc::clone(&target_size);
        let shutdown_c = Arc::clone(&shutdown);

        let worker = thread::Builder::new()
            .name("mpv-render".into())
            .spawn(move || {
                if let Err(e) = render_loop(mpv_c, state_c, image_c, seq_c, size_c, shutdown_c) {
                    tracing::error!(error = %e, "mpv render loop exited");
                }
            })
            .ok();

        Ok(Self {
            mpv,
            state,
            image,
            frame_seq,
            target_size,
            current,
            shutdown,
            _worker: worker,
        })
    }

    pub fn state(&self) -> PlaybackState {
        self.state.lock().clone()
    }

    pub fn frame_seq(&self) -> u64 {
        self.frame_seq.load(Ordering::Relaxed)
    }

    /// UI 取当前画面。`img(arc)` 直接可用。
    pub fn latest_image(&self) -> Option<Arc<RenderImage>> {
        self.image.lock().clone()
    }

    /// 播放区尺寸变化时调用，决定 SW render 的输出分辨率。
    pub fn set_target_size(&self, w: u32, h: u32) {
        if w >= 16 && h >= 16 && w <= 2560 && h <= 1440 {
            let mut t = self.target_size.lock();
            if t.0 != w || t.1 != h {
                *t = (w, h);
            }
        }
    }

    pub fn load(&self, path: &Path) -> Result<()> {
        let path_str = path.to_str().context("path is not valid UTF-8")?;
        self.load_str(path_str)
    }

    /// 加载本地路径或网络 URL（http/rtmp/rtsp/...）。
    pub fn load_str(&self, path_or_url: &str) -> Result<()> {
        let path_str = path_or_url.trim();
        if path_str.is_empty() {
            anyhow::bail!("empty path/url");
        }
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        mpv.command("loadfile", &[path_str, "replace"])
            .context("loadfile")?;
        drop(guard);

        *self.current.lock() = path_str.to_string();
        let mut s = self.state.lock();
        let p = PathBuf::from(path_str);
        s.path = Some(p.clone());
        s.title = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| path_str.to_string());
        s.position = 0.0;
        s.duration = 0.0;
        s.eof = false;
        s.paused = false;
        s.error = None;
        s.has_video = false;
        drop(s);
        *self.image.lock() = None;
        Ok(())
    }

    pub fn toggle_pause(&self) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        let paused: bool = mpv.get_property("pause").unwrap_or(false);
        mpv.set_property("pause", !paused)?;
        self.state.lock().paused = !paused;
        Ok(())
    }

    pub fn set_pause(&self, paused: bool) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        mpv.set_property("pause", paused)?;
        self.state.lock().paused = paused;
        Ok(())
    }

    pub fn seek_relative(&self, seconds: f64) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        let s = seconds.to_string();
        mpv.command("seek", &[&s, "relative"])?;
        Ok(())
    }

    pub fn seek_absolute(&self, seconds: f64) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        let s = seconds.to_string();
        mpv.command("seek", &[&s, "absolute"])?;
        Ok(())
    }

    pub fn set_volume(&self, volume: i64) -> Result<()> {
        let volume = volume.clamp(0, 150);
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        mpv.set_property("volume", volume)?;
        self.state.lock().volume = volume;
        Ok(())
    }

    pub fn toggle_mute(&self) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        let mute: bool = mpv.get_property("mute").unwrap_or(false);
        mpv.set_property("mute", !mute)?;
        self.state.lock().mute = !mute;
        Ok(())
    }

    pub fn set_rate(&self, rate: f64) -> Result<()> {
        let rate = rate.clamp(0.25, 4.0);
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        mpv.set_property("speed", rate)?;
        Ok(())
    }

    /// 切换硬/软解。mpv 的 hwdec 变更只对下一个文件生效，
    /// 所以若正在播放则记录进度并重载。
    pub fn set_decode(&self, mode: DecodeMode) -> Result<()> {
        let pos = {
            let mut guard = self.mpv.lock();
            let mpv = guard.as_mut().context("mpv shut down")?;
            mpv.set_property("hwdec", mode.hwdec())?;
            mpv.get_property::<f64>("time-pos").unwrap_or(0.0)
        };
        let cur = self.current.lock().clone();
        if !cur.is_empty() {
            self.load_str(&cur)?;
            if pos > 1.0 {
                let _ = self.seek_absolute(pos);
            }
        }
        Ok(())
    }

    pub fn set_quality(&self, q: Quality) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        for (k, v) in q.props() {
            let _ = mpv.set_property(*k, *v);
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let mut guard = self.mpv.lock();
        let mpv = guard.as_mut().context("mpv shut down")?;
        mpv.command("stop", &[])?;
        drop(guard);
        *self.current.lock() = String::new();
        let mut s = self.state.lock();
        s.path = None;
        s.title.clear();
        s.position = 0.0;
        s.duration = 0.0;
        s.paused = true;
        s.has_video = false;
        drop(s);
        *self.image.lock() = None;
        Ok(())
    }
}

impl Drop for PlayerHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        *self.mpv.lock() = None;
    }
}

/// 把 mpv 的 B,G,R,X 缓冲转成 GPUI 可显示的 RenderImage。
/// 只补 alpha=255，不交换通道。
fn frame_to_render_image(w: u32, h: u32, bgrx: Vec<u8>) -> Option<Arc<RenderImage>> {
    let mut px = bgrx;
    let need = (w as usize) * (h as usize) * 4;
    if px.len() < need {
        px.resize(need, 0);
    }
    px.truncate(need);
    // rgb0 第 4 字节为 0；GPUI 按 BGRA 读，alpha=0 会全透明 -> 补 255
    for a in px.chunks_exact_mut(4) {
        a[3] = 255;
    }
    let rgba = image::RgbaImage::from_raw(w, h, px)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}

#[allow(clippy::too_many_arguments)]
fn render_loop(
    mpv_arc: Arc<Mutex<Option<Mpv>>>,
    state: Arc<Mutex<PlaybackState>>,
    image_out: Arc<Mutex<Option<Arc<RenderImage>>>>,
    seq: Arc<AtomicU64>,
    target_size: Arc<Mutex<(u32, u32)>>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    use libmpv2_sys as sys;
    use std::ffi::CString;

    let render_ctx = {
        let guard = mpv_arc.lock();
        let mpv = guard.as_ref().context("mpv missing at render start")?;
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
            bail!("mpv_render_context_create(SW) failed: {err}");
        }
        ctx
    };

    let mut pixels: Vec<u8> = Vec::new();
    let mut last_w = 0u32;
    let mut last_h = 0u32;

    while !shutdown.load(Ordering::Relaxed) {
        {
            let mut guard = mpv_arc.lock();
            if let Some(mpv) = guard.as_mut() {
                loop {
                    match mpv.wait_event(0.0) {
                        Some(Ok(Event::Shutdown)) | None => break,
                        Some(Ok(Event::EndFile(_))) => {
                            let mut s = state.lock();
                            s.eof = true;
                            s.paused = true;
                        }
                        Some(Ok(Event::PropertyChange { name, change, .. })) => {
                            let mut s = state.lock();
                            match name.as_ref() {
                                "time-pos" => {
                                    if let libmpv2::events::PropertyData::Double(v) = change {
                                        s.position = v;
                                    }
                                }
                                "duration" => {
                                    if let libmpv2::events::PropertyData::Double(v) = change {
                                        s.duration = v;
                                    }
                                }
                                "pause" => {
                                    if let libmpv2::events::PropertyData::Flag(v) = change {
                                        s.paused = v;
                                    }
                                }
                                "volume" => {
                                    if let libmpv2::events::PropertyData::Int64(v) = change {
                                        s.volume = v;
                                    }
                                }
                                "mute" => {
                                    if let libmpv2::events::PropertyData::Flag(v) = change {
                                        s.mute = v;
                                    }
                                }
                                "media-title" => {
                                    if let libmpv2::events::PropertyData::Str(v) = change {
                                        s.title = v.to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            state.lock().error = Some(e.to_string());
                        }
                    }
                }
                if let Ok(pos) = mpv.get_property::<f64>("time-pos") {
                    state.lock().position = pos;
                }
                if let Ok(dur) = mpv.get_property::<f64>("duration") {
                    state.lock().duration = dur;
                }
            } else {
                break;
            }
        }

        let flags = unsafe { sys::mpv_render_context_update(render_ctx) };
        let need_render = (flags as u64)
            & (sys::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64)
            != 0;

        if need_render {
            let (tw, th) = *target_size.lock();
            let w = tw.max(16);
            let h = th.max(16);

            if w != last_w || h != last_h {
                pixels.resize((w as usize) * (h as usize) * 4, 0);
                last_w = w;
                last_h = h;
            }

            let size = [w as i32, h as i32];
            let format = CString::new("rgb0").unwrap();
            let stride: usize = (w as usize) * 4;
            let ptr = pixels.as_mut_ptr();

            let mut params = [
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
                    data: ptr as *mut _,
                },
                sys::mpv_render_param {
                    type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                    data: ptr::null_mut(),
                },
            ];

            let err = unsafe { sys::mpv_render_context_render(render_ctx, params.as_mut_ptr()) };
            if err >= 0 {
                if let Some(img) = frame_to_render_image(w, h, pixels.clone()) {
                    *image_out.lock() = Some(img);
                    seq.fetch_add(1, Ordering::Relaxed);
                    let mut s = state.lock();
                    s.frame_w = w;
                    s.frame_h = h;
                    s.has_video = true;
                }
            }
        }

        thread::sleep(Duration::from_millis(if need_render { 8 } else { 16 }));
    }

    unsafe {
        sys::mpv_render_context_free(render_ctx);
    }
    Ok(())
}
