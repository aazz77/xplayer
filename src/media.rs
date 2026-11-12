//! 图片解码：磁盘缩略图 / 内存封面字节 -> GPUI `RenderImage`。
//!
//! GPUI 按 **BGRA** 解释 `image::Frame` 字节，因此把解码出的 RGBA
//! 通道 R/B 交换后再构造（与 nyaterm 的 `pixel.0.swap(0,2)` 一致）。

use gpui::RenderImage;
use image::RgbaImage;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

/// 简单的进程内图片缓存：key -> RenderImage
pub struct ImageCache {
    map: Mutex<HashMap<String, Arc<RenderImage>>>,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    /// 从磁盘文件加载（缩略图 PNG）。失败返回 None 且不缓存。
    pub fn from_file(&self, path: &str) -> Option<Arc<RenderImage>> {
        if path.is_empty() || !Path::new(path).is_file() {
            return None;
        }
        let key = path.to_string();
        if let Some(img) = self.map.lock().get(&key) {
            return Some(img.clone());
        }
        let bytes = std::fs::read(&key).ok()?;
        let img = decode(&bytes)?;
        self.map.lock().insert(key, img.clone());
        Some(img)
    }

    /// 从内存字节加载（音乐封面）。key 一般取曲目路径。
    pub fn from_bytes(&self, key: &str, bytes: &[u8]) -> Option<Arc<RenderImage>> {
        if bytes.is_empty() {
            return None;
        }
        if let Some(img) = self.map.lock().get(key) {
            return Some(img.clone());
        }
        let img = decode(bytes)?;
        self.map.lock().insert(key.to_string(), img.clone());
        Some(img)
    }

    pub fn clear(&self) {
        self.map.lock().clear();
    }
}

fn decode(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(64 * 1024 * 1024);
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    let mut rgba: RgbaImage = reader.decode().ok()?.to_rgba8();
    // RGBA -> BGRA
    for p in rgba.pixels_mut() {
        p.0.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}
