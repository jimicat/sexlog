use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::{ensure_private_dir, restrict_file};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8787;

#[derive(Debug)]
pub struct Settings {
    pub config_file: PathBuf,
    pub db_file: PathBuf,
    pub report_dir: PathBuf,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Default, Deserialize)]
struct FileSettings {
    database: Option<PathBuf>,
    report_dir: Option<PathBuf>,
    #[serde(default)]
    server: FileServerSettings,
}

#[derive(Debug, Default, Deserialize)]
struct FileServerSettings {
    host: Option<String>,
    port: Option<u16>,
}

impl Settings {
    pub fn load(db_override: Option<PathBuf>) -> Result<Self> {
        let base_dirs = BaseDirs::new().context("无法确定用户家目录")?;
        let base_dir = base_dirs.home_dir().join(".sexlog");

        let config_file = env::var_os("SEXLOG_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| base_dir.join("config.toml"));

        let file_settings = if config_file.exists() {
            let raw = fs::read_to_string(&config_file)
                .with_context(|| format!("无法读取配置文件 {}", config_file.display()))?;
            toml::from_str::<FileSettings>(&raw)
                .with_context(|| format!("配置文件格式有误 {}", config_file.display()))?
        } else {
            FileSettings::default()
        };

        let config_base = config_file.parent().unwrap_or_else(|| Path::new("."));
        let db_file = if let Some(path) = db_override {
            absolute(path)
        } else if let Some(path) = file_settings.database {
            resolve_path(path, config_base)
        } else {
            base_dir.join("sexlog.db")
        };

        let report_dir = file_settings
            .report_dir
            .map(|path| resolve_path(path, config_base))
            .unwrap_or_else(|| base_dir.join("report"));

        let host = file_settings
            .server
            .host
            .unwrap_or_else(|| DEFAULT_HOST.to_string())
            .trim()
            .to_string();
        if host.is_empty() {
            bail!("配置中的 server.host 不能为空");
        }
        let port = file_settings.server.port.unwrap_or(DEFAULT_PORT);
        if port == 0 {
            bail!("配置中的 server.port 不能为 0");
        }

        Ok(Self {
            config_file,
            db_file,
            report_dir,
            host,
            port,
        })
    }

    pub fn init_config(&self) -> Result<bool> {
        if self.config_file.exists() {
            return Ok(false);
        }
        if let Some(parent) = self.config_file.parent() {
            ensure_private_dir(parent)?;
        }
        fs::write(&self.config_file, CONFIG_TEMPLATE)
            .with_context(|| format!("无法写入配置文件 {}", self.config_file.display()))?;
        restrict_file(&self.config_file)?;
        Ok(true)
    }
}

fn resolve_path(path: PathBuf, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn absolute(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::path::absolute(&path).unwrap_or(path)
    }
}

pub const CONFIG_TEMPLATE: &str = r#"# sexlog 配置文件
#
# 未配置的项默认保存在用户家目录下的 ~/.sexlog 中：
#   ~/.sexlog/config.toml   配置文件
#   ~/.sexlog/sexlog.db     数据库
#   ~/.sexlog/report/       静态仪表盘
#
# 相对路径以本配置文件所在目录为基准。

# 数据库文件路径
# database = "sexlog.db"

# 静态仪表盘输出目录
# report_dir = "report"

[server]
host = "127.0.0.1"
port = 8787
"#;
