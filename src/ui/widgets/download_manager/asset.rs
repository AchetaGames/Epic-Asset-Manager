use crate::tools::asset_info::Search;
use crate::ui::widgets::download_manager::Msg::CancelChunk;
use crate::ui::widgets::download_manager::{Msg, PostDownloadAction, ThreadMessages};
use glib::clone;
use gtk4::glib;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use log::{debug, error, info, warn};
use rand::Rng;
use reqwest::Url;
use sha1::digest::core_api::CoreWrapper;
use sha1::{Digest, Sha1, Sha1Core};
use std::ffi::OsString;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::runtime::Builder;

#[derive(Default, Debug, Clone)]
pub struct DownloadedFile {
    pub asset: String,
    pub release: String,
    pub name: String,
    pub chunks: Vec<egs_api::api::types::download_manifest::FileChunkPart>,
    pub finished_chunks: Vec<egs_api::api::types::download_manifest::FileChunkPart>,
    hash: String,
}

pub trait Asset {
    /// Add an asset for download
    /// This is the first step in the process
    fn add_asset_download(
        &self,
        _release_id: String,
        _asset: egs_api::api::types::asset_info::AssetInfo,
        _target: &Option<String>,
        _actions: Option<Vec<PostDownloadAction>>,
    ) {
        unimplemented!()
    }

    fn download_asset_manifest(
        &self,
        _release_id: String,
        _asset: egs_api::api::types::asset_info::AssetInfo,
        _sender: async_channel::Sender<(
            String,
            Vec<egs_api::api::types::download_manifest::DownloadManifest>,
        )>,
    ) {
        unimplemented!()
    }

    /// Process the downoad manifest to start downloading files
    /// Second step in an asset download
    /// This also validates if the file was already downloaded and if it is and the hashes match does not download it again.
    fn start_download_asset(
        &self,
        _id: &str,
        _dm: &[egs_api::api::types::download_manifest::DownloadManifest],
    ) {
        unimplemented!()
    }

    /// Download individual files
    /// This is a third step in the asset download process
    /// Splits files into chunks
    fn download_asset_file(
        &self,
        _id: String,
        _release: String,
        _filename: String,
        _manifest: egs_api::api::types::download_manifest::FileManifestList,
    ) {
        unimplemented!()
    }

    fn redownload_chunk(&self, _link: &Url, _p: PathBuf, _g: &str) {
        unimplemented!()
    }

    /// Download Chunks
    fn download_chunk(&self, _link: Url, _p: PathBuf, _g: String) {
        unimplemented!()
    }

    fn remove_chunk(&self, _p: PathBuf, _g: String) {
        unimplemented!()
    }

    fn chunk_progress_report(&self, _guid: &str, _progress: u128, _finished: bool) {
        unimplemented!()
    }

    fn asset_finished(&self, _item: &super::download_item::EpicDownloadItem) {
        unimplemented!()
    }

    fn file_already_extracted(
        &self,
        _asset_id: String,
        _progress: u128,
        _fullname: String,
        _filename: String,
    ) {
        unimplemented!()
    }

    fn pause_asset_download(&self, _asset: String) {
        unimplemented!()
    }

    fn asset_cleanup(&self, _asset: String) {
        unimplemented!()
    }

    fn pause_asset_chunk(&self, _url: Url, _path: PathBuf, _guid: String) {
        unimplemented!()
    }

    fn resume_asset_download(&self, _asset: String) {
        unimplemented!()
    }

    fn cancel_asset_download(&self, _asset: String) {
        unimplemented!()
    }
}

trait AssetPriv {
    fn load_thumbnail(
        &self,
        _id: String,
        _thumbnail: Option<egs_api::api::types::asset_info::KeyImage>,
    ) {
        unimplemented!()
    }

    fn extract_file_from_chunks(
        &self,
        _finished_files: &mut Vec<String>,
        _file: &str,
        _f: &mut DownloadedFile,
    ) {
        unimplemented!()
    }
}

