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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    /// Add a FAB asset for download
    /// Entry point for FAB marketplace asset downloads
    fn add_fab_asset_download(
        &self,
        _fab_asset: egs_api::api::types::fab_library::FabAsset,
        _artifact_id: String,
        _platform: String,
        _target: &Option<String>,
    ) {
        unimplemented!()
    }

    /// Fetch FAB download manifest from the Fab API
    /// Calls fab_asset_manifest() then fab_download_manifest() to obtain a DownloadManifest
    fn download_fab_asset_manifest(
        &self,
        _asset_id: String,
        _namespace: String,
        _artifact_id: String,
        _platform: String,
        _sender: async_channel::Sender<(
            String,
            Vec<egs_api::api::types::download_manifest::DownloadManifest>,
        )>,
    ) {
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

        // Debug logging to file
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/asset_click.log")
        {
            let _ = writeln!(
                f,
                "[add_asset_download] called for: {:?}, release_id: {}",
                asset.title, release_id
            );
        }

        let self_ = self.imp();

        let item = {
            let mut state = self_.state.borrow_mut();
            match state.download_items.get_mut(&release_id) {
                None => {
                    let item = super::download_item::EpicDownloadItem::new();
                    debug!("Adding item to the list under: {}", release_id);
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/asset_click.log")
                    {
                        let _ = writeln!(
                            f,
                            "[add_asset_download] Creating new download item for: {}",
                            release_id
                        );
                    }
                    state
                        .download_items
                        .insert(release_id.clone(), item.clone());
                    item
                }
                Some(_) => {
                    // Item is already downloading do nothing
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/asset_click.log")
                    {
                        let _ = writeln!(
                            f,
                            "[add_asset_download] Item already exists, returning early"
                        );
                    }
                    return;
                }
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

        // Notify library that this asset is downloading
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l = w_.logged_in_stack.clone();
            let l_ = l.imp();
            l_.library.set_asset_downloading(&asset.id, true);
        }

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
                    let _ = sender.send_blocking(super::Msg::StartAssetDownload(id, manifest));
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

        // Debug: check if window is set
        {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/asset_click.log")
            {
                let has_window = self_.window.get().is_some();
                let _ = writeln!(
                    f,
                    "[download_asset_manifest] called for: {:?}, has_window: {}",
                    asset.title, has_window
                );
            }
        }

        if let Some(window) = self_.window.get() {
            {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/asset_click.log")
                {
                    let _ = writeln!(
                        f,
                        "[download_asset_manifest] Window found, queuing manifest download"
                    );
                }
            }
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let id = release_id.clone();
            self_.download_pool.execute(move || {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                    let _ = writeln!(f, "[download_asset_manifest] Thread pool executing for: {}", id);
                }
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let start = std::time::Instant::now();
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                    let _ = writeln!(f, "[download_asset_manifest] Looking for release with app_id: {}", release_id);
                }
                // Find release by app_id (release_id parameter is actually the app_id)
                let release_info = asset.release_info.as_ref().and_then(|releases| {
                    releases.iter().find(|r| r.app_id.as_ref() == Some(&release_id)).cloned()
                });
                if let Some(_release_info) = release_info {
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                        let _ = writeln!(f, "[download_asset_manifest] Found release_info, fetching asset manifest...");
                    }
                    if let Some(manifest) = crate::RUNTIME.block_on(eg.asset_manifest(
                        None,
                        None,
                        Some(asset.namespace.clone()),
                        Some(asset.id.clone()),
                        Some(release_id.clone()),
                    )) {
                        debug!("Got asset manifest: {:?}", manifest);
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                            let _ = writeln!(f, "[download_asset_manifest] Got asset manifest, fetching download manifests...");
                        }
                        let d = crate::RUNTIME.block_on(eg.asset_download_manifests(manifest));
                        debug!("Got asset download manifests for {}", id);
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                            let _ = writeln!(f, "[download_asset_manifest] Got {} download manifests, sending to channel", d.len());
                        }
                        let _ = sender.send_blocking((id, d));
                        // TODO cache download manifest
                    } else if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                        let _ = writeln!(f, "[download_asset_manifest] ERROR: Failed to get asset manifest from API");
                    }
                } else if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/asset_click.log") {
                    let _ = writeln!(f, "[download_asset_manifest] ERROR: No release_info found for: {}", release_id);
                }
                debug!("Download Manifest requests took {:?}", start.elapsed());
            });
        } else {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/asset_click.log")
            {
                let _ = writeln!(
                    f,
                    "[download_asset_manifest] ERROR: No window set on download manager!"
                );
            }
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
        {
            let mut state = self_.state.borrow_mut();
            for manifest in dm {
                for m in manifest.files().values() {
                    for chunk in m.file_chunk_parts.clone() {
                        if let Some(url) = chunk.link {
                            match state.chunk_urls.get_mut(&chunk.guid) {
                                None => {
                                    state.chunk_urls.insert(chunk.guid, vec![url.clone()]);
                                }
                                Some(v) => {
                                    v.push(url.clone());
                                }
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
        {
            let mut state = self_.state.borrow_mut();
            state.downloaded_files.insert(
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
        }
        let sender = self_.sender.clone();
        for chunk in manifest.file_chunk_parts {
            // perform chunk download make sure we do not download the same chunk twice
            let (should_redownload, chunks_len) = {
                let mut state = self_.state.borrow_mut();
                state
                    .asset_guids
                    .entry(id.clone())
                    .or_default()
                    .push(chunk.guid.clone());
                let mut should_redownload = false;
                match state.downloaded_chunks.get_mut(&chunk.guid) {
                    None => {
                        state
                            .downloaded_chunks
                            .insert(chunk.guid.clone(), vec![full_filename.clone()]);
                        info!("Inserting file into chunk init {}", full_filename.clone());
                        should_redownload = true;
                    }
                    Some(files) => {
                        files.push(full_filename.clone());
                        info!("Inserting file into chunk {}", full_filename.clone());
                    }
                }
                (should_redownload, state.downloaded_chunks.len())
            };
            if should_redownload {
                let mut p = target.clone();
                let g = chunk.guid.clone();
                p.push(format!("{g}.chunk"));
                let _ =
                    sender.send_blocking(Msg::RedownloadChunk(Url::parse("unix:/").unwrap(), p, g));
            }
            info!("Chunks length: {}", chunks_len);
        }
    }

    fn redownload_chunk(&self, link: &Url, p: PathBuf, g: &str) {
        let self_ = self.imp();
        let sender = self_.sender.clone();
        let mut state = self_.state.borrow_mut();
        match state.chunk_urls.get_mut(g) {
            None => {
                // Unable to get chunk urls
                let _ =
                    sender.send_blocking(Msg::PerformChunkDownload(link.clone(), p, g.to_string()));
            }
            Some(v) => {
                v.retain(|x| !x.eq(link));
                if v.is_empty() {
                    // No other URL available, redownloading
                    //TODO: This has the potential to loop forever
                    let _ = sender.send_blocking(Msg::PerformChunkDownload(
                        link.clone(),
                        p.clone(),
                        g.to_string(),
                    ));
                };
                let mut rng = rand::rng();
                let index = rng.random_range(0..v.len());
                let new: Option<&Url> = v.get(index);
                match new {
                    None => {
                        // Unable to get random URL, retrying the same one
                        let _ = sender.send_blocking(Msg::PerformChunkDownload(
                            link.clone(),
                            p,
                            g.to_string(),
                        ));
                    }
                    Some(u) => {
                        // Using new url to redownload the chunk
                        let _ = sender.send_blocking(Msg::PerformChunkDownload(
                            u.clone(),
                            p,
                            g.to_string(),
                        ));
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
            if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                return;
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
            let Some(parent) = p.parent() else {
                error!("Chunk path has no parent: {:?}", p);
                return;
            };
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create chunk directory {:?}: {}", parent, e);
                return;
            }
            let mut client = match reqwest::blocking::get(link.clone()) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to start chunk download, trying again later: {}", e);
                    let _ = sender.send_blocking(Msg::RedownloadChunk(
                        link.clone(),
                        p.clone(),
                        g.clone(),
                    ));
                    return;
                }
            };
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut downloaded: u128 = 0;
            let mut file = match File::create(&p) {
                Ok(file) => file,
                Err(e) => {
                    error!("Unable to create chunk file {:?}: {}", p, e);
                    return;
                }
            };
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
                            if let Err(e) = std::io::Write::write_all(&mut file, &buffer[0..size]) {
                                error!("Failed to write chunk data for {}: {}", g, e);
                                return;
                            }
                            let _ = sender.send_blocking(Msg::ChunkDownloadProgress(
                                g.clone(),
                                size as u128,
                                false,
                            ));
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
            let _ = sender.send_blocking(Msg::ChunkDownloadProgress(g.clone(), downloaded, true));
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
            let mut to_extract: Vec<(String, DownloadedFile)> = Vec::new();
            {
                let mut state = self_.state.borrow_mut();
                state.chunk_urls.remove(guid);
                let affected_files = state
                    .downloaded_chunks
                    .get(guid)
                    .cloned()
                    .unwrap_or_default();
                for file in &affected_files {
                    debug!("Affected files: {}", file);
                    if let Some(f) = state.downloaded_files.get_mut(file) {
                        for chunk in &f.chunks {
                            if chunk.guid == guid {
                                f.finished_chunks.push(chunk.clone());
                                break;
                            }
                        }
                        if f.finished_chunks.len() == f.chunks.len() {
                            to_extract.push((file.clone(), f.clone()));
                        }
                    }
                }
            }
            for (file, mut file_details) in to_extract {
                self.extract_file_from_chunks(&mut finished_files, &file, &mut file_details);
            }
        } else {
            let files = {
                let state = self_.state.borrow();
                state.downloaded_chunks.get(guid).cloned()
            };
            if let Some(files) = files {
                for file in files {
                    let asset_id = {
                        let state = self_.state.borrow();
                        state.downloaded_files.get(&file).map(|f| f.asset.clone())
                    };
                    if let Some(asset_id) = asset_id {
                        let Some(item) = self.get_item(&asset_id) else {
                            break;
                        };
                        item.add_downloaded_size(progress);
                        self.emit_by_name::<()>("tick", &[]);

                        // Update library asset progress (throttled to avoid UI flooding)
                        if let Some(asset_id) = item.asset() {
                            let total = item.total_size();
                            let downloaded = item.downloaded_size();
                            if total > 0 {
                                let progress_fraction = downloaded as f64 / total as f64;
                                // Only update every ~1% to avoid flooding UI
                                let progress_pct = (progress_fraction * 100.0) as u32;
                                let old_pct =
                                    ((downloaded - progress) as f64 / total as f64 * 100.0) as u32;
                                if progress_pct != old_pct {
                                    if let Some(w) = self_.window.get() {
                                        let w_ = w.imp();
                                        let l = w_.logged_in_stack.clone();
                                        let l_ = l.imp();
                                        // Get speed from item
                                        let speed: Option<String> = item.property("speed");
                                        let speed_str = speed.unwrap_or_default();
                                        l_.library.set_asset_download_progress(
                                            &asset_id,
                                            progress_fraction,
                                            &speed_str,
                                        );
                                    }
                                }
                            }
                        }
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
        let _ = self_.sender.send_blocking(Msg::FileExtracted(asset_id));
    }

    fn pause_asset_download(&self, asset: String) {
        let self_ = self.imp();
        let guids = {
            let state = self_.state.borrow();
            state.asset_guids.get(&asset).cloned()
        };
        if let Some(guids) = guids {
            for guid in guids {
                self.send_to_thread_sender(&guid, &ThreadMessages::Pause);
            }
        }
        if let Some(item) = self.get_item(&asset) {
            item.set_property("status", "Paused".to_string());
            item.set_property("speed", String::new());
        }
    }

    fn asset_cleanup(&self, asset: String) {
        let self_ = self.imp();
        let item = self.get_item(&asset);
        let mut state = self_.state.borrow_mut();
        if let Some(guids) = state.asset_guids.remove(&asset) {
            if let Some(item) = item {
                if let Some(v) = item.version() {
                    state.download_items.remove(&v);
                }
                if let Some(r) = item.release() {
                    state.download_items.remove(&r);
                }
                for guid in guids {
                    for file in state
                        .downloaded_chunks
                        .remove(guid.as_str())
                        .unwrap_or_default()
                    {
                        state.downloaded_files.remove(&file);
                    }
                }
            }
        }
    }

    fn pause_asset_chunk(&self, url: Url, path: PathBuf, guid: String) {
        let self_ = self.imp();
        self_
            .state
            .borrow_mut()
            .paused_asset_chunks
            .entry(guid)
            .or_default()
            .push((url, path));
    }

    fn resume_asset_download(&self, asset: String) {
        let self_ = self.imp();
        let pending = {
            let mut state = self_.state.borrow_mut();
            let mut pending = Vec::new();
            let guids = state.asset_guids.get(&asset).cloned().unwrap_or_default();
            for guid in &guids {
                if let Some(values) = state.paused_asset_chunks.remove(guid.as_str()) {
                    pending.push((guid.clone(), values));
                }
            }
            pending
        };
        for (guid, values) in pending {
            for (url, path) in values {
                let _ = self_.sender.send_blocking(Msg::RedownloadChunk(
                    url.clone(),
                    path.clone(),
                    guid.clone(),
                ));
            }
        }
    }

    fn cancel_asset_download(&self, asset: String) {
        let self_ = self.imp();
        let item = self.get_item(&asset);
        let paused = item.as_ref().map_or(false, |item| item.paused());
        if let Some(item) = &item {
            item.set_property("status", "Canceled".to_string());
            item.set_property("speed", String::new());
        }

        let (guids, paused_chunks) = {
            let mut state = self_.state.borrow_mut();
            let guids = state.asset_guids.remove(&asset);
            let mut paused_chunks = Vec::new();
            if let Some(item) = &item {
                if let Some(v) = item.version() {
                    state.download_items.remove(&v);
                }
                if let Some(r) = item.release() {
                    state.download_items.remove(&r);
                }
            }
            if let Some(ref guids) = guids {
                for guid in guids {
                    for file in state
                        .downloaded_chunks
                        .remove(guid.as_str())
                        .unwrap_or_default()
                    {
                        state.downloaded_files.remove(&file);
                    }
                    // Remove chunks if we are already in paused state
                    if paused {
                        if let Some(values) = state.paused_asset_chunks.remove(guid.as_str()) {
                            paused_chunks.push((guid.clone(), values));
                        }
                    }
                }
            }
            (guids, paused_chunks)
        };

        if let Some(guids) = guids {
            for guid in guids {
                self.send_to_thread_sender(&guid, &ThreadMessages::Cancel);
            }
        }
        for (guid, values) in paused_chunks {
            for (url, path) in values {
                let _ = self_
                    .sender
                    .send_blocking(CancelChunk(url, path, guid.clone()));
            }
        }
    }

    fn add_fab_asset_download(
        &self,
        fab_asset: egs_api::api::types::fab_library::FabAsset,
        artifact_id: String,
        platform: String,
        target: &Option<String>,
    ) {
        debug!("Adding FAB download: {}", fab_asset.title);
        let self_ = self.imp();
        let asset_id = fab_asset.asset_id.clone();

        let item = {
            let mut state = self_.state.borrow_mut();
            match state.download_items.get_mut(&asset_id) {
                None => {
                    let item = super::download_item::EpicDownloadItem::new();
                    debug!("Adding FAB item to the list under: {}", asset_id);
                    state.download_items.insert(asset_id.clone(), item.clone());
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
        item.set_property(
            "item-type",
            crate::ui::widgets::download_manager::download_item::ItemType::Asset,
        );
        item.set_property("asset", asset_id.clone());
        item.set_property("release", artifact_id.clone());
        item.set_property("label", fab_asset.title.clone());
        item.set_property("target", target.clone());
        item.set_property("status", "initializing...".to_string());

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
                    let _ = sender.send_blocking(super::Msg::StartFabAssetDownload(id, manifest));
                }
            }
        ));

        let namespace = fab_asset.asset_namespace.clone();
        self.download_fab_asset_manifest(asset_id, namespace, artifact_id, platform, sender);
    }

    fn download_fab_asset_manifest(
        &self,
        asset_id: String,
        namespace: String,
        artifact_id: String,
        platform: String,
        sender: async_channel::Sender<(
            String,
            Vec<egs_api::api::types::download_manifest::DownloadManifest>,
        )>,
    ) {
        let self_ = self.imp();

        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let eg = win_.model.borrow().epic_games.borrow().clone();
            let id = asset_id.clone();
            self_.download_pool.execute(move || {
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let start = std::time::Instant::now();
                match crate::RUNTIME.block_on(eg.fab_asset_manifest(
                    &artifact_id,
                    &namespace,
                    &asset_id,
                    Some(&platform),
                )) {
                    Ok(download_infos) => {
                        if let Some(download_info) = download_infos.into_iter().next() {
                            if let Some(base_url) =
                                download_info.distribution_point_base_urls.first()
                            {
                                match crate::RUNTIME.block_on(
                                    eg.fab_download_manifest(download_info.clone(), base_url),
                                ) {
                                    Ok(manifest) => {
                                        debug!(
                                            "Got FAB download manifest for {} in {:?}",
                                            id,
                                            start.elapsed()
                                        );
                                        let _ = sender.send_blocking((id, vec![manifest]));
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to get FAB download manifest for {}: {:?}",
                                            id, e
                                        );
                                    }
                                }
                            } else {
                                error!("No distribution point base URLs for FAB asset {}", id);
                            }
                        } else {
                            error!("No download info returned for FAB asset {}", id);
                        }
                    }
                    Err(e) => {
                        error!("Failed to get FAB asset manifest for {}: {:?}", id, e);
                    }
                }
                debug!("FAB manifest requests took {:?}", start.elapsed());
            });
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
            let _ = sender.send_blocking(CancelChunk(link.clone(), p.to_path_buf(), g.to_string()));
        }
        ThreadMessages::Pause => {
            let _ = sender.send_blocking(Msg::PauseChunk(
                link.clone(),
                p.to_path_buf(),
                g.to_string(),
            ));
        }
    }
}

fn save_asset_manifest(
    t: &Path,
    manifest: &egs_api::api::types::download_manifest::DownloadManifest,
) {
    if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
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

    if let Err(e) = std::fs::create_dir_all(t) {
        error!("Unable to create target directory {:?}: {}", t, e);
        return;
    }
    match File::create(tar.join("manifest.json")) {
        Ok(mut json_manifest_file) => match serde_json::to_string(&manifest) {
            Ok(json) => {
                if let Err(e) = std::io::Write::write_all(&mut json_manifest_file, json.as_bytes())
                {
                    error!("Unable to write json manifest: {}", e);
                }
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
            if let Err(e) = std::io::Write::write_all(&mut manifest_file, &manifest.to_vec()) {
                error!("Unable to write binary manifest: {}", e);
            }
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
    if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
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
                let _ = sender.send_blocking(Msg::FileAlreadyDownloaded(
                    r_id.to_string(),
                    m.size(),
                    full_path.to_string_lossy().to_string(),
                    f_name.to_string(),
                ));
            } else {
                warn!("Hashes do not match, downloading again: {:?}", full_path);
                let _ = sender.send_blocking(Msg::PerformAssetDownload(
                    r_id.to_string(),
                    r_name.to_string(),
                    f_name.to_string(),
                    m,
                ));
            };
        }
        // File does not exist perform download
        Err(_) => {
            let _ = sender.send_blocking(Msg::PerformAssetDownload(
                r_id.to_string(),
                r_name.to_string(),
                f_name.to_string(),
                m,
            ));
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
                if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                if cache_path.as_path().exists() {
                    match gtk4::gdk::Texture::from_file(&gtk4::gio::File::for_path(
                        cache_path.as_path(),
                    )) {
                        Ok(t) => {
                            let _ = sender.send_blocking(Msg::ProcessItemThumbnail(id.clone(), t));
                        }
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
            if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            };
            vault.push(&finished.name);
            let Some(parent) = vault.parent() else {
                error!("Vault path has no parent: {:?}", vault);
                return;
            };
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create vault directory {:?}: {}", parent, e);
                return;
            }
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
                        let _ = sender.send_blocking(Msg::FinalizeFileDownload(
                            file_c.to_string(),
                            f_c.clone(),
                        ));
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
        match File::open(&t) {
            Ok(mut f) => {
                let metadata = match f.metadata() {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        error!("Unable to read metadata for chunk {:?}: {}", t, e);
                        continue;
                    }
                };
                let buffer_len = usize::try_from(metadata.len()).unwrap_or_default();
                if buffer_len == 0 {
                    error!("Chunk metadata length is zero for {:?}", t);
                    continue;
                }
                let mut buffer = vec![0_u8; buffer_len];
                if let Err(e) = f.read_exact(&mut buffer) {
                    error!("Failed to read chunk {:?}: {}", t, e);
                    continue;
                }
                let ch = match egs_api::api::types::chunk::Chunk::from_vec(buffer) {
                    None => {
                        error!("Failed to parse chunk from file: {:?}", chunk.link);
                        break;
                    }
                    Some(c) => c,
                };
                if u128::from(
                    ch.uncompressed_size
                        .unwrap_or_else(|| u32::try_from(ch.data.len()).unwrap_or_default()),
                ) < chunk.offset + chunk.size
                {
                    error!("Chunk is not big enough");
                    break;
                };
                hasher
                    .update(&ch.data[chunk.offset as usize..(chunk.offset + chunk.size) as usize]);
                if let Err(e) = std::io::Write::write_all(
                    target,
                    &ch.data[chunk.offset as usize..(chunk.offset + chunk.size) as usize],
                ) {
                    error!("Failed to write chunk data to target: {}", e);
                    break;
                }
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
        let Some(parent) = tar.parent() else {
            error!("Target path has no parent: {:?}", tar);
            continue;
        };
        if let Err(e) = std::fs::create_dir_all(parent) {
            error!("Unable to create target directory {:?}: {}", parent, e);
            continue;
        }
        if let Err(e) = std::fs::copy(from, tar) {
            error!("Unable to copy file: {:?}", e);
        };
    }
}
