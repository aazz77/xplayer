# mpv-gpui-player

Nexus（Windows 端）风格的本地播放器，Rust + GPUI + libmpv。**浅色主题**、**仅 MPV 后端**、支持**硬解/软解切换**。

## 本次改动

**1. 修复「只有声音没画面」**

原项目 mpv 用 `vo=libmpv`（不创建自己的窗口），`player.rs` 已经用 SW render API 把每帧 BGRA 抓进内存，但播放页只有占位文字，没有任何一处把帧画出来 —— 音频照常出声、画面全黑。

现在：

- 帧缓冲转成 `gpui::RenderImage`，播放页用 `img(arc).size_full().object_fit(Contain)` 铺满播放区；
- `rgb0` 的第 4 字节是 0，GPUI 按 BGRA 解释会当成全透明，逐像素补 `alpha=255`（零通道转换）；
- 后台 30fps 轮询，`cx.notify()` 驱动重绘；播放区尺寸变化时 `set_target_size()` 同步给 mpv。

**2. 只用 mpv，硬解/软解可切换**

不引入 libvlc。设置页「MPV 解码方式」在 `hwdec=auto-copy`（硬解，帧回拷内存）与 `hwdec=no`（软解）之间切换；因为 mpv 的 `hwdec` 变更只对下一个文件生效，切换时会记录进度并重载当前媒体。另有「画质预设」三档（滤镜/缩放参数）。

**3. 界面按 Nexus 截图重做**

| 页面 | 内容 |
|---|---|
| 视频库 | 左侧 200px 路径栏、搜索、缩略图网格（格式角标 / hover 播放圆）、右下角网络流 FAB |
| IPTV | 源卡片（远程/文件/自动更新角标）、导入对话框、刷新/编辑/删除 |
| 音乐库 | 歌曲/专辑/歌手分类下钻、排序、内嵌封面、LRC 歌词 |
| 设置 | 主题色 6 预设、播放后端（固定 MPV）、解码方式、画质、倍速、音量、关于 |
| 视频播放页 | 画面区 + 右侧 260px 播放列表、进度/音量/倍速 |
| IPTV 播放页 | 7:3 分屏 + 频道面板、LIVE 角标、无进度条 |
| 音乐播放页 | 封面、滚动歌词、队列 |
| 全局 | 64px 左侧导航、64px 底部常驻迷你播放栏 |

**4. 修复播放时内存涨到数 GB**

根因在 GPUI 侧：`img()` 以 `(RenderImage.id, frame_index)` 为 key，把每帧**永久**存进 `sprite_atlas` 并上传一张 GPU 纹理；而 `RenderImage::new()` 的 id 是全局递增、永不复用的。原实现每帧新建 `Arc<RenderImage>` 且从不调用 `window.drop_image()` → 30fps 下每秒新增几十 MB 纹理，播放几分钟就涨到几个 G。

现在 `AppView::render` 只保留最近两帧，每帧把**前前一帧**用 `window.drop_image()` 从 atlas 移除（延迟一帧，保证 GPU 已绘制完），atlas 恒定 2 张纹理，内存不再随时间增长；同时删掉 `player.rs` 里每帧一次无人使用的整帧像素拷贝。缩略图/封面走 `ImageCache` 复用同一个 `Arc`（id 不变），不会重复插入 atlas。

## 依赖说明

默认用 **crates.io 的 `gpui = "=0.2.2"`**：这份代码已在该版本上完整通过 `cargo check` 与 `cargo build`，构建无需拉取巨大的 zed 仓库。

如需切回 nyaterm 使用的 zed fork，改 3 处（`Cargo.toml` 两行 + `main.rs` 两行）：

```toml
# Cargo.toml：替换 gpui 一行，并加回 gpui_platform
gpui = { version = "=0.2.2", git = "https://github.com/nyakang/zed", rev = "a45a77c08a7cd5ed364778eb84961e1bde650b73" }
gpui_platform = { git = "https://github.com/nyakang/zed", rev = "a45a77c08a7cd5ed364778eb84961e1bde650b73" }
```

```rust
// src/main.rs
use gpui_platform::application;                       // 加回导入
gpui::Application::new().run(|cx: &mut App| { ... }) // 改回 application().run(...)
```

> fork 只是修 bug，基础 API 与 crates.io 版一致（页面代码是按 nyaterm 验证过的方法白名单写的）；但仓库很大，首次 `cargo fetch` 可能耗时很久。

`vendor/xattr` 是 `xattr 0.2.3` 的**一行补丁**：Linux 上新版 libc 没有 `ENOATTR` 常量，而 `gpui → gpui_http_client → zed-async-tar → xattr` 会因此编译失败，补丁改成 `ENODATA`。`Cargo.toml` 里的 `[patch.crates-io]` 指向它；Windows 构建不走这段常量，保留补丁无副作用。

## 构建

**Windows**（主要目标）

1. Rust stable；
2. 下载 [shinchiro/mpv-winbuild-cmake](https://github.com/shinchiro/mpv-winbuild-cmake) 的 `mpv-dev-x86_64-*.7z`，把 `libmpv-2.dll` 与 `.lib` 放到链接路径；运行时 `libmpv-2.dll` 与 exe 同目录；
3. `cargo build --release`。

`.github/workflows/ci.yml` 已包含自动下载 mpv-dev 并构建的脚本。

**Linux**

```bash
sudo apt install libmpv-dev pkg-config \
  libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev libx11-dev libxcb1-dev \
  libfontconfig1-dev libfreetype6-dev
cargo build --release
```

> 少了 `libxkbcommon-x11-dev` 会报 `unable to find library -lxkbcommon-x11`。

## 操作

- 左侧导航切页；视频库/音乐库先点「+」添加本地文件夹；IPTV 点「导入」加 m3u 源或远程链接。
- 播放页快捷键：`Space` 暂停、`←/→` ±10s、`↑/↓` 音量、`F` 全屏、`Esc` 返回。
- **中文输入**：GPUI 0.2.2 没有成熟 IME 输入框，输入框右侧有「贴」按钮 —— 先复制文本，再点「贴」即可（搜索框、网络流 URL、IPTV 链接都用这个）。
- 设置保存在 `%LOCALAPPDATA%\mpv-gpui-player\settings.json`（Linux 为 `~/.local/share/mpv-gpui-player/settings.json`）。

## 已知限制

- 画面走 SW 取帧，硬解用 `auto-copy` 把帧回拷内存；4K 高码率时 CPU/内存带宽占用偏高。
- 图标用 Unicode 字形代替矢量图标，与 Nexus 的 Material Icons 不完全一致。
- 缩略图/封面只做进程内内存缓存，不落盘。
- 无字幕/音轨/画面比例切换（与 Nexus 一致，有意不做）。