impl Asset for super::EpicDownloadManager {
    /// Add an asset for download
    /// This is the first step in the process
    fn add_asset_download(
        &self,
        release_id: String,
        asset: egs_api::api::types::asset_info::AssetInfo,
        target: &Option<String>,
        actions: Option<Vec<PostDownloadAction>>,
    ) {
        debug!("Adding download: {:?}", asset.title);

        let self_ = self.imp();

        let mut items = self_.download_items.borrow_mut();
        let item = match items.get_mut(&release_id) {
            None => {
                let item = super::download_item::EpicDownloadItem::new();
                debug!("Adding item to the list under: {}", release_id);
                items.insert(release_id.clone(), item.clone());
                item
            }
            Some(_) => {
                // Item is already downloading do nothing
                return;
            }
        };
        if let Some(w) = self_.window.get() {
            item.set_window(w);
        }
        item.set_download_manager(self);
        if let Some(actions) = actions {
            item.add_actions(&actions);
        };
        item.set_property(
            "item-type",
            crate::ui::widgets::download_manager::download_item::ItemType::Asset,
        );
        item.set_property("asset", asset.id.clone());
        item.set_property("release", release_id.clone());
        item.set_property("label", asset.title.clone());
        item.set_property("target", target.clone());
        item.set_property("status", "initializing...".to_string());
        self.load_thumbnail(release_id.clone(), asset.thumbnail());

        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some());

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
                    edm.finish(&item);
                    None
                }
            ),
        );

        let (sender, receiver) = async_channel::unbounded();

        glib::spawn_future_local(clone!(
            #[weak(rename_to=download_manager)]
            self,
            #[upgrade_or_panic]
            async move {
                let self_ = download_manager.imp();
                let sender = self_.sender.clone();
                while let Ok((id, manifest)) = receiver.recv().await {
                    sender
                        .send_blocking(super::Msg::StartAssetDownload(id, manifest))
                        .unwrap();
                }
            }
        ));

        self.download_asset_manifest(release_id, asset, sender);
    }
    fn download_asset_manifest(
        &self,
        release_id: String,
        asset: egs_api::api::types::asset_info::AssetInfo,
        sender: async_channel::Sender<(
            String,
            Vec<egs_api::api::types::download_manifest::DownloadManifest>,
        )>,
    ) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let id = release_id.clone();
            self_.download_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                }
                let start = std::time::Instant::now();
                if let Some(release_info) = asset.release_info(&release_id) {
                    if let Some(manifest) = Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(eg.asset_manifest(
                            None,
                            None,
                            Some(asset.namespace),
                            Some(asset.id),
                            Some(release_info.app_id.unwrap_or_default()),
                        ))
                    {
                        debug!("Got asset manifest: {:?}", manifest);
                        let d = Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(eg.asset_download_manifests(manifest));
                        debug!("Got asset download manifests for {}", id);
                        sender.send_blocking((id, d)).unwrap();
                        // TODO cache download manifest
                    };
                }
                debug!("Download Manifest requests took {:?}", start.elapsed());
            });
        }
    }
    /// Process the downoad manifest to start downloading files
    /// Second step in an asset download
    /// This also validates if the file was already downloaded and if it is and the hashes match does not download it again.
    fn start_download_asset(
        &self,
        id: &str,
        dm: &[egs_api::api::types::download_manifest::DownloadManifest],
    ) {
        let self_ = self.imp();
        let Some(item) = self.get_item(id) else {
            return;
        };
        if dm.is_empty() {
            item.set_property("status", "Failed to get download manifests".to_string());
            return;
        }
        let mut targets: Vec<(String, bool)> = Vec::new();
        let mut to_vault = true;
        {
            let actions = item.actions();

            for act in actions {
                match act {
                    PostDownloadAction::Copy(t, over) => {
                        targets.push((t.clone(), over));
                    }
                    PostDownloadAction::NoVault => {
                        to_vault = false;
                    }
                }
            }
        };

        let target = if to_vault || targets.is_empty() {
            let mut v = match self.unreal_vault_dir(id) {
                None => {
                    return;
                }
                Some(s) => PathBuf::from(s),
            };
            v.push(dm[0].app_name_string.clone());
            v.push("data");
            v
        } else {
            PathBuf::from_str(&targets.pop().unwrap().0).unwrap()
        };
        let t = target.clone();
        let manifest = dm[0].clone();
        // Create target directory in the vault and save manifests to it
        self_
            .download_pool
            .execute(move || save_asset_manifest(&t, &manifest));

        item.set_property("status", "waiting for download slot".to_string());
        item.set_total_size(dm[0].total_download_size());
        item.set_total_files(dm[0].file_manifest_list.len() as u64);
        item.set_property("path", target.as_path().display().to_string());

        // consolidate manifests
        for manifest in dm {
            for m in manifest.files().values() {
                for chunk in m.file_chunk_parts.clone() {
                    if let Some(url) = chunk.link {
                        let mut chunks = self_.chunk_urls.borrow_mut();
                        match chunks.get_mut(&chunk.guid) {
                            None => {
                                chunks.insert(chunk.guid, vec![url.clone()]);
                            }
                            Some(v) => {
                                v.push(url.clone());
                            }
                        }
                    }
                }
            }
        }

        item.set_property("status", "validating".to_string());
        for (filename, manifest) in dm[0].files() {
            info!("Starting download of {} file {}", id, filename);
            let r_id = id.to_string();
            let r_name = dm[0].app_name_string.clone();
            let f_name = filename.clone();
            let sender = self_.sender.clone();

            let m = manifest.clone();
            let full_path = target.clone().as_path().join(filename);

            self_.download_pool.execute(move || {
                initiate_file_download(&r_id, &r_name, &f_name, &sender, m, &full_path);
            });
        }
    }
    /// Download individual files
    /// This is a third step in the asset download process
    /// Splits files into chunks
    fn download_asset_file(
        &self,
        id: String,
        release: String,
        filename: String,
        manifest: egs_api::api::types::download_manifest::FileManifestList,
    ) {
        info!("Downloading file {} for {}", filename, id);
        let self_ = self.imp();
        let Some(_item) = self.get_item(&id) else {
            return;
        };
        let vaults = self_.settings.strv("unreal-vault-directories");
        let mut target = std::path::PathBuf::from(vaults.first().map_or_else(
            || {
                self_
                    .settings
                    .string("temporary-download-directory")
                    .to_string()
            },
            std::string::ToString::to_string,
        ));
        target.push(release.clone());
        target.push("temp");
        let full_filename = format!("{id}/{release}/{filename}");
        self_.downloaded_files.borrow_mut().insert(
            full_filename.clone(),
            DownloadedFile {
                asset: id.clone(),
                release,
                name: filename,
                chunks: manifest.file_chunk_parts.clone(),
                finished_chunks: vec![],
                hash: manifest.file_hash,
            },
        );
        let sender = self_.sender.clone();
        for chunk in manifest.file_chunk_parts {
            // perform chunk download make sure we do not download the same chunk twice
            self_
                .asset_guids
                .borrow_mut()
                .entry(id.clone())
                .or_default()
                .push(chunk.guid.clone());
            let mut chunks = self_.downloaded_chunks.borrow_mut();
            match chunks.get_mut(&chunk.guid) {
                None => {
                    chunks.insert(chunk.guid.clone(), vec![full_filename.clone()]);
                    info!("Inserting file into chunk init {}", full_filename.clone());
                    let mut p = target.clone();
                    let g = chunk.guid.clone();
                    p.push(format!("{g}.chunk"));
                    sender
                        .send_blocking(Msg::RedownloadChunk(Url::parse("unix:/").unwrap(), p, g))
                        .unwrap();
                }
                Some(files) => {
                    files.push(full_filename.clone());
                    info!("Inserting file into chunk {}", full_filename.clone());
                }
            }
            info!("Chunks length: {}", chunks.len());
        }
    }

    fn redownload_chunk(&self, link: &Url, p: PathBuf, g: &str) {
        let self_ = self.imp();
        let sender = self_.sender.clone();
        let mut chunks = self_.chunk_urls.borrow_mut();
        match chunks.get_mut(g) {
            None => {
                // Unable to get chunk urls
                sender
                    .send_blocking(Msg::PerformChunkDownload(link.clone(), p, g.to_string()))
                    .unwrap();
            }
            Some(v) => {
                v.retain(|x| !x.eq(link));
                if v.is_empty() {
                    // No other URL available, redownloading
                    //TODO: This has the potential to loop forever
                    sender
                        .send_blocking(Msg::PerformChunkDownload(
                            link.clone(),
                            p.clone(),
                            g.to_string(),
                        ))
                        .unwrap();
                };
                let mut rng = rand::thread_rng();
                let index = rng.gen_range(0..v.len());
                let new: Option<&Url> = v.get(index);
                match new {
                    None => {
                        // Unable to get random URL, retrying the same one
                        sender
                            .send_blocking(Msg::PerformChunkDownload(
                                link.clone(),
                                p,
                                g.to_string(),
                            ))
                            .unwrap();
                    }
                    Some(u) => {
                        // Using new url to redownload the chunk
                        sender
                            .send_blocking(Msg::PerformChunkDownload(u.clone(), p, g.to_string()))
                            .unwrap();
                    }
                }
            }
        }
    }

    /// Download Chunks
    fn download_chunk(&self, link: Url, p: PathBuf, g: String) {
        let self_ = self.imp();
        if !link.has_host() {
            return;
        }
        let (send, recv) = std::sync::mpsc::channel::<ThreadMessages>();
        self.add_thread_sender(g.clone(), send);
        let sender = self_.sender.clone();
        self_.download_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            };
            if let Ok(m) = recv.try_recv() {
                process_thread_message(&link, &p, &g, &sender, &m);
                return;
            }
            debug!(
                "Downloading chunk {} from {} to {:?}",
                g,
                link.to_string(),
                p
            );
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            let mut client = match reqwest::blocking::get(link.clone()) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to start chunk download, trying again later: {}", e);
                    sender
                        .send_blocking(Msg::RedownloadChunk(link.clone(), p.clone(), g.clone()))
                        .unwrap();
                    return;
                }
            };
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut downloaded: u128 = 0;
            let mut file = File::create(&p).unwrap();
            loop {
                if let Ok(m) = recv.try_recv() {
                    process_thread_message(&link, &p, &g, &sender, &m);
                    return;
                }
                match client.read(&mut buffer) {
                    Ok(size) => {
                        if let Ok(m) = recv.try_recv() {
                            process_thread_message(&link, &p, &g, &sender, &m);
                            return;
                        }
                        if size > 0 {
                            downloaded += size as u128;
                            std::io::Write::write_all(&mut file, &buffer[0..size]).unwrap();
                            sender
                                .send_blocking(Msg::ChunkDownloadProgress(
                                    g.clone(),
                                    size as u128,
                                    false,
                                ))
                                .unwrap();
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
            sender
                .send_blocking(Msg::ChunkDownloadProgress(g.clone(), downloaded, true))
                .unwrap();
        });
    }

    fn remove_chunk(&self, path: PathBuf, _g: String) {
        if let Err(e) = std::fs::remove_file(&path) {
            warn!("Unable to remove chunk {:?}", e);
        };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::remove_dir(parent) {
                info!("Unable to remove chunk directory {:?}", e);
            };
        };
    }

    fn chunk_progress_report(&self, guid: &str, progress: u128, finished: bool) {
        let self_ = self.imp();
        if finished {
            debug!("Finished downloading {}", guid);
            let mut finished_files: Vec<String> = Vec::new();
            let chunks = self_.downloaded_chunks.borrow();
            self_.chunk_urls.borrow_mut().remove(guid);
            if let Some(files) = chunks.get(guid) {
                for file in files {
                    debug!("Affected files: {}", file);
                    if let Some(f) = self_.downloaded_files.borrow_mut().get_mut(file) {
                        for chunk in &f.chunks {
                            if chunk.guid == guid {
                                f.finished_chunks.push(chunk.clone());
                                break;
                            }
                        }
                        if f.finished_chunks.len() == f.chunks.len() {
                            self.extract_file_from_chunks(&mut finished_files, file, f);
                        }
                    }
                }
            }
        } else {
            let chunks = self_.downloaded_chunks.borrow();
            if let Some(files) = chunks.get(guid) {
                for file in files {
                    if let Some(f) = self_.downloaded_files.borrow_mut().get_mut(file) {
                        let Some(item) = self.get_item(&f.asset) else {
                            break;
                        };
                        item.add_downloaded_size(progress);
                        self.emit_by_name::<()>("tick", &[]);
                        break;
                    }
                }
            }
        }
    }

    fn asset_finished(&self, item: &super::download_item::EpicDownloadItem) {
        self.finish(item);
        if let Some(r) = item.release() {
            self.asset_cleanup(r);
        };
    }

    fn file_already_extracted(
        &self,
        asset_id: String,
        progress: u128,
        fullname: String,
        filename: String,
    ) {
        let self_ = self.imp();
        let Some(item) = self.get_item(&asset_id) else {
            return;
        };

        let mut targets: Vec<(String, bool)> = Vec::new();
        {
            let actions = item.actions();

            for act in actions {
                if let PostDownloadAction::Copy(t, over) = act {
                    targets.push((t.clone(), over));
                }
            }
        }

        self_.file_pool.execute(move || {
            copy_files(&PathBuf::from_str(&fullname).unwrap(), targets, &filename);
        });

        item.add_downloaded_size(progress);
        self.emit_by_name::<()>("tick", &[]);
        self_
            .sender
            .send_blocking(Msg::FileExtracted(asset_id))
            .unwrap();
    }

    fn pause_asset_download(&self, asset: String) {
        let self_ = self.imp();
        if let Some(guids) = self_.asset_guids.borrow().get(&asset) {
            for guid in guids {
                self.send_to_thread_sender(&guid.clone(), &ThreadMessages::Pause);
            }
        }
        if let Some(item) = self.get_item(&asset) {
            item.set_property("status", "Paused".to_string());
            item.set_property("speed", String::new());
        }
    }

    fn asset_cleanup(&self, asset: String) {
        let self_ = self.imp();
        if let Some(guids) = self_.asset_guids.borrow_mut().remove(&asset) {
            if let Some(item) = self.get_item(&asset) {
                if let Some(v) = item.version() {
                    self_.download_items.borrow_mut().remove(&v);
                }
                if let Some(r) = item.release() {
                    self_.download_items.borrow_mut().remove(&r);
                }
                for guid in guids {
                    for file in self_
                        .downloaded_chunks
                        .borrow_mut()
                        .remove(guid.as_str())
                        .unwrap_or_default()
                    {
                        self_.downloaded_files.borrow_mut().remove(&file);
                    }
                }
            }
        }
    }

    fn pause_asset_chunk(&self, url: Url, path: PathBuf, guid: String) {
        let self_ = self.imp();
        self_
            .paused_asset_chunks
            .borrow_mut()
            .entry(guid)
            .or_default()
            .push((url, path));
    }

    fn resume_asset_download(&self, asset: String) {
        let self_ = self.imp();
        if let Some(guids) = self_.asset_guids.borrow().get(&asset) {
            for guid in guids {
                if let Some(values) = self_.paused_asset_chunks.borrow_mut().remove(guid.as_str()) {
                    for (url, path) in values {
                        self_
                            .sender
                            .send_blocking(Msg::RedownloadChunk(
                                url.clone(),
                                path.clone(),
                                guid.clone(),
                            ))
                            .unwrap();
                    }
                }
            }
        }
    }

    fn cancel_asset_download(&self, asset: String) {
        let self_ = self.imp();
        if let Some(guids) = self_.asset_guids.borrow_mut().remove(&asset) {
            if let Some(item) = self.get_item(&asset) {
                item.set_property("status", "Canceled".to_string());
                item.set_property("speed", String::new());
                if let Some(v) = item.version() {
                    self_.download_items.borrow_mut().remove(&v);
                }
                if let Some(r) = item.release() {
                    self_.download_items.borrow_mut().remove(&r);
                }
                let paused = item.paused();
                for guid in guids {
                    self.send_to_thread_sender(&guid.clone(), &ThreadMessages::Cancel);
                    for file in self_
                        .downloaded_chunks
                        .borrow_mut()
                        .remove(guid.as_str())
                        .unwrap_or_default()
                    {
                        self_.downloaded_files.borrow_mut().remove(&file);
                    }
                    // Remove chunks if we are already in paused state
                    if paused {
                        if let Some(values) =
                            self_.paused_asset_chunks.borrow_mut().remove(guid.as_str())
                        {
                            for (url, path) in values {
                                self_
                                    .sender
                                    .send_blocking(CancelChunk(url, path, guid.clone()))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}

fn process_thread_message(
    link: &Url,
    p: &Path,
    g: &str,
    sender: &async_channel::Sender<Msg>,
    m: &ThreadMessages,
) {
    match m {
        ThreadMessages::Cancel => {
            sender
                .send_blocking(CancelChunk(link.clone(), p.to_path_buf(), g.to_string()))
                .unwrap();
        }
        ThreadMessages::Pause => {
            sender
                .send_blocking(Msg::PauseChunk(
                    link.clone(),
                    p.to_path_buf(),
                    g.to_string(),
                ))
                .unwrap();
        }
    }
}

fn save_asset_manifest(
    t: &Path,
    manifest: &egs_api::api::types::download_manifest::DownloadManifest,
) {
    if let Ok(w) = crate::RUNNING.read() {
        if !*w {
            return;
        }
    }
    let tar = match t.file_name() {
        None => {
            return;
        }
        Some(fname) => {
            if fname.eq("data") {
                match t.parent() {
                    None => {
                        return;
                    }
                    Some(p) => p,
                }
            } else {
                t
            }
        }
    };

    std::fs::create_dir_all(t).expect("Unable to create target directory");
    match File::create(tar.join("manifest.json")) {
        Ok(mut json_manifest_file) => match serde_json::to_string(&manifest) {
            Ok(json) => {
                std::io::Write::write_all(&mut json_manifest_file, json.as_bytes()).unwrap();
            }
            Err(e) => {
                error!("Unable to save json manifest: {}", e);
            }
        },
        Err(e) => {
            error!("Unable to save Manifest: {:?}", e);
        }
    }
    match File::create(tar.join("manifest")) {
        Ok(mut manifest_file) => {
            std::io::Write::write_all(&mut manifest_file, &manifest.to_vec()).unwrap();
        }
        Err(e) => {
            error!("Unable to save binary Manifest: {:?}", e);
        }
    }
}

fn initiate_file_download(
    r_id: &str,
    r_name: &str,
    f_name: &str,
    sender: &async_channel::Sender<Msg>,
    m: egs_api::api::types::download_manifest::FileManifestList,
    full_path: &Path,
) {
    debug!("Initiating file download: {}", f_name);
    if let Ok(w) = crate::RUNNING.read() {
        if !*w {
            return;
        }
    };
    match File::open(full_path) {
        Ok(mut f) => {
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut hasher = sha1::Sha1::new();
            loop {
                if let Ok(size) = f.read(&mut buffer) {
                    if size > 0 {
                        hasher.update(&buffer[..size]);
                    } else {
                        break;
                    }
                }
            }
            let hash = hasher.finalize();
            if m.file_hash
                .eq(&hash.iter().fold(String::new(), |mut output, b| {
                    write!(output, "{b:02x}").unwrap();
                    output
                }))
            {
                sender
                    .send_blocking(Msg::FileAlreadyDownloaded(
                        r_id.to_string(),
                        m.size(),
                        full_path.to_str().unwrap().to_string(),
                        f_name.to_string(),
                    ))
                    .unwrap();
            } else {
                warn!("Hashes do not match, downloading again: {:?}", full_path);
                sender
                    .send_blocking(Msg::PerformAssetDownload(
                        r_id.to_string(),
                        r_name.to_string(),
                        f_name.to_string(),
                        m,
                    ))
                    .unwrap();
            };
        }
        // File does not exist perform download
        Err(_) => {
            sender
                .send_blocking(Msg::PerformAssetDownload(
                    r_id.to_string(),
                    r_name.to_string(),
                    f_name.to_string(),
                    m,
                ))
                .unwrap();
        }
    }
}

impl AssetPriv for super::EpicDownloadManager {
    fn load_thumbnail(
        &self,
        id: String,
        thumbnail: Option<egs_api::api::types::asset_info::KeyImage>,
    ) {
        let self_ = self.imp();
        if let Some(t) = thumbnail {
            let cache_dir = self_.settings.string("cache-directory").to_string();
            let mut cache_path = std::path::PathBuf::from(cache_dir);
            let sender = self_.sender.clone();
            cache_path.push("images");
            let name = Path::new(t.url.path())
                .extension()
                .and_then(std::ffi::OsStr::to_str);
            cache_path.push(format!("{}.{}", t.md5, name.unwrap_or("png")));
            self_.thumbnail_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                }
                if cache_path.as_path().exists() {
                    match gtk4::gdk::Texture::from_file(&gtk4::gio::File::for_path(
                        cache_path.as_path(),
                    )) {
                        Ok(t) => sender
                            .send_blocking(Msg::ProcessItemThumbnail(id.clone(), t))
                            .unwrap(),
                        Err(e) => {
                            error!("Unable to load file to texture: {}", e);
                        }
                    };
                } else {
                    warn!("Need to load image");
                }
            });
        }
    }

    fn extract_file_from_chunks(
        &self,
        finished_files: &mut Vec<String>,
        file: &str,
        f: &mut DownloadedFile,
    ) {
        let self_ = self.imp();
        let vaults = self_.settings.strv("unreal-vault-directories");
        let temp_dir = std::path::PathBuf::from(vaults.first().map_or_else(
            || {
                self_
                    .settings
                    .string("temporary-download-directory")
                    .to_string()
            },
            std::string::ToString::to_string,
        ));
        let mut targets: Vec<(String, bool)> = Vec::new();
        let mut to_vault = true;
        {
            let actions = self
                .get_item(&f.asset)
                .map_or_else(Vec::new, |i| i.actions());

            for act in actions {
                match act {
                    PostDownloadAction::Copy(t, over) => {
                        targets.push((t.clone(), over));
                    }
                    PostDownloadAction::NoVault => {
                        to_vault = false;
                    }
                }
            }
        }

        debug!("File finished {}", f.name);
        finished_files.push(file.to_string());
        let finished = f.clone();
        let mut temp = temp_dir;
        temp.push(f.release.clone());
        temp.push("temp");
        let mut vault = if to_vault || targets.is_empty() {
            let mut v = match self.unreal_vault_dir(&f.asset) {
                None => {
                    return;
                }
                Some(s) => PathBuf::from(s),
            };
            v.push(f.release.clone());
            v.push("data");
            v
        } else {
            PathBuf::from_str(&targets.pop().unwrap().0).unwrap()
        };
        let sender = self_.sender.clone();
        let f_c = f.clone();
        let file_c = file.to_string();
        self_.file_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            };
            vault.push(&finished.name);
            std::fs::create_dir_all(vault.parent().unwrap()).unwrap();
            debug!("Created target directory: {:?}", vault.to_str());
            match File::create(vault.clone()) {
                Ok(mut target) => {
                    let hash =
                        extract_chunks(finished.chunks, &temp.clone(), &mut target).finalize();
                    if finished
                        .hash
                        .eq(&hash.iter().fold(String::new(), |mut output, b| {
                            write!(output, "{b:02x}").unwrap();
                            output
                        }))
                    {
                        copy_files(&vault.clone(), targets, &finished.name);
                        sender
                            .send_blocking(Msg::FinalizeFileDownload(
                                file_c.to_string(),
                                f_c.clone(),
                            ))
                            .unwrap();
                    } else {
                        error!("Failed to validate hash on: {:?}", vault);
                        // TODO: Try to download this file again
                    };
                }
                Err(e) => {
                    error!("Error opening the target file: {:?}", e);
                }
            }
        });
    }
}

