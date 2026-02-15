use crate::ui::widgets::download_manager::{download_item, Msg, ThreadMessages};
use crate::ui::widgets::logged_in::engines::epic_download::Blob;
use crate::ui::widgets::logged_in::refresh::Refresh;
use glib::clone;
use gtk4::glib;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::{self, prelude::*};
use log::{debug, error, info, warn};
use regex::Regex;
use reqwest::Url;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use zip::ZipArchive;

pub trait EpicFile {
    fn perform_file_download(&self, _url: &str, _size: u64, _version: &str) {
        unimplemented!()
    }

    fn download_engine_from_epic(&self, _version: &str) {
        unimplemented!()
    }

    fn epic_finished(&self, _item: &download_item::EpicDownloadItem) {
        unimplemented!()
    }

    fn start_version_file_download(&self, _version: &str) {
        unimplemented!()
    }

    fn engine_target_directory(&self) -> Option<PathBuf> {
        unimplemented!()
    }

    fn epic_download_progress(&self, _version: &str, _progress: u64) {
        unimplemented!()
    }

    fn epic_file_extraction_progress(&self, _version: &str, _data: u64) {
        unimplemented!()
    }

    fn cancel_epic_download(&self, _version: String) {
        unimplemented!()
    }

    fn pause_epic_download(&self, _version: String) {
        unimplemented!()
    }

    fn resume_epic_download(&self, _version: String) {
        unimplemented!()
    }

    fn epic_file_finished(&self, _version: &str) {
        unimplemented!()
    }

    fn epic_file_extracted(&self, _version: &str) {
        unimplemented!()
    }
}

impl EpicFile for crate::ui::widgets::download_manager::EpicDownloadManager {
    fn perform_file_download(&self, url: &str, size: u64, version: &str) {
        let self_ = self.imp();
        let Some(item) = self.get_item(version) else {
            return;
        };
        item.set_property("status", "waiting for download slot".to_string());
        item.set_total_size(u128::from(size));
        item.set_total_files(1);
        let (send, recv) = std::sync::mpsc::channel::<ThreadMessages>();
        self.add_thread_sender(version.to_string(), send);
        let sender = self_.sender.clone();
        let link = Url::parse(url).expect("Valid URL");
        let ver = version.to_string();
        let mut p = self
            .engine_target_directory()
            .expect("Invalid Target directory");
        p.push("epic");
        p.push(version);
        self_.download_pool.execute(move || {
            run(size, &recv, &sender, &link, ver, &mut p);
        });
    }

    fn engine_target_directory(&self) -> Option<PathBuf> {
        let self_ = self.imp();

        let target = match self_.settings.strv("unreal-engine-directories").first() {
            None => {
                if let Some(w) = self_.window.get() {
                    w.add_notification(
                        "missing engine config",
                        "Unable to download engine missing Unreal Engine Directories configuration",
                        gtk4::MessageType::Error,
                    );
                }
                return None;
            }
            Some(p) => PathBuf::from(p.as_str()),
        };
        Some(target)
    }

