use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .with_context(|| format!("无法创建目录 {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path).with_context(|| format!("无法创建目录 {}", path.display()))?;
    }
    Ok(())
}

pub fn restrict_file(path: &Path) -> Result<()> {
    set_mode(path, 0o600)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("无法设置文件权限 {}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}