fn extract_chunks(
    chunks: Vec<egs_api::api::types::download_manifest::FileChunkPart>,
    temp: &Path,
    target: &mut File,
) -> CoreWrapper<Sha1Core> {
    let mut hasher = Sha1::new();
    for chunk in chunks {
        let mut t = temp.to_path_buf();
        t.push(format!("{}.chunk", chunk.guid));
        match File::open(t) {
            Ok(mut f) => {
                let metadata = f.metadata().expect("Unable to read metadata");
                let mut buffer = vec![0_u8; usize::try_from(metadata.len()).unwrap()];
                f.read_exact(&mut buffer).expect("Read failed");
                let ch = match egs_api::api::types::chunk::Chunk::from_vec(buffer) {
                    None => {
                        error!("Failed to parse chunk from file: {:?}", chunk.link);
                        break;
                    }
                    Some(c) => c,
                };
                if u128::from(
                    ch.uncompressed_size
                        .unwrap_or_else(|| u32::try_from(ch.data.len()).unwrap()),
                ) < chunk.offset + chunk.size
                {
                    error!("Chunk is not big enough");
                    break;
                };
                hasher
                    .update(&ch.data[chunk.offset as usize..(chunk.offset + chunk.size) as usize]);
                std::io::Write::write_all(
                    target,
                    &ch.data[chunk.offset as usize..(chunk.offset + chunk.size) as usize],
                )
                .unwrap();
            }
            Err(e) => {
                error!("Error opening the chunk file: {:?}", e);
            }
        }
        debug!("chunk: {:?}", chunk);
    }
    hasher
}

fn copy_files(from: &Path, targets: Vec<(String, bool)>, filename: &str) {
    for t in targets {
        let mut tar = PathBuf::from_str(&t.0).unwrap();
        tar.push(filename);
        if tar.exists() && !t.1 {
            continue;
        }
        if tar.exists() {
            let ext = tar.extension().map_or_else(
                || OsString::from_str(".bak").unwrap(),
                |ext| {
                    let mut new = ext.to_os_string();
                    new.push(".bak");
                    new
                },
            );
            let mut bak = tar.clone();
            bak.set_extension(ext);
            if let Err(err) = std::fs::rename(&tar, bak) {
                error!("Unable to create backup: {:?}", err);
            };
        }
        std::fs::create_dir_all(tar.parent().unwrap()).unwrap();
        if let Err(e) = std::fs::copy(from, tar) {
            error!("Unable to copy file: {:?}", e);
        };
    }
}