    fn download_engine_from_epic(&self, version: &str) {
        debug!("Initializing epic engine download of {}", version);
        let self_ = self.imp();
        let re = Regex::new(r"Linux_Unreal_Engine_(\d\.\d+.\d+)_?(preview-\d+)?").unwrap();
        let item = {
            let mut state = self_.state.borrow_mut();
            match state.download_items.get_mut(version) {
                None => {
                    let item = download_item::EpicDownloadItem::new();
                    debug!("Adding item to the list under: {}", version);
                    state
                        .download_items
                        .insert(version.to_string(), item.clone());
                    item
                }
                Some(_) => {
                    return;
                }
            }
        };
        if let Some(w) = self_.window.get() {
            item.set_window(w);
        }
        item.set_download_manager(self);
        item.set_property("version", version);
        item.set_property("item-type", download_item::ItemType::Epic);
        for cap in re.captures_iter(version) {
            item.set_property(
                "label",
                cap.get(2).map_or_else(
                    || cap[1].to_string(),
                    |suffix| {
                        format!(
                            "{} ({})",
                            cap.get(1).map_or("", |v| v.as_str()),
                            suffix.as_str()
                        )
                    },
                ),
            );
        }
        item.set_property("status", "initializing...".to_string());

        item.connect_local(
            "finished",
            false,
            clone!(
                #[weak(rename_to=edm)]
                self,
                #[weak]
                item,
                #[upgrade_or]
                None,
                move |_| {
                    edm.epic_finished(&item);
                    None
                }
            ),
        );

        item.set_property("thumbnail", Some(gtk4::gdk::Texture::from_resource(
            "/io/github/achetagames/epic_asset_manager/icons/scalable/emblems/ue-logo-symbolic.svg",
        )));

        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some());
        self.start_version_file_download(version);
    }

    fn start_version_file_download(&self, version: &str) {
        let self_ = self.imp();
        let (sender, receiver) = async_channel::unbounded();

        let vers = version.to_string();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=dm)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(response) = receiver.recv().await {
                    let self_ = dm.imp();
                    let s = self_.sender.clone();
                    if let Some(ver) = filter_versions(response, &vers) {
                        let _ =
                            s.send_blocking(Msg::EpicDownloadStart(ver.name, ver.url, ver.size));
                    }
                }
            }
        ));
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let logged_in = win_.logged_in_stack.imp();
            let engines_box = logged_in.engines.imp();
            let engines_side = engines_box.side.imp();
            let engine_install = engines_side.install.imp();
            let epic_download = engine_install.epic.clone();
            epic_download.get_versions(sender);
        }
    }

    fn epic_finished(&self, item: &download_item::EpicDownloadItem) {
        let self_ = self.imp();
        if let Some(version) = item.version() {
            let mut p = self
                .engine_target_directory()
                .expect("Invalid Target directory");
            p.push("epic");
            p.push(version);
            if let Err(e) = fs::remove_file(&p) {
                error!("Unable to remove downloaded file: {}", e);
            };
            if let Some(parent) = p.parent() {
                if let Err(e) = fs::remove_dir(parent) {
                    error!("Unable to remove epic download directory: {}", e);
                };
            }
            if let Some(window) = self_.window.get() {
                let win_ = window.imp();
                let l = win_.logged_in_stack.imp();
                let e = l.engines.clone();
                e.run_refresh();
            }
        }

        self.finish(item);
    }

    fn epic_download_progress(&self, version: &str, progress: u64) {
        let Some(item) = self.get_item(version) else {
            return;
        };
        item.add_downloaded_size(u128::from(progress));

        self.emit_by_name::<()>("tick", &[]);
    }

    fn cancel_epic_download(&self, version: String) {
        let self_ = self.imp();
        if let Some(item) = self.get_item(&version) {
            self.send_to_thread_sender(&version.clone(), &ThreadMessages::Cancel);
            item.set_property("status", "Canceled".to_string());
            item.set_property("speed", String::new());
            if let Some(v) = item.version() {
                self_.state.borrow_mut().download_items.remove(&v);
            }
        }

        let mut p = self
            .engine_target_directory()
            .expect("Invalid Target directory");
        p.push("epic");
        p.push(version);
        if let Err(e) = fs::remove_file(p) {
            warn!("Unable to remove file {:?}", e);
        };
    }

    fn pause_epic_download(&self, version: String) {
        if let Some(item) = self.get_item(&version) {
            self.send_to_thread_sender(&version, &ThreadMessages::Pause);
            item.set_property("status", "Paused".to_string());
            item.set_property("speed", String::new());
        }
    }

    fn resume_epic_download(&self, version: String) {
        self.start_version_file_download(&version);
    }

    fn epic_file_extracted(&self, version: &str) {
        if let Some(item) = self.get_item(version) {
            item.file_processed();
            self.emit_by_name::<()>("tick", &[]);
        }
    }

    fn epic_file_finished(&self, version: &str) {
        let self_ = self.imp();
        info!("Finished file download");
        let mut p = self
            .engine_target_directory()
            .expect("Invalid Target directory");
        p.push("epic");
        p.push(version);
        let re = Regex::new(r"Linux_Unreal_Engine_(\d\.\d+.\d+(?:_preview-\d+)?)").unwrap();

        let mut target = self
            .engine_target_directory()
            .expect("Invalid Target directory");
        if let Some(cap) = re.captures_iter(version).next() {
            target.push(&cap[1]);
        }
        if p.exists() {
            if let Some(item) = self.get_item(version) {
                let metadata = fs::metadata(p.as_path()).expect("unable to read metadata");
                item.add_downloaded_size(item.total_size() - item.downloaded_size());
                if u128::from(metadata.size()) == item.total_size() {
                    let file = match File::open(&p) {
                        Ok(file) => file,
                        Err(e) => {
                            error!("Unable to open downloaded file {:?}: {}", p, e);
                            return;
                        }
                    };
                    if target.exists() {
                        warn!("Target already exists.");
                    }
                    let archive = match ZipArchive::new(file) {
                        Ok(archive) => archive,
                        Err(e) => {
                            error!("Unable to read zip archive {:?}: {}", p, e);
                            return;
                        }
                    };
                    item.set_total_files(archive.len() as u64);
                    let sender = self_.sender.clone();
                    let ver = version.to_string();
                    let (send, recv) = std::sync::mpsc::channel::<ThreadMessages>();
                    self.add_thread_sender(version.to_string(), send);
                    self_.file_pool.execute(move || {
                        extract(&target, archive, &sender, ver, &recv);
                    });
                }
            }
        }
    }
    fn epic_file_extraction_progress(&self, version: &str, data: u64) {
        if let Some(item) = self.get_item(version) {
            item.add_extracted_size(u128::from(data));
        }
    }
}

