use crate::download::DownloadedFile;
use crate::window::EpicAssetManagerWindow;
use byte_unit::Byte;
use egs_api::api::types::download_manifest::FileManifestList;
use gtk4::CheckButton;
use log::{debug, error, info, warn};
use reqwest::Url;
use sha1::{Digest, Sha1};
use slab_tree::{NodeId, NodeMut, NodeRef, Tree, TreeBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::{fs, thread};

pub(crate) trait Chunks {
    fn chunk_extraction_finished(&mut self, _file: String, _path: PathBuf) {
        unimplemented!()
    }

    fn chunk_download_progress_report(&mut self, _guid: String, _progress: u128, _finished: bool) {
        unimplemented!()
    }

    fn chunk_init_download(&mut self, _all: bool, _asset_id: String, _release: String) {
        unimplemented!()
    }

    fn download_file_validated(
        &mut self,
        _asset_id: String,
        _release: String,
        _filename: String,
        _manifest: FileManifestList,
    ) {
        unimplemented!()
    }

    fn select_file_for_download(
        &mut self,
        _asset_id: String,
        _app_name: Option<String>,
        _filename: Option<String>,
        _chbox_id: NodeId,
        _size: u128,
    ) {
        unimplemented!()
    }
}

impl Chunks for EpicAssetManagerWindow {
    fn chunk_extraction_finished(&mut self, file: String, path: PathBuf) {
        info!("File finished {}", file);
        for (chunk, files) in self.model.downloaded_chunks.iter_mut() {
            let mut p = path.clone();
            files.retain(|x| !x.eq(&file));
            if files.is_empty() {
                p.push("temp");
                p.push(format!("{}.chunk", chunk));
                debug!("Removing chunk {}", p.as_path().to_str().unwrap());
                if let Err(e) = fs::remove_file(p.clone()) {
                    error!("Unable to remove chunk file: {}", e);
                };
                if let Err(e) = fs::remove_dir(p.parent().unwrap().clone()) {
                    debug!("Unable to remove the temp directory(yet): {}", e)
                };
            };
        }
        self.model.downloaded_chunks.retain(|_, v| !v.is_empty());
    }

    fn chunk_download_progress_report(&mut self, guid: String, progress: u128, finished: bool) {
        if finished {
            debug!("Finished downloading {}", guid);
            let mut finished_files: Vec<String> = Vec::new();
            if let Some(files) = self.model.downloaded_chunks.get(&guid) {
                for file in files {
                    debug!("Affected files: {}", file);
                    if let Some(f) = self.model.downloaded_files.get_mut(file) {
                        for chunk in &f.chunks {
                            if chunk.guid == guid {
                                f.finished_chunks.push(chunk.clone());
                                break;
                            }
                        }
                        if f.finished_chunks.len() == f.chunks.len() {
                            debug!("File finished {}", f.name);
                            finished_files.push(file.clone());
                            let finished = f.clone();
                            let mut path = PathBuf::from(
                                self.model
                                    .configuration
                                    .directories
                                    .unreal_vault_directory
                                    .clone(),
                            );
                            path.push(finished.release.clone());
                            let mut temp_path = PathBuf::from(
                                self.model
                                    .configuration
                                    .directories
                                    .temporary_download_directory
                                    .clone(),
                            );
                            temp_path.push(finished.release.clone());
                            let stream = self.model.relm.stream().clone();
                            let msg_path = temp_path.clone();
                            let (_channel, sender) = Channel::new(move |f| {
                                stream.emit(crate::ui::messages::Msg::ExtractionFinished(
                                    f,
                                    msg_path.clone(),
                                ))
                            });
                            temp_path.push("temp");
                            path.push("data");

                            let chunk_file = file.clone();
                            self.model.file_pool.execute(move || {
                                path.push(finished.name);
                                fs::create_dir_all(path.parent().unwrap().clone()).unwrap();
                                match File::create(path.clone()) {
                                    Ok(mut target) => {
                                        let mut hasher = Sha1::new();
                                        for chunk in finished.chunks {
                                            let mut t = temp_path.clone();
                                            t.push(format!("{}.chunk", chunk.guid));
                                            match File::open(t) {
                                                Ok(mut f) => {
                                                    let metadata = f
                                                        .metadata()
                                                        .expect("Unable to read metadata");
                                                    let mut buffer =
                                                        vec![0 as u8; metadata.len() as usize];
                                                    f.read(&mut buffer).expect("Read failed");
                                                    let ch =
                                                        egs_api::api::types::chunk::Chunk::from_vec(
                                                            buffer,
                                                        ).unwrap();
                                                    if (ch
                                                        .uncompressed_size
                                                        .unwrap_or(ch.data.len() as u32)
                                                        as u128)
                                                        < chunk.offset + chunk.size
                                                    {
                                                        error!("Chunk is not big enough");
                                                        break;
                                                    };
                                                    hasher.update(
                                                        &ch.data[chunk.offset as usize
                                                            ..(chunk.offset + chunk.size) as usize],
                                                    );
                                                    target
                                                        .write(
                                                            &ch.data[chunk.offset as usize
                                                                ..(chunk.offset + chunk.size)
                                                                    as usize],
                                                        )
                                                        .unwrap();
                                                }
                                                Err(e) => {
                                                    error!("Error opening the chunk file: {:?}", e)
                                                }
                                            }
                                            debug!("chunk: {:?}", chunk);
                                        }
                                        let hash = hasher.finalize();
                                        if !finished.hash.eq(&hash
                                            .iter()
                                            .map(|b| format!("{:02x}", b))
                                            .collect::<String>())
                                        {
                                            error!("Failed to validate hash on: {:?}", path);
                                        };
                                        sender.send(chunk_file).unwrap();
                                    }
                                    Err(e) => {
                                        error!("Error opening the target file: {:?}", e)
                                    }
                                }
                            })
                        }
                    }
                    self.model
                        .downloaded_files
                        .retain(|k, _| !finished_files.contains(k))
                }
            }
        } else {
            debug!("Got progress report from {}, current: {}", guid, progress);
        }
    }

    fn chunk_init_download(&mut self, all: bool, asset_id: String, release: String) {
        info!(
            "Starting download for {} release {}",
            asset_id.clone(),
            release.clone()
        );
        let asset = match crate::DATA.asset_info.read() {
            Ok(asset_map) => match asset_map.get(asset_id.as_str()) {
                None => {
                    return;
                }
                Some(a) => a.clone(),
            },
            Err(_) => {
                return;
            }
        };

        let rel = match asset.release_name(&release) {
            None => {
                return;
            }
            Some(rel) => rel,
        };

        if let Ok(download_manifests) = crate::DATA.download_manifests.read() {
            if let Some(dm) =
                download_manifests.get(rel.id.clone().unwrap_or(asset_id.clone()).as_str())
            {
                let files = if !all {
                    debug!("Not downloading all");
                    if let Some(map) = self.model.selected_files.get(&asset.id) {
                        if let Some(files) = map.get(&release) {
                            Some(files)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                // Save download manifest
                let manifest = dm.clone();
                let mut target = PathBuf::from(
                    self.model
                        .configuration
                        .directories
                        .unreal_vault_directory
                        .clone(),
                );
                target.push(release.clone());

                // Save manifest files
                let t = target.clone();
                self.model.download_pool.execute(move || {
                    fs::create_dir_all(t.clone()).expect("Unable to create target directory");
                    match File::create(t.as_path().join("manifest.json")) {
                        Ok(mut json_manifest_file) => {
                            json_manifest_file
                                .write(
                                    serde_json::to_string(&manifest)
                                        .unwrap()
                                        .as_bytes()
                                        .as_ref(),
                                )
                                .unwrap();
                        }
                        Err(e) => {
                            error!("Unable to save Manifest: {:?}", e)
                        }
                    }
                    match File::create(t.as_path().join("manifest")) {
                        Ok(mut manifest_file) => {
                            manifest_file.write(&manifest.to_vec()).unwrap();
                        }
                        Err(e) => {
                            error!("Unable to save binary Manifest: {:?}", e)
                        }
                    }
                });

                for (filename, manifest) in dm.files() {
                    debug!("Checking download {}", filename);
                    if let Some(file_list) = files {
                        if !file_list.contains(&filename) {
                            continue;
                        }
                    }
                    info!("Starting download of {}", filename);
                    let stream = self.model.relm.stream().clone();
                    let a_id = asset.id.clone();
                    let r_id = release.clone();
                    let f_name = filename.clone();
                    let (_channel, sender) = Channel::new(move |m| {
                        stream.emit(crate::ui::messages::Msg::DownloadFileValidated(
                            a_id.clone(),
                            r_id.clone(),
                            f_name.clone(),
                            m,
                        ))
                    });

                    let m = manifest.clone();
                    let full_path = target.clone().as_path().join("data").join(filename);
                    thread::spawn(move || match File::open(full_path.clone()) {
                        Ok(mut f) => {
                            let mut buffer: [u8; 1024] = [0; 1024];
                            let mut hasher = Sha1::new();
                            loop {
                                match f.read(&mut buffer) {
                                    Ok(size) => {
                                        if size > 0 {
                                            hasher.update(&buffer[..size]);
                                        } else {
                                            break;
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            let hash = hasher.finalize();
                            if !m.file_hash.eq(&hash
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>())
                            {
                                warn!("Hashes do not match, downloading again: {:?}", full_path);
                                sender.send(m).unwrap();
                            };
                        }
                        Err(_) => {
                            sender.send(m).unwrap();
                        }
                    });
                }
            }
        };
    }

    fn download_file_validated(
        &mut self,
        asset_id: String,
        release: String,
        filename: String,
        manifest: FileManifestList,
    ) {
        let mut path = PathBuf::from(
            self.model
                .configuration
                .directories
                .unreal_vault_directory
                .clone(),
        );

        path.push(release.clone());
        path.push("temp");
        let downloaded = DownloadedFile {
            asset: asset_id.clone(),
            release: release.clone(),
            name: filename.clone(),
            chunks: manifest.file_chunk_parts.clone(),
            hash: manifest.file_hash,
            finished_chunks: vec![],
        };
        let full_filename = format!(
            "{}/{}/{}",
            asset_id.clone(),
            release.clone(),
            filename.clone()
        );
        self.model
            .downloaded_files
            .insert(full_filename.clone(), downloaded);
        for chunk in manifest.file_chunk_parts {
            match self.model.downloaded_chunks.get_mut(&chunk.guid) {
                None => {
                    self.model
                        .downloaded_chunks
                        .insert(chunk.guid.clone(), vec![full_filename.clone()]);
                    let link = chunk.link.unwrap();
                    let mut p = path.clone();
                    let g = chunk.guid.clone();
                    p.push(format!("{}.chunk", g));
                    let sender = self
                        .widgets
                        .asset_download_widgets
                        .download_progress_sender
                        .clone();
                    self.model.download_pool.execute(move || {
                        EpicAssetManagerWindow::perform_download(link, p, g, sender);
                    });
                }
                Some(files) => files.push(full_filename.clone()),
            }
        }
    }

    fn select_file_for_download(
        &mut self,
        asset_id: String,
        app_name: Option<String>,
        filename: Option<String>,
        chbox_id: NodeId,
        size: u128,
    ) {
        let mut ch = true;
        match app_name {
            // We received checked from a non file node
            None => {
                let mut changed_children: Vec<(String, String, String, u128)> = Vec::new();
                if let Some(chbox) = self.model.download_manifest_tree.get(chbox_id) {
                    ch = match chbox.data() {
                        Some(c) => c.clone().is_active(),
                        _ => false,
                    };
                    self.change_children_state(chbox, ch, &mut changed_children);
                }
                for (a_id, app_name, filename, size) in changed_children {
                    self.toggle_file_for_download(
                        a_id.clone(),
                        app_name.clone(),
                        filename.clone(),
                        Some(ch),
                        size,
                    );
                }
            }
            Some(a_name) => {
                // Selected files
                if let Some(f_name) = filename {
                    ch = self.toggle_file_for_download(asset_id, a_name, f_name, None, size);
                }
            }
        };
        if ch {
            self.check_parents(chbox_id);
        } else {
            let mut bool_tree: Tree<(bool, Option<NodeId>)> =
                TreeBuilder::new().with_root((false, None)).build();
            build_bool_tree(
                self.model.download_manifest_tree.root().unwrap(),
                bool_tree.root_mut().unwrap(),
                chbox_id,
            );
            self.uncheck_boxes(bool_tree.root().unwrap(), chbox_id, false)
        }
        self.widgets
            .asset_download_widgets
            .selected_files_size
            .set_text(
                &Byte::from_bytes(self.model.selected_files_size)
                    .get_appropriate_unit(false)
                    .to_string(),
            )
    }
}

impl EpicAssetManagerWindow {
    fn perform_download(link: Url, p: PathBuf, g: String, sender: Sender<(String, u128, bool)>) {
        debug!(
            "Downloading chunk {} from {} to {:?}",
            g,
            link.to_string(),
            p
        );
        fs::create_dir_all(p.parent().unwrap().clone()).unwrap();
        let mut client = reqwest::blocking::get(link).unwrap();
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut downloaded: u128 = 0;
        let mut file = File::create(p).unwrap();
        loop {
            match client.read(&mut buffer) {
                Ok(size) => {
                    if size > 0 {
                        downloaded += size as u128;
                        file.write(&buffer[0..size]).unwrap();
                        sender.send((g.clone(), size as u128, false)).unwrap();
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
        sender.send((g.clone(), downloaded.clone(), true)).unwrap();
    }

    fn check_parents(&self, nid: NodeId) {
        if let Some(chbox) = self.model.download_manifest_tree.get(nid) {
            if let Some(parent) = chbox.parent() {
                let mut all_checked = true;

                for child in parent.children() {
                    if let Some(ch) = child.data() {
                        if !ch.clone().is_active() {
                            all_checked = false;
                            break;
                        }
                    }
                }
                if all_checked {
                    if let Some(p) = parent.data() {
                        if let Some(handler) =
                            self.model.download_manifest_handlers.get(&parent.node_id())
                        {
                            let parent_chbox = p.clone();
                            parent_chbox.block_signal(handler);
                            parent_chbox.clone().set_active(all_checked);
                            parent_chbox.unblock_signal(handler);
                        }
                    }
                }
                self.check_parents(parent.node_id());
            }
        }
    }

    fn change_children_state(
        &self,
        parent: NodeRef<Option<CheckButton>>,
        new_state: bool,
        filechanges: &mut Vec<(String, String, String, u128)>,
    ) {
        for child in parent.children() {
            if let Some(child_chbox) = child.data() {
                if let Some(handler) = self.model.download_manifest_handlers.get(&child.node_id()) {
                    child_chbox.block_signal(handler);
                    child_chbox.clone().set_active(new_state);
                    child_chbox.unblock_signal(handler);
                    if child.first_child().is_none() {
                        // dealing with file handle it
                        if let Some((asset_id, app_name, filename, size)) = self
                            .model
                            .download_manifest_file_details
                            .get(&child.node_id())
                        {
                            filechanges.push((
                                asset_id.clone(),
                                app_name.clone(),
                                filename.clone(),
                                size.clone(),
                            ));
                        }
                    }
                }
            }
            self.change_children_state(child, new_state, filechanges);
        }
    }

    fn uncheck_boxes(
        &mut self,
        parent_bool: NodeRef<(bool, Option<NodeId>)>,
        affected: NodeId,
        found: bool,
    ) {
        for child in parent_bool.children() {
            let mut f = found;
            if let Some(nid) = child.data().1 {
                if nid == affected {
                    f = true
                };
                if let Some(ch) = self.model.download_manifest_tree.get(nid) {
                    if let Some(chbox) = ch.data() {
                        let checkbox = chbox.clone();

                        if let Some(handler) = self.model.download_manifest_handlers.get(&nid) {
                            checkbox.block_signal(handler);
                            checkbox.set_active(child.data().0);
                            checkbox.unblock_signal(handler);
                            if f {
                                if ch.first_child().is_none() {
                                    let mut changed_children: Vec<(String, String, String, u128)> =
                                        Vec::new();
                                    if let Some((asset_id, app_name, filename, size)) =
                                        self.model.download_manifest_file_details.get(&nid)
                                    {
                                        changed_children.push((
                                            asset_id.clone(),
                                            app_name.clone(),
                                            filename.clone(),
                                            size.clone(),
                                        ));
                                    }
                                    for (a_id, app_name, filename, size) in changed_children {
                                        self.toggle_file_for_download(
                                            a_id.clone(),
                                            app_name.clone(),
                                            filename.clone(),
                                            Some(false),
                                            size,
                                        );
                                    }
                                }
                            }
                        };
                    }
                }
            }
            self.uncheck_boxes(child, affected, f);
        }
    }
}

fn build_bool_tree(
    parent_master: NodeRef<Option<CheckButton>>,
    mut parent_bool: NodeMut<(bool, Option<NodeId>)>,
    affected_node: NodeId,
) {
    for child in parent_master.children() {
        if child.node_id() == affected_node {
            parent_bool.append((false, Some(child.node_id())));
        } else {
            let new = parent_bool.append((all_children_checked(&child), Some(child.node_id())));
            build_bool_tree(child, new, affected_node);
        };
    }
}

fn all_children_checked(parent_master: &NodeRef<Option<CheckButton>>) -> bool {
    if parent_master.first_child().is_none() {
        if let Some(chbox) = parent_master.data() {
            return chbox.clone().is_active();
        }
        false
    } else {
        let mut all_checked = true;
        for child in parent_master.children() {
            if !all_children_checked(&child) {
                all_checked = false;
            }
        }
        all_checked
    }
}

impl Win {
    pub(crate) fn toggle_file_for_download(
        &mut self,
        asset_id: String,
        a_name: String,
        f_name: String,
        forced_state: Option<bool>,
        size: u128,
    ) -> bool {
        let f_state = forced_state.unwrap_or(true);
        let res = match self.model.selected_files.get_mut(&asset_id) {
            None => {
                if (forced_state.is_some() && f_state) || forced_state.is_none() {
                    self.model.selected_files.insert(
                        asset_id,
                        HashMap::from_iter(
                            [(a_name, vec![f_name])]
                                .iter()
                                .cloned()
                                .collect::<HashMap<String, Vec<String>>>(),
                        ),
                    );
                    self.model.selected_files_size += size;
                    true
                } else {
                    false
                }
            }
            Some(map) => match map.get_mut(&a_name) {
                None => {
                    if (forced_state.is_some() && f_state) || forced_state.is_none() {
                        map.insert(a_name, vec![f_name]);
                        self.model.selected_files_size += size;
                        true
                    } else {
                        false
                    }
                }
                Some(files) => match files.iter().position(|r| r.eq(&f_name)) {
                    None => {
                        if (forced_state.is_some() && f_state) || forced_state.is_none() {
                            files.push(f_name);
                            self.model.selected_files_size += size;
                            true
                        } else {
                            false
                        }
                    }
                    Some(i) => {
                        if (forced_state.is_some() && !f_state) || forced_state.is_none() {
                            files.remove(i);
                            self.model.selected_files_size =
                                match self.model.selected_files_size.checked_sub(size) {
                                    None => 0,
                                    Some(v) => v,
                                };
                            false
                        } else {
                            true
                        }
                    }
                },
            },
        };
        res
    }
}
