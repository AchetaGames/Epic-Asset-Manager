use crate::ui::widgets::download_manager::DownloadMsg::{DockerBlobFailed, DockerCanceled};
use crate::ui::widgets::download_manager::{
    download_item, DownloadMsg, DownloadStatus, ThreadMessages,
};
use glib::clone;
use gtk4::glib;
use gtk4::glib::Sender;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use regex::Regex;
use std::path::PathBuf;
use std::thread;

pub(crate) trait Docker {
    fn perform_docker_blob_downloads(
        &self,
        _version: &str,
        _size: u64,
        _digests: Vec<(String, u64)>,
    ) {
        unimplemented!()
    }

    fn download_docker_digest(&self, _version: &str, _digest: (String, u64)) {
        unimplemented!()
    }

    fn cancel_docker_digest(&self, _version: &str, _digest: (String, u64)) {
        unimplemented!()
    }

    fn pause_docker_digest(&self, _version: String, _digest: (String, u64)) {
        unimplemented!()
    }

    fn download_engine_from_docker(&self, _version: &str) {
        unimplemented!()
    }

    fn docker_target_directory(&self) -> Option<PathBuf> {
        unimplemented!()
    }

    fn docker_download_progress(&self, _version: &str, _progress: u64) {
        unimplemented!()
    }

    fn docker_blob_finished(&self, _version: &str, _digest: &str) {
        unimplemented!()
    }

    fn docker_extract_digests(&self, _version: &str) {
        unimplemented!()
    }

    fn docker_extraction_finished(&self, _version: &str) {
        unimplemented!()
    }

    fn docker_finished(&self, _item: &download_item::EpicDownloadItem) {
        unimplemented!()
    }

    fn cancel_docker_download(&self, _version: String) {
        unimplemented!()
    }

    fn pause_docker_download(&self, _version: String) {
        unimplemented!()
    }
    fn resume_docker_download(&self, _version: String) {
        unimplemented!()
    }
}

impl Docker for crate::ui::widgets::download_manager::EpicDownloadManager {
    #[cfg(target_os = "linux")]
    fn perform_docker_blob_downloads(&self, version: &str, size: u64, digests: Vec<(String, u64)>) {
        let self_ = self.imp();
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.set_property("status", "waiting for download slot".to_string());
        item.set_total_size(size as u128);
        item.set_total_files(digests.len() as u64);

        let v = version.to_string();

        let mut d = self_.docker_digests.borrow_mut();
        if d.get_mut(version).is_none() {
            let mut vec: Vec<(String, crate::ui::widgets::download_manager::DownloadStatus)> =
                Vec::new();
            for digest in digests {
                vec.push((
                    digest.0.clone(),
                    crate::ui::widgets::download_manager::DownloadStatus::Init,
                ));
                self.download_docker_digest(&v, digest);
            }
            d.insert(v, vec);
        }
    }

