//! IPTV sources + simple M3U parser (group / channel).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IptvSource {
    pub name: String,
    /// "network" | "local"
    pub type_: String,
    pub url: String,
    pub auto_update: bool,
}

#[derive(Debug, Clone)]
pub struct M3uChannel {
    pub name: String,
    pub group: String,
    pub urls: Vec<String>,
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IptvPlaylist {
    /// group name -> channels
    pub groups: BTreeMap<String, Vec<M3uChannel>>,
}

impl IptvPlaylist {
    pub fn group_names(&self) -> Vec<String> {
        self.groups.keys().cloned().collect()
    }

    pub fn channels_in(&self, group: &str) -> &[M3uChannel] {
        self.groups
            .get(group)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn total_channels(&self) -> usize {
        self.groups.values().map(|v| v.len()).sum()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SourcesFile {
    sources: Vec<IptvSource>,
}

pub struct IptvStore {
    path: PathBuf,
    pub sources: Vec<IptvSource>,
    /// Cached playlist for the "active" source index
    pub active: Option<usize>,
    pub playlist: IptvPlaylist,
    pub browse_group: String,
}

impl IptvStore {
    pub fn new() -> Result<Self> {
        let base = data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("xplayer");
        fs::create_dir_all(&base)?;
        let path = base.join("iptv_sources.json");
        let mut sources = Vec::new();
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(f) = serde_json::from_str::<SourcesFile>(&text) {
                sources = f.sources;
            }
        }
        Ok(Self {
            path,
            sources,
            active: None,
            playlist: IptvPlaylist::default(),
            browse_group: String::new(),
        })
    }

    fn save(&self) -> Result<()> {
        let f = SourcesFile {
            sources: self.sources.clone(),
        };
        fs::write(&self.path, serde_json::to_string_pretty(&f)?)?;
        Ok(())
    }

    /// 供 UI 编辑源后保存
    pub fn save_sources(&self) -> Result<()> {
        self.save()
    }

    pub fn add_source(&mut self, src: IptvSource) -> Result<()> {
        self.sources.push(src);
        self.save()
    }

    pub fn remove_source(&mut self, index: usize) -> Result<()> {
        if index < self.sources.len() {
            self.sources.remove(index);
            if self.active == Some(index) {
                self.active = None;
                self.playlist = IptvPlaylist::default();
            }
            self.save()?;
        }
        Ok(())
    }

    pub fn load_source(&mut self, index: usize) -> Result<()> {
        let src = self
            .sources
            .get(index)
            .context("source index out of range")?
            .clone();
        let text = if src.type_ == "local" {
            fs::read_to_string(&src.url).with_context(|| format!("read {}", src.url))?
        } else {
            fetch_url(&src.url)?
        };
        let pl = parse_m3u(&text);
        self.active = Some(index);
        self.playlist = pl;
        self.browse_group = self
            .playlist
            .group_names()
            .first()
            .cloned()
            .unwrap_or_default();
        Ok(())
    }
}

fn fetch_url(url: &str) -> Result<String> {
    // Minimal HTTP GET without extra crate: use std + ureq if we add it.
    // Prefer adding `ureq` for reliability.
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .with_context(|| format!("GET {url}"))?;
    let text = resp.into_string().context("read body")?;
    Ok(text)
}

/// Parse M3U / M3U8 playlist text into groups.
pub fn parse_m3u(text: &str) -> IptvPlaylist {
    let mut groups: BTreeMap<String, Vec<M3uChannel>> = BTreeMap::new();
    let mut pending_name = String::from("未知频道");
    let mut pending_group = String::from("未分组");
    let mut pending_logo: Option<String> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXTINF:") {
            pending_logo = attr(line, "tvg-logo");
            pending_group = attr(line, "group-title").unwrap_or_else(|| "未分组".into());
            // name after last comma
            pending_name = line
                .rsplit(',')
                .next()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "未知频道".into());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        // URL line
        let url = line.to_string();
        let entry = groups.entry(pending_group.clone()).or_default();
        // merge same name in group
        if let Some(ch) = entry.iter_mut().find(|c| c.name == pending_name) {
            if !ch.urls.contains(&url) {
                ch.urls.push(url);
            }
        } else {
            entry.push(M3uChannel {
                name: pending_name.clone(),
                group: pending_group.clone(),
                urls: vec![url],
                logo: pending_logo.clone(),
            });
        }
        pending_name = "未知频道".into();
        pending_group = "未分组".into();
        pending_logo = None;
    }

    IptvPlaylist { groups }
}

fn attr(extinf: &str, key: &str) -> Option<String> {
    // group-title="..."
    let pat = format!("{key}=\"");
    let start = extinf.find(&pat)? + pat.len();
    let rest = &extinf[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
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

#[allow(dead_code)]
pub fn load_m3u_file(path: &Path) -> Result<IptvPlaylist> {
    let text = fs::read_to_string(path)?;
    Ok(parse_m3u(&text))
}
