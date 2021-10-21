use crate::ui::widgets::download_manager::DownloadStatus;
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

    fn download_engine_from_docker(&self, _version: &str) {
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
}

impl Docker for crate::ui::widgets::download_manager::EpicDownloadManager {
    fn perform_docker_blob_downloads(&self, version: &str, size: u64, digests: Vec<(String, u64)>) {
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.set_property("status", "waiting for download slot".to_string())
            .unwrap();
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

    fn download_docker_digest(&self, version: &str, digest: (String, u64)) {
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
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
                let mut target = std::path::PathBuf::from(
                    self_
                        .settings
                        .string("temporary-download-directory")
                        .to_string(),
                );
                target.push("docker");
                debug!("Going to download to {:?}", target);
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
                                error!("Failed blob download because: {:?}", e);
                                s.send(crate::ui::widgets::download_manager::DownloadMsg::DockerBlobFailed(v, (d, size))).unwrap();
                            }
                        };
                    });
                    while let Ok(progress) = rx.recv() {
                        sender
                                    .send(crate::ui::widgets::download_manager::DownloadMsg::DockerDownloadProgress(
                                        ver.clone(),
                                        progress,
                                    ))
                                    .unwrap();
                    }
                });
            }
        }
    }

    fn docker_download_progress(&self, version: &str, progress: u64) {
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.add_downloaded_size(progress as u128);

        self.emit_by_name("tick", &[]).unwrap();
    }

    fn docker_blob_finished(&self, version: &str, digest: &str) {
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            for d in digests {
                if d.0.eq(digest) {
                    d.1 = crate::ui::widgets::download_manager::DownloadStatus::Downloaded;
                }
            }
        }
        self.docker_extract_digests(version);
    }

    fn docker_extract_digests(&self, version: &str) {
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            let mut to_extract: Vec<String> = Vec::new();
            let mut target = std::path::PathBuf::from(
                self_
                    .settings
                    .string("temporary-download-directory")
                    .to_string(),
            );
            target.push("docker");
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
                std::fs::create_dir(&path).expect("Failed to created directory");
                let can_path = path.canonicalize().unwrap();
                let sender = self_.sender.clone();
                let v = version.to_string();
                self_.file_pool.execute(move || {
                    match ghregistry::render::unpack_partial_files(
                        to_extract,
                        &can_path,
                        "home/ue4/UnrealEngine/",
                    ) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error during render: {:?}", e);
                        }
                    };
                    sender.send(
                        crate::ui::widgets::download_manager::DownloadMsg::DockerExtractionFinished(
                            v,
                        ),
                    ).unwrap();
                });
            }
        }
    }

    fn docker_extraction_finished(&self, version: &str) {
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
        if let Some(digests) = self_.docker_digests.borrow_mut().get_mut(version) {
            let item = match self.get_item(version) {
                None => {
                    return;
                }
                Some(i) => i,
            };
            let mut target = std::path::PathBuf::from(
                self_
                    .settings
                    .string("temporary-download-directory")
                    .to_string(),
            );
            target.push("docker");
            for d in digests {
                match d.1 {
                    DownloadStatus::Init
                    | DownloadStatus::Downloaded
                    | DownloadStatus::Extracted => {
                        continue;
                    }
                    DownloadStatus::Extracting => {
                        d.1 = DownloadStatus::Extracted;
                        let mut t = target.clone();
                        t.push(&d.0);
                        std::fs::remove_file(t);
                        item.file_processed();
                    }
                };
            }
        }
        self.docker_extract_digests(version);
    }

    fn download_engine_from_docker(&self, version: &str) {
        debug!("Initializing docker engine download of {}", version);
        let self_: &crate::ui::widgets::download_manager::imp::EpicDownloadManager =
            crate::ui::widgets::download_manager::imp::EpicDownloadManager::from_instance(self);
        let item = crate::ui::widgets::download_manager::download_item::EpicDownloadItem::new();
        let re = Regex::new(r"dev-(?:slim-)?(\d\.\d+.\d+)").unwrap();
        let mut items = self_.download_items.borrow_mut();
        match items.get_mut(version) {
            None => {
                debug!("Adding item to the list under: {}", version);
                items.insert(version.to_string(), item.clone());
            }
            Some(_) => {
                return;
            }
        };
        for cap in re.captures_iter(version) {
            item.set_property("label", cap[1].to_string()).unwrap();
        }
        item.set_property("status", "initializing...".to_string())
            .unwrap();

        match gtk4::gdk_pixbuf::Pixbuf::from_resource(
            "/io/github/achetagames/epic_asset_manager/icons/ue-logo-symbolic.svg",
        ) {
            Ok(pix) => {
                item.set_property("thumbnail", &pix).unwrap();
            }
            Err(e) => {
                error!("Unable to load icon: {}", e);
            }
        };
        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some())
            .unwrap();

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
                                println!("{:#?}", digests);
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
}
