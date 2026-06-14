//! 偏好设置 JSON 持久化
//! 文件: ~/Library/Application Support/com.weifengchen.ever-living/preferences.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Ever-Living server 端口
    #[serde(default = "default_port")]
    pub port: u16,
    /// 开机自启
    #[serde(default)]
    pub autostart: bool,
    /// 全局快捷键
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    /// 是否启用通知
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    /// 界面语言: "zh-CN" | "en"
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_port() -> u16 {
    7070
}
fn default_shortcut() -> String {
    "CmdOrCtrl+Shift+O".into()
}
fn default_true() -> bool {
    true
}
fn default_locale() -> String {
    "zh-CN".into()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            port: default_port(),
            autostart: false,
            shortcut: default_shortcut(),
            notifications_enabled: true,
            locale: default_locale(),
        }
    }
}

fn prefs_path() -> Result<std::path::PathBuf, String> {
    let dir = {
        let home = std::env::var("HOME").unwrap_or_default();
        std::path::Path::new(&home).join("Library/Application Support/com.weifengchen.ever-living")
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    Ok(dir.join("preferences.json"))
}

pub fn load() -> Preferences {
    let path = match prefs_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[Ever-Living] {e}");
            return Preferences::default();
        }
    };
    if !path.exists() {
        return Preferences::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(e) => {
            eprintln!("[Ever-Living] 读取配置失败: {e}");
            Preferences::default()
        }
    }
}

pub fn save(prefs: &Preferences) -> Result<(), String> {
    let path = prefs_path()?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("写入配置失败: {e}"))
}