fn extract(
    target: &std::path::Path,
    mut archive: ZipArchive<File>,
    sender: &async_channel::Sender<Msg>,
    ver: String,
    recv: &Receiver<ThreadMessages>,
) {
    for i in 0..archive.len() {
        if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let mut file_target = target.to_path_buf();
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(e) => {
                error!("Unable to read zip entry {}: {}", i, e);
                continue;
            }
        };
        let outpath = if let Some(path) = file.enclosed_name() {
            path.to_owned()
        } else {
            let _ = sender.send_blocking(Msg::EpicFileExtracted(ver.clone()));
            continue;
        };
        file_target.push(&outpath);
        if file_target.exists() {
            let metadata = fs::metadata(file_target.as_path()).expect("unable to read metadata");
            if metadata.size() == file.size() {
                let _ = sender.send_blocking(Msg::EpicFileExtracted(ver.clone()));
                continue;
            }
        }
        if (*file.name()).ends_with('/') {
            if let Err(e) = fs::create_dir_all(&file_target) {
                error!("Unable to create directory {:?}: {}", file_target, e);
                continue;
            }
        } else {
            if let Some(p) = file_target.parent() {
                if !p.exists() {
                    if let Err(e) = fs::create_dir_all(p) {
                        error!("Unable to create directory {:?}: {}", p, e);
                        continue;
                    }
                }
            }
            let mut outfile = match File::create(&file_target) {
                Ok(outfile) => outfile,
                Err(e) => {
                    error!("Unable to create output file {:?}: {}", file_target, e);
                    continue;
                }
            };

            let mut buffer: [u8; 1024] = [0; 1024];
            loop {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                match file.read(&mut buffer) {
                    Ok(size) => {
                        if let Ok(m) = recv.try_recv() {
                            process_epic_thread_message(ver, sender, &m);
                            return;
                        }
                        if size > 0 {
                            if let Err(e) = outfile.write_all(&buffer[0..size]) {
                                error!("Unable to write extracted file {:?}: {}", file_target, e);
                                return;
                            }
                            let _ = sender.send_blocking(Msg::EpicFileExtractionProgress(
                                ver.clone(),
                                size as u64,
                            ));
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Extraction error: {:?}", e);
                        break;
                    }
                }
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                if let Err(e) = fs::set_permissions(&file_target, fs::Permissions::from_mode(mode))
                {
                    error!(
                        "Unable to set permissions on {:?}, mode: {} : {}",
                        file_target, mode, e
                    );
                }
            }
        }
        let _ = sender.send_blocking(Msg::EpicFileExtracted(ver.clone()));
    }
    let _ = sender.send_blocking(Msg::EpicFileFinished(ver));
}