    #[cfg(target_os = "linux")]
    fn download_docker_digest(&self, version: &str, digest: (String, u64)) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_: &crate::window::imp::EpicAssetManagerWindow =
                crate::window::imp::EpicAssetManagerWindow::from_instance(window);
            if let Some(dclient) = &*win_.model.borrow().dclient.borrow() {
                let ver = version.to_string();
                let d = digest.0.clone();
                let size = digest.1;
                let client = dclient.clone();
                let sender = self_.sender.clone();
                let pool = self_.download_pool.clone();
                let target = match self.docker_target_directory() {
                    None => return,
                    Some(t) => t,
                };
                debug!("Going to download to {:?}", target);
                let (send, recv) = std::sync::mpsc::channel::<super::ThreadMessages>();
                self.add_thread_sender(ver.clone(), send);
                thread::spawn(move || {
                    let (tx, rx): (std::sync::mpsc::Sender<u64>, std::sync::mpsc::Receiver<u64>) =
                        std::sync::mpsc::channel();
                    let v = ver.clone();
                    let s = sender.clone();
                    pool.execute(move || {
                        match client.get_blob_with_progress_file(
                            "epicgames/unreal-engine",
                            &d,
                            Some(size),
                            Some(tx),
                            target.as_path(),
                        ) {
                            Ok(_) => s.send(crate::ui::widgets::download_manager::DownloadMsg::DockerBlobFinished(v, d)).unwrap(),
                            Err(e) => {

                                match &e {
                                    ghregistry::errors::Error::IO(err) => {
                                        error!("Failed blob download because: {:?}", err);
                                        s.send(crate::ui::widgets::download_manager::DownloadMsg::IOError(err.to_string())).unwrap();
                                    },
                                    ghregistry::errors::Error::Sender(_e) => {}
                                    _ => {
                                        error!("Failed blob download because: {:?}", e);
                                        s.send(crate::ui::widgets::download_manager::DownloadMsg::DockerBlobFailed(v, (d, size))).unwrap()
                                    }
                                }
                            }
                        };
                    });
                    while let Ok(progress) = rx.recv() {
                        if let Ok(m) = recv.try_recv() {
                            process_docker_thread_message(ver.clone(), digest.clone(), &sender, m);
                            return;
                        }
                        sender.send(crate::ui::widgets::download_manager::DownloadMsg::DockerDownloadProgress(
                            ver.clone(),
                            progress,
                        )).unwrap();
                    }
                });
            }
        }
    }
    #[cfg(target_os = "linux")]
    fn cancel_docker_digest(&self, _version: &str, digest: (String, u64)) {
        let mut target = match self.docker_target_directory() {
            None => return,
            Some(t) => t,
        };
        target.push(digest.0);
        if let Err(e) = std::fs::remove_file(target) {
            warn!("Unable to remove docker file {:?}", e);
        };
    }

    #[cfg(target_os = "linux")]
    fn pause_docker_digest(&self, version: String, digest: (String, u64)) {
        let self_ = self.imp();
        self_
            .paused_docker_digests
            .borrow_mut()
            .entry(version)
            .or_insert(vec![])
            .push(digest);
    }

    #[cfg(target_os = "linux")]
    fn download_engine_from_docker(&self, version: &str) {
        debug!("Initializing docker engine download of {}", version);
        let self_ = self.imp();

        let re = Regex::new(r"dev-(?:slim-)?(\d\.\d+.\d+)").unwrap();
        let mut items = self_.download_items.borrow_mut();
        let item = match items.get_mut(version) {
            None => {
                let item =
                    crate::ui::widgets::download_manager::download_item::EpicDownloadItem::new();
                debug!("Adding item to the list under: {}", version);
                items.insert(version.to_string(), item.clone());
                item
            }
            Some(_) => {
                return;
            }
        };
        if let Some(w) = self_.window.get() {
            item.set_window(w);
        }
        item.set_download_manager(self);
        item.set_property("version", version);
        for cap in re.captures_iter(version) {
            item.set_property("label", cap[1].to_string());
        }
        item.set_property("status", "initializing...".to_string());

        item.connect_local(
            "finished",
            false,
            clone!(@weak self as edm, @weak item => @default-return None, move |_| {
                edm.docker_finished(&item);
                None
            }),
        );

        match gtk4::gdk_pixbuf::Pixbuf::from_resource(
            "/io/github/achetagames/epic_asset_manager/icons/scalable/emblems/ue-logo-symbolic.svg",
        ) {
            Ok(pix) => {
                item.set_property("thumbnail", &pix);
            }
            Err(e) => {
                error!("Unable to load icon: {}", e);
            }
        };
        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some());

        if let Some(window) = self_.window.get() {
            let win_: &crate::window::imp::EpicAssetManagerWindow =
                crate::window::imp::EpicAssetManagerWindow::from_instance(window);
            if let Some(dclient) = &*win_.model.borrow().dclient.borrow() {
                let client = dclient.clone();
                let sender = self_.sender.clone();
                let v = version.to_string();
                self_.download_pool.execute(move || {
                    match client.get_manifest("epicgames/unreal-engine", &v) { 
                        Ok(manifest) => match manifest.layers_digests(None) {
                            Ok(digests) => {
                                sender
                                    .send(crate::ui::widgets::download_manager::DownloadMsg::PerformDockerEngineDownload(
                                        v,
                                        manifest.download_size().unwrap_or(0),
                                        digests,
                                    ))
                                    .unwrap();
                            }
                            Err(e) => {
                                error!("Unable to get manifest layers: {:?}", e);
                            }
                        },
                        Err(e) => {
                            error!("Unable to get docker manifest {:?}", e);
                        }
                    };
                });
            }
        }
    }

    fn docker_target_directory(&self) -> Option<PathBuf> {
        let self_ = self.imp();

        let mut target = match self_.settings.strv("unreal-engine-directories").get(0) {
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
            Some(p) => PathBuf::from(p),
        };
        target.push("docker");
        Some(target)
    }

    #[cfg(target_os = "linux")]
    fn docker_download_progress(&self, version: &str, progress: u64) {
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.add_downloaded_size(progress as u128);

        self.emit_by_name::<()>("tick", &[]);
    }

    #[cfg(target_os = "linux")]
    fn docker_blob_finished(&self, version: &str, digest: &str) {
        let self_ = self.imp();
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            for d in digests {
                if d.0.eq(digest) {
                    d.1 = crate::ui::widgets::download_manager::DownloadStatus::Downloaded;
                }
            }
        }
        self.docker_extract_digests(version);
    }

    #[cfg(target_os = "linux")]
    fn docker_extract_digests(&self, version: &str) {
        let self_ = self.imp();
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            let mut to_extract: Vec<String> = Vec::new();
            let target = match self.docker_target_directory() {
                None => return,
                Some(t) => t,
            };
            for d in digests {
                match d.1 {
                    DownloadStatus::Init => {
                        break;
                    }
                    DownloadStatus::Downloaded => {
                        let mut p = target.clone();
                        p.push(&d.0);
                        to_extract.push(p.to_str().unwrap_or_default().to_string());
                        d.1 = DownloadStatus::Extracting;
                    }
                    DownloadStatus::Extracted => {
                        continue;
                    }
                    DownloadStatus::Extracting => {
                        return;
                    }
                };
            }
            if !to_extract.is_empty() {
                let path = match self_.settings.strv("unreal-engine-directories").get(0) {
                    None => PathBuf::from(&version),
                    Some(p) => {
                        let mut path = PathBuf::from(p);
                        path.push(&version);
                        path
                    }
                };
                if let Err(e) = std::fs::create_dir_all(&path) {
                    warn!("Unable to create target directory: {}", e);
                };
                let can_path = path.canonicalize().unwrap();
                let sender = self_.sender.clone();
                let v = version.to_string();
                #[cfg(target_os = "linux")]
                {
                    self_.file_pool.execute(move || {
                        match ghregistry::render::unpack_partial_files(
                            to_extract.clone(),
                            &can_path,
                            "home/ue4/UnrealEngine/",
                        ) {
                            Ok(_) => {
                                sender.send(
                                    crate::ui::widgets::download_manager::DownloadMsg::DockerExtractionFinished(
                                        v,
                                    ),
                                ).unwrap();
                            }
                            Err(e) => {
                                error!("Error during render of {:?}: {:?}", to_extract, e);
                            }
                        };
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn docker_extraction_finished(&self, version: &str) {
        let self_ = self.imp();
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            let item = match self.get_item(version) {
                None => {
                    return;
                }
                Some(i) => i,
            };
            let target = match self.docker_target_directory() {
                None => return,
                Some(t) => t,
            };
            let mut remaining = 0;
            for d in digests {
                match d.1 {
                    DownloadStatus::Init | DownloadStatus::Downloaded => {
                        remaining += 1;
                    }
                    DownloadStatus::Extracted => {}
                    DownloadStatus::Extracting => {
                        d.1 = DownloadStatus::Extracted;
                        let mut t = target.clone();
                        t.push(&d.0);
                        std::fs::remove_file(t).expect("Unable to remove digest file");
                        item.file_processed();
                    }
                };
            }
            if remaining == 0 {
                if let Some(window) = self_.window.get() {
                    let win_: &crate::window::imp::EpicAssetManagerWindow = window.imp();
                    let l_ = win_.logged_in_stack.imp();
                    l_.engines.load_engines();
                }
            }
        }
        self.docker_extract_digests(version);
    }

    fn docker_finished(&self, item: &download_item::EpicDownloadItem) {
        self.finish(item);
    }

    #[cfg(target_os = "linux")]
    fn cancel_docker_download(&self, version: String) {
        let self_ = self.imp();
        if let Some(item) = self.get_item(&version) {
            self.send_to_thread_sender(version.clone(), ThreadMessages::Cancel);
            item.set_property("status", "Canceled".to_string());
            item.set_property("speed", "".to_string());
            if item.paused() {
                if let Some(values) = self_.paused_docker_digests.borrow_mut().remove(&version) {
                    for digest in values {
                        self_
                            .sender
                            .send(DockerCanceled(version.clone(), digest))
                            .unwrap();
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn pause_docker_download(&self, version: String) {
        if let Some(item) = self.get_item(&version) {
            self.send_to_thread_sender(version, ThreadMessages::Pause);
            item.set_property("status", "Paused".to_string());
            item.set_property("speed", "".to_string());
        }
    }

    #[cfg(target_os = "linux")]
    fn resume_docker_download(&self, version: String) {
        let self_ = self.imp();
        if let Some(values) = self_.paused_docker_digests.borrow_mut().remove(&version) {
            for digest in values {
                self_
                    .sender
                    .send(DockerBlobFailed(version.clone(), digest))
                    .unwrap();
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn process_docker_thread_message(
    version: String,
    digest: (String, u64),
    sender: &Sender<crate::ui::widgets::download_manager::DownloadMsg>,
    m: crate::ui::widgets::download_manager::ThreadMessages,
) {
    match m {
        ThreadMessages::Cancel => {
            sender
                .send(DownloadMsg::DockerCanceled(version, digest))
                .unwrap();
        }
        ThreadMessages::Pause => {
            sender
                .send(DownloadMsg::DockerPaused(version, digest))
                .unwrap();
        }
    }
}
