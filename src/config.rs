use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// 应用配置目前仍保留 ping_* 字段名，主要为了兼容已生成的 config.toml。
// 实际含义已经变成“网络探针间隔”，不再执行系统 ping 命令。
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub ping_interval_secs: u64,
    pub ping_host: String,
    pub web_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ping_interval_secs: 3,
            ping_host: "223.5.5.5".to_string(),
            web_port: 18888,
        }
    }
}

impl AppConfig {
    // 手写一个极小 TOML 子集，避免为了三项配置引入额外依赖。
    pub fn to_toml_string(&self) -> String {
        format!(
            "ping_interval_secs = {}\nping_host = \"{}\"\nweb_port = {}\n",
            self.ping_interval_secs, self.ping_host, self.web_port
        )
    }
}

pub fn ensure_files(config_path: &Path, curl_path: &Path) -> io::Result<()> {
    // 首次启动时创建配置和 cURL 文件；之后不覆盖用户已有内容。
    if !config_path.exists() {
        fs::write(config_path, AppConfig::default().to_toml_string())?;
    }
    if !curl_path.exists() {
        fs::write(curl_path, "")?;
    }
    Ok(())
}

pub fn load_config(path: &Path) -> AppConfig {
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return AppConfig::default(),
    };

    // 解析失败的字段直接落回默认值，避免配置文件局部损坏导致程序无法启动。
    let mut cfg = AppConfig::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let val = parts.next().unwrap_or("").trim();
        match key {
            "ping_interval_secs" => {
                if let Ok(v) = val.parse::<u64>() {
                    if (1..=3600).contains(&v) {
                        cfg.ping_interval_secs = v;
                    }
                }
            }
            "ping_host" => {
                let stripped = val.trim_matches('"').trim_matches('\'');
                if !stripped.is_empty() {
                    cfg.ping_host = stripped.to_string();
                }
            }
            "web_port" => {
                if let Ok(v) = val.parse::<u16>() {
                    if v > 0 {
                        cfg.web_port = v;
                    }
                }
            }
            _ => {}
        }
    }

    cfg
}

pub fn save_config(path: &Path, config: &AppConfig) -> io::Result<()> {
    fs::write(path, config.to_toml_string())
}

pub fn read_curl(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn write_curl(path: &Path, content: &str) -> io::Result<()> {
    fs::write(path, content)
}

pub fn ensure_parent_dir(path: &PathBuf) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