fn run(
    size: u64,
    recv: &Receiver<ThreadMessages>,
    sender: &async_channel::Sender<Msg>,
    link: &Url,
    ver: String,
    p: &mut PathBuf,
) {
    if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    };
    if let Ok(m) = recv.try_recv() {
        process_epic_thread_message(ver, sender, &m);
        return;
    }
    debug!(
        "Downloading engine {} from {} to {:?}",
        ver,
        link.to_string(),
        p
    );
    let Some(parent) = p.parent() else {
        error!("Download path has no parent: {:?}", p);
        return;
    };
    if let Err(e) = fs::create_dir_all(parent) {
        error!("Unable to create download directory {:?}: {}", parent, e);
        return;
    }
    let mut client = if p.exists() {
        let metadata = fs::metadata(p.as_path()).expect("unable to read metadata");
        if metadata.size() == size {
            debug!("Already downloaded {}", p.to_str().unwrap_or_default());
            let _ = sender.send_blocking(Msg::EpicDownloadProgress(ver.clone(), size));
            let _ = sender.send_blocking(Msg::EpicFileFinished(ver));
            return;
        };

        let c = reqwest::blocking::Client::new();
        debug!("Trying to resume download");
        match c
            .get(link.clone())
            .header(
                reqwest::header::RANGE,
                format!("bytes={}-{}", metadata.size(), size),
            )
            .send()
        {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to resume Engine download: {}", e);
                return;
            }
        }
    } else {
        match reqwest::blocking::get(link.clone()) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to start Engine download: {}", e);
                return;
            }
        }
    };
    let mut buffer: [u8; 1024] = [0; 1024];
    let mut file = match File::create(&p) {
        Ok(file) => file,
        Err(e) => {
            error!("Unable to create download file {:?}: {}", p, e);
            return;
        }
    };
    loop {
        if let Ok(m) = recv.try_recv() {
            process_epic_thread_message(ver, sender, &m);
            return;
        }
        if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        match client.read(&mut buffer) {
            Ok(size) => {
                if let Ok(m) = recv.try_recv() {
                    process_epic_thread_message(ver, sender, &m);
                    return;
                }
                if size > 0 {
                    if let Err(e) = file.write_all(&buffer[0..size]) {
                        error!("Unable to write download file {:?}: {}", p, e);
                        return;
                    }
                    let _ =
                        sender.send_blocking(Msg::EpicDownloadProgress(ver.clone(), size as u64));
                } else {
                    break;
                }
            }
            Err(e) => {
                error!("Download error: {:?}", e);
                break;
            }
        }
    }
    let _ = sender.send_blocking(Msg::EpicFileFinished(ver));
}

fn filter_versions(versions: Vec<Blob>, version: &str) -> Option<Blob> {
    versions.into_iter().find(|ver| ver.name.eq(version))
}

fn process_epic_thread_message(
    version: String,
    sender: &async_channel::Sender<Msg>,
    m: &ThreadMessages,
) {
    match m {
        ThreadMessages::Cancel => {
            let _ = sender.send_blocking(Msg::EpicCanceled(version));
        }
        ThreadMessages::Pause => {
            let _ = sender.send_blocking(Msg::EpicPaused(version));
        }
    }
}
