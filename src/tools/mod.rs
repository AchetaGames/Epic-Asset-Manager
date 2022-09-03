pub mod asset_info;
pub mod epic_web;
pub mod or;

pub async fn open_directory(path: &str) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(dir) = std::fs::File::open(path) {
            match ashpd::desktop::open_uri::open_directory(
                &ashpd::WindowIdentifier::default(),
                &dir,
            )
            .await
            {
                Err(e) => {
                    error!("Unable to open directory using portals: {}", e);
                }
                Ok(_) => {
                    return;
                }
            };
        }
    }
    let p = std::path::PathBuf::from(path);
    if p.is_dir() {
        if let Some(dir) = p.to_str() {
            if let Err(e) = opener::open(dir) {
                error!("Unable to open directory: {}", e);
            };
        }
    } else {
        if let Some(directory) = p.parent() {
            if let Some(parent) = directory.to_str() {
                if let Err(e) = opener::open(parent) {
                    error!("Unable to open directory: {}", e);
                };
            }
        }
    }
}
