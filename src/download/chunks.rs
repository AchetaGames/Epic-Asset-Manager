use crate::download::DownloadedFile;
use crate::Win;
use egs_api::api::types::download_manifest::FileManifestList;
use relm::{Channel, Sender};
use reqwest::Url;
use sha1::{Digest, Sha1};
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
        _app_name: String,
        _filename: String,
    ) {
        unimplemented!()
    }
}

impl Chunks for Win {
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

        let rel = match asset.get_release_name(&release) {
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

                for (filename, manifest) in dm.get_files() {
                    if let Some(file_list) = files {
                        if !file_list.contains(&filename) {
                            return;
                        }
                    }
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

    fn select_file_for_download(&mut self, asset_id: String, app_name: String, filename: String) {
        match self.model.selected_files.get_mut(&asset_id) {
            None => {
                self.model.selected_files.insert(
                    asset_id,
                    HashMap::from_iter(
                        [(app_name, vec![filename])]
                            .iter()
                            .cloned()
                            .collect::<HashMap<String, Vec<String>>>(),
                    ),
                );
            }
            Some(map) => match map.get_mut(&app_name) {
                None => {
                    map.insert(app_name, vec![filename]);
                }
                Some(files) => {
                    match files.iter().position(|r| r.eq(&filename)) {
                        None => {
                            files.push(filename);
                        }
                        Some(i) => {
                            files.remove(i);
                        }
                    };
                }
            },
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
                        Win::perform_download(link, p, g, sender);
                    });
                }
                Some(files) => files.push(full_filename.clone()),
            }
        }
    }
}

impl Win {
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
}
