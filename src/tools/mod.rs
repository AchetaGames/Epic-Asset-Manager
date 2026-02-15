use log::error;

pub mod asset_info;
pub mod epic_web;
pub mod or;

/// Open a directory using the XDG portal (Flatpak-safe) with `opener` fallback.
///
/// ashpd/zbus objects (Proxy, SignalStream) call `tokio::spawn` in their Drop
/// impls. We must ensure every ashpd object is dropped **inside** the
/// `block_on` async context so the Tokio runtime is still alive during cleanup.
pub fn open_directory(path: &str) {
    let path = path.to_string();
    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            if let Ok(dir) = std::fs::File::open(&path) {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    // Run the portal call AND drop all ashpd/zbus objects inside the
                    // async block so their Drop impls execute within the Tokio runtime.
                    let result: Result<(), String> = rt.block_on(async {
                        let req = ashpd::desktop::open_uri::OpenDirectoryRequest::default()
                            .send(&dir)
                            .await;
                        match req {
                            Ok(_request) => {
                                // _request (ashpd::Request<()>) dropped here, inside the runtime
                                Ok(())
                            }
                            Err(e) => Err(format!("{e}")),
                        }
                    });
                    match result {
                        Ok(()) => return,
                        Err(e) => {
                            error!("Unable to open directory using portals: {}", e);
                        }
                    }
                }
            }
        }
        let p = std::path::PathBuf::from(&path);
        if p.is_dir() {
            if let Some(dir) = p.to_str() {
                if let Err(e) = opener::open(dir) {
                    error!("Unable to open directory: {}", e);
                };
            }
        } else if let Some(directory) = p.parent() {
            if let Some(parent) = directory.to_str() {
                if let Err(e) = opener::open(parent) {
                    error!("Unable to open directory: {}", e);
                };
            }
        }
    });
}

/// Open a file using the XDG portal (Flatpak-safe) with `opener` fallback.
///
/// See `open_directory` for explanation of why ashpd objects must be dropped
/// inside the `block_on` async context.
pub fn open_file(path: &str) {
    let path = path.to_string();
    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            if let Ok(file) = std::fs::File::open(&path) {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    let result: Result<(), String> = rt.block_on(async {
                        let req = ashpd::desktop::open_uri::OpenFileRequest::default()
                            .send_file(&file)
                            .await;
                        match req {
                            Ok(_request) => Ok(()),
                            Err(e) => Err(format!("{e}")),
                        }
                    });
                    match result {
                        Ok(()) => return,
                        Err(e) => {
                            error!("Unable to open file using portals: {}", e);
                        }
                    }
                }
            }
        }
        if let Err(e) = opener::open(&path) {
            error!("Unable to open file: {}", e);
        }
    });
}

pub fn is_sandboxed() -> bool {
    #[cfg(target_os = "linux")]
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        if let Ok(rt) = rt {
            return rt.block_on(ashpd::is_sandboxed());
        }
    }
    false
}
