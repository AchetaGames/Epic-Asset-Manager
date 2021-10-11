mod download_item;

use crate::tools::asset_info::Search;
use crate::ui::widgets::download_manager::download_item::EpicDownloadItem;
use glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, info, warn};
use rand::Rng;
use reqwest::Url;
use sha1::{Digest, Sha1};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DownloadMsg {
    ProcessItemThumbnail(String, Vec<u8>),
    StartAssetDownload(
        String,
        Vec<egs_api::api::types::download_manifest::DownloadManifest>,
    ),
    PerformAssetDownload(
        String,
        String,
        String,
        egs_api::api::types::download_manifest::FileManifestList,
    ),
    PerformChunkDownload(Url, PathBuf, String),
    RedownloadChunk(Url, PathBuf, String),
    ChunkDownloadProgress(String, u128, bool),
    FinalizeFileDownload(String, DownloadedFile),
    FileAlreadyDownloaded(String, u128),
    FileExtracted(String),
}

#[derive(Default, Debug, Clone)]
pub struct DownloadedFile {
    pub(crate) asset: String,
    pub(crate) release: String,
    pub(crate) name: String,
    pub(crate) chunks: Vec<egs_api::api::types::download_manifest::FileChunkPart>,
    pub(crate) finished_chunks: Vec<egs_api::api::types::download_manifest::FileChunkPart>,
    hash: String,
}

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio;
    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_manager.ui")]
    pub struct EpicDownloadManager {
        pub actions: gio::SimpleActionGroup,
        pub settings: gio::Settings,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_pool: ThreadPool,
        pub thumbnail_pool: ThreadPool,
        pub image_pool: ThreadPool,
        pub file_pool: ThreadPool,
        pub sender: gtk4::glib::Sender<super::DownloadMsg>,
        pub receiver: RefCell<Option<gtk4::glib::Receiver<super::DownloadMsg>>>,
        pub download_items: RefCell<
            HashMap<String, crate::ui::widgets::download_manager::download_item::EpicDownloadItem>,
        >,
        pub downloaded_files: RefCell<HashMap<String, super::DownloadedFile>>,
        pub downloaded_chunks: RefCell<HashMap<String, Vec<String>>>,
        pub chunk_urls: RefCell<HashMap<String, Vec<Url>>>,
        pub asset_files: RefCell<HashMap<String, Vec<String>>>,
        #[template_child]
        pub downloads: TemplateChild<gtk4::Box>,
        has_children: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadManager {
        const NAME: &'static str = "EpicDownloadManager";
        type Type = super::EpicDownloadManager;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = gtk4::glib::MainContext::channel(gtk4::glib::PRIORITY_DEFAULT);
            Self {
                actions: gio::SimpleActionGroup::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                window: OnceCell::new(),
                sender,
                download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
                receiver: RefCell::new(Some(receiver)),
                download_items: RefCell::new(HashMap::new()),
                downloaded_files: RefCell::new(HashMap::new()),
                downloaded_chunks: RefCell::new(HashMap::new()),
                chunk_urls: RefCell::new(HashMap::new()),
                asset_files: RefCell::new(HashMap::new()),
                downloads: TemplateChild::default(),
                thumbnail_pool: ThreadPool::with_name("Thumbnail Pool".to_string(), 5),
                image_pool: ThreadPool::with_name("Image Pool".to_string(), 5),
                file_pool: ThreadPool::with_name("File Pool".to_string(), 1),
                has_children: RefCell::new(false),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicDownloadManager {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.setup_messaging();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "tick",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::new_boolean(
                    "has-items",
                    "has items",
                    "Has Items",
                    false,
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "has-items" => {
                    let has_children = value.get().unwrap();
                    self.has_children.replace(has_children);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "has-items" => self.has_children.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicDownloadManager {}
    impl BoxImpl for EpicDownloadManager {}
}

glib::wrapper! {
    pub struct EpicDownloadManager(ObjectSubclass<imp::EpicDownloadManager>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicDownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicDownloadManager {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicDownloadManager");

        stack
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);

        action!(
            self_.actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(&details);
                if let Some(w) = self_.window.get() {
                   w.show_logged_in()
                }
            })
        );

        self.insert_action_group("download_manager", Some(&self_.actions));
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as download_manager => @default-panic, move |msg| {
                download_manager.update(msg);
                glib::Continue(true)
            }),
        );
    }

    pub fn update(&self, msg: DownloadMsg) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        match msg {
            DownloadMsg::ProcessItemThumbnail(id, image) => {
                let item = match self.get_item(id) {
                    None => return,
                    Some(i) => i,
                };
                let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&image).unwrap();
                pixbuf_loader.close().ok();

                if let Some(pix) = pixbuf_loader.pixbuf() {
                    item.set_property("thumbnail", &pix).unwrap();
                };
            }
            DownloadMsg::StartAssetDownload(id, manifest) => {
                self.start_download_asset(id, manifest);
            }
            DownloadMsg::PerformAssetDownload(id, release, name, manifest) => {
                self.download_asset_file(id, release, name, manifest);
            }
            DownloadMsg::PerformChunkDownload(link, path, guid) => {
                self.download_chunk(link, path, guid);
            }
            DownloadMsg::RedownloadChunk(link, path, guid) => {
                self.redownload_chunk(link, path, guid);
            }
            DownloadMsg::ChunkDownloadProgress(guid, size, finished) => {
                self.chunk_progress_report(guid, size, finished);
            }
            DownloadMsg::FinalizeFileDownload(file, file_details) => {
                self.finalize_file_download(file, file_details);
            }
            DownloadMsg::FileAlreadyDownloaded(id, progress) => {
                let item = match self.get_item(id.clone()) {
                    None => {
                        return;
                    }
                    Some(i) => i,
                };
                item.add_downloaded_size(progress);
                self.emit_by_name("tick", &[]).unwrap();
                self_.sender.send(DownloadMsg::FileExtracted(id)).unwrap();
            }
            DownloadMsg::FileExtracted(id) => {
                let item = match self.get_item(id) {
                    None => {
                        return;
                    }
                    Some(i) => i,
                };
                item.file_processed();
                self.emit_by_name("tick", &[]).unwrap();
            }
        }
    }

    fn get_item(&self, id: String) -> Option<EpicDownloadItem> {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let mut items = self_.download_items.borrow_mut();
        items.get_mut(&id).map(|i| i.clone())
    }

    pub fn load_thumbnail(
        &self,
        id: String,
        thumbnail: Option<egs_api::api::types::asset_info::KeyImage>,
    ) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        match thumbnail {
            None => {}
            Some(t) => {
                let cache_dir = self_.settings.string("cache-directory").to_string();
                let mut cache_path = std::path::PathBuf::from(cache_dir);
                let sender = self_.sender.clone();
                cache_path.push("images");
                let name = Path::new(t.url.path()).extension().and_then(OsStr::to_str);
                cache_path.push(format!("{}.{}", t.md5, name.unwrap_or(".png")));
                self_.thumbnail_pool.execute(move || {
                    if let Ok(w) = crate::RUNNING.read() {
                        if !*w {
                            return;
                        }
                    }
                    match File::open(cache_path.as_path()) {
                        Ok(mut f) => {
                            let metadata = std::fs::metadata(&cache_path.as_path())
                                .expect("unable to read metadata");
                            let mut buffer = vec![0; metadata.len() as usize];
                            f.read_exact(&mut buffer).expect("buffer overflow");
                            let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
                            pixbuf_loader.write(&buffer).unwrap();
                            pixbuf_loader.close().ok();
                            match pixbuf_loader.pixbuf() {
                                None => {}
                                Some(pb) => {
                                    let width = pb.width();
                                    let height = pb.height();

                                    let width_percent = 64.0 / width as f64;
                                    let height_percent = 64.0 / height as f64;
                                    let percent = if height_percent < width_percent {
                                        height_percent
                                    } else {
                                        width_percent
                                    };
                                    let desired = (width as f64 * percent, height as f64 * percent);
                                    sender
                                        .send(DownloadMsg::ProcessItemThumbnail(
                                            id.clone(),
                                            pb.scale_simple(
                                                desired.0.round() as i32,
                                                desired.1.round() as i32,
                                                gtk4::gdk_pixbuf::InterpType::Bilinear,
                                            )
                                            .unwrap()
                                            .save_to_bufferv("png", &[])
                                            .unwrap(),
                                        ))
                                        .unwrap()
                                }
                            };
                        }
                        Err(_) => {
                            warn!("Need to load image");
                        }
                    };
                })
            }
        }
    }

    /// Add an asset for download
    /// This is the first step in the process
    pub fn add_asset_download(
        &self,
        release_id: String,
        asset: egs_api::api::types::asset_info::AssetInfo,
    ) {
        debug!("Adding download: {:?}", asset.title);

        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let item = crate::ui::widgets::download_manager::download_item::EpicDownloadItem::new();
        let mut items = self_.download_items.borrow_mut();
        match items.get_mut(&release_id) {
            None => {
                debug!("Adding item to the list under: {}", release_id);
                items.insert(release_id.clone(), item.clone());
            }
            Some(_) => {
                return;
            }
        };
        item.set_property("label", asset.title.clone()).unwrap();
        item.set_property("status", "initializing...".to_string())
            .unwrap();
        self.load_thumbnail(release_id.clone(), asset.thumbnail());

        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some())
            .unwrap();

        item.connect_local(
            "finished",
            false,
            clone!(@weak self as edm, @weak item => @default-return None, move |_| {
                let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(&edm);
                self_.downloads.remove(&item);
                edm.set_property("has-items", self_.downloads.first_child().is_some()).unwrap();
                edm.emit_by_name("tick", &[]).unwrap();
                None
            }),
        ).unwrap();

        if let Some(window) = self_.window.get() {
            let win_: &crate::window::imp::EpicAssetManagerWindow = window.data();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = self_.sender.clone();
            let id = release_id.clone();
            self_.download_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                }
                let start = std::time::Instant::now();
                if let Some(release_info) = asset.release_info(&release_id) {
                    if let Some(manifest) = Runtime::new().unwrap().block_on(eg.asset_manifest(
                        None,
                        None,
                        Some(asset.namespace),
                        Some(asset.id),
                        Some(release_info.app_id.unwrap_or_default()),
                    )) {
                        debug!("Got asset manifest");
                        let d = Runtime::new()
                            .unwrap()
                            .block_on(eg.asset_download_manifests(manifest));
                        debug!("Got asset download manifests");
                        sender.send(DownloadMsg::StartAssetDownload(id, d)).unwrap();
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
        id: String,
        dm: Vec<egs_api::api::types::download_manifest::DownloadManifest>,
    ) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let item = match self.get_item(id.clone()) {
            None => return,
            Some(i) => i,
        };
        if dm.is_empty() {
            item.set_property("status", "Failed to get download manifests".to_string())
                .unwrap();
            return;
        }
        let vault_dir = match self_.settings.strv("unreal-vault-directories").get(0) {
            None => {
                return;
            }
            Some(s) => s.to_string(),
        };
        let mut target = std::path::PathBuf::from(vault_dir);
        target.push(dm[0].app_name_string.clone());
        let t = target.clone();
        let manifest = dm[0].clone();
        // Create target directory in the vault
        self_.download_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            }
            std::fs::create_dir_all(t.clone()).expect("Unable to create target directory");
            match File::create(t.as_path().join("manifest.json")) {
                Ok(mut json_manifest_file) => {
                    json_manifest_file
                        .write_all(json5::to_string(&manifest).unwrap().as_bytes().as_ref())
                        .unwrap();
                }
                Err(e) => {
                    error!("Unable to save Manifest: {:?}", e)
                }
            }
            match File::create(t.as_path().join("manifest")) {
                Ok(mut manifest_file) => {
                    manifest_file.write_all(&manifest.to_vec()).unwrap();
                }
                Err(e) => {
                    error!("Unable to save binary Manifest: {:?}", e)
                }
            }
        });
        item.set_property("status", "waiting for download slot".to_string())
            .unwrap();
        item.set_total_size(dm[0].total_download_size());
        item.set_total_files(dm[0].file_manifest_list.len() as u64);
        item.set_property("path", target.as_path().display().to_string())
            .unwrap();

        // consolidate manifests

        for manifest in &dm {
            for m in manifest.files().values() {
                for chunk in m.file_chunk_parts.clone() {
                    match chunk.link {
                        None => {}
                        Some(url) => {
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
        }

        for (filename, manifest) in dm[0].files() {
            info!("Starting download of {}", filename);
            let r_id = id.clone();
            let r_name = dm[0].app_name_string.clone();
            let f_name = filename.clone();
            let sender = self_.sender.clone();

            let m = manifest.clone();
            let full_path = target.clone().as_path().join("data").join(filename);
            self_.download_pool.execute(move || {
                if let Ok(w) = crate::RUNNING.read() {
                    if !*w {
                        return;
                    }
                };
                match File::open(full_path.clone()) {
                    Ok(mut f) => {
                        let mut buffer: [u8; 1024] = [0; 1024];
                        let mut hasher = Sha1::new();
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
                        if !m.file_hash.eq(&hash
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<String>())
                        {
                            warn!("Hashes do not match, downloading again: {:?}", full_path);
                            sender
                                .send(DownloadMsg::PerformAssetDownload(r_id, r_name, f_name, m))
                                .unwrap();
                        } else {
                            sender
                                .send(DownloadMsg::FileAlreadyDownloaded(r_id, m.size()))
                                .unwrap();
                        };
                    }
                    // File does not exist perform download
                    Err(_) => {
                        sender
                            .send(DownloadMsg::PerformAssetDownload(r_id, r_name, f_name, m))
                            .unwrap();
                    }
                }
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
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let _item = match self.get_item(id.clone()) {
            None => return,
            Some(i) => i,
        };
        let mut target = std::path::PathBuf::from(
            self_
                .settings
                .string("temporary-download-directory")
                .to_string(),
        );
        target.push(release.clone());
        target.push("temp");
        let full_filename = format!("{}/{}/{}", id, release, filename);
        self_.downloaded_files.borrow_mut().insert(
            full_filename.clone(),
            DownloadedFile {
                asset: id,
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
            let mut chunks = self_.downloaded_chunks.borrow_mut();
            match chunks.get_mut(&chunk.guid) {
                None => {
                    chunks.insert(chunk.guid.clone(), vec![full_filename.clone()]);
                    let mut p = target.clone();
                    let g = chunk.guid.clone();
                    p.push(format!("{}.chunk", g));
                    sender
                        .send(DownloadMsg::RedownloadChunk(
                            Url::parse("unix:/").unwrap(),
                            p,
                            g,
                        ))
                        .unwrap();
                }
                Some(files) => files.push(full_filename.clone()),
            }
        }
    }

    fn redownload_chunk(&self, link: Url, p: PathBuf, g: String) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let sender = self_.sender.clone();
        let mut chunks = self_.chunk_urls.borrow_mut();
        match chunks.get_mut(&g) {
            None => {
                // Unable to get chunk urls
                sender
                    .send(DownloadMsg::PerformChunkDownload(
                        link.clone(),
                        p,
                        g.clone(),
                    ))
                    .unwrap();
            }
            Some(v) => {
                v.retain(|x| !x.eq(&link));
                if v.is_empty() {
                    // No other URL available, redownloading
                    //TODO: This has the potential to loop forever
                    sender
                        .send(DownloadMsg::PerformChunkDownload(
                            link.clone(),
                            p.clone(),
                            g.clone(),
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
                            .send(DownloadMsg::PerformChunkDownload(
                                link.clone(),
                                p,
                                g.clone(),
                            ))
                            .unwrap();
                    }
                    Some(u) => {
                        // Using new url to redownload the chunk
                        sender
                            .send(DownloadMsg::PerformChunkDownload(u.clone(), p, g.clone()))
                            .unwrap();
                    }
                }
            }
        }
    }

    /// Download Chunks
    fn download_chunk(&self, link: Url, p: PathBuf, g: String) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        if !link.has_host() {
            return;
        }
        let sender = self_.sender.clone();
        self_.download_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            };
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
                        .send(DownloadMsg::RedownloadChunk(
                            link.clone(),
                            p.clone(),
                            g.clone(),
                        ))
                        .unwrap();
                    return;
                }
            };
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut downloaded: u128 = 0;
            let mut file = File::create(p).unwrap();
            loop {
                match client.read(&mut buffer) {
                    Ok(size) => {
                        if size > 0 {
                            downloaded += size as u128;
                            file.write_all(&buffer[0..size]).unwrap();
                            sender
                                .send(DownloadMsg::ChunkDownloadProgress(
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
                .send(DownloadMsg::ChunkDownloadProgress(
                    g.clone(),
                    downloaded,
                    true,
                ))
                .unwrap();
        });
    }

    fn chunk_progress_report(&self, guid: String, progress: u128, finished: bool) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        if finished {
            debug!("Finished downloading {}", guid);
            let mut finished_files: Vec<String> = Vec::new();
            let chunks = self_.downloaded_chunks.borrow();
            self_.chunk_urls.borrow_mut().remove(&guid);
            if let Some(files) = chunks.get(&guid) {
                let vault_dir = PathBuf::from(
                    match self_.settings.strv("unreal-vault-directories").get(0) {
                        None => {
                            return;
                        }
                        Some(s) => s.to_string(),
                    },
                );

                let temp_dir = PathBuf::from(
                    self_
                        .settings
                        .string("temporary-download-directory")
                        .to_string(),
                );
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
                            debug!("File finished {}", f.name);
                            finished_files.push(file.clone());
                            let finished = f.clone();
                            let mut temp = temp_dir.clone();
                            let mut vault = vault_dir.clone();
                            temp.push(f.release.clone());
                            temp.push("temp");
                            vault.push(f.release.clone());
                            vault.push("data");
                            let sender = self_.sender.clone();
                            let f_c = f.clone();
                            let file_c = file.clone();
                            self_.file_pool.execute(move || {
                                if let Ok(w) = crate::RUNNING.read() {
                                    if !*w {
                                        return;
                                    }
                                };
                                vault.push(finished.name);
                                std::fs::create_dir_all(vault.parent().unwrap()).unwrap();
                                debug!("Created target directory: {:?}", vault.to_str());
                                match File::create(vault.clone()) {
                                    Ok(mut target) => {
                                        let mut hasher = Sha1::new();
                                        for chunk in finished.chunks {
                                            let mut t = temp.clone();
                                            t.push(format!("{}.chunk", chunk.guid));
                                            match File::open(t) {
                                                Ok(mut f) => {
                                                    let metadata = f
                                                        .metadata()
                                                        .expect("Unable to read metadata");
                                                    let mut buffer =
                                                        vec![0_u8; metadata.len() as usize];
                                                    f.read_exact(&mut buffer).expect("Read failed");
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
                                                        .write_all(
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
                                            error!("Failed to validate hash on: {:?}", vault);
                                            // TODO: Try to download this file again
                                        } else {
                                            sender
                                                .send(DownloadMsg::FinalizeFileDownload(
                                                    file_c.clone(),
                                                    f_c.clone(),
                                                ))
                                                .unwrap();
                                        };
                                    }
                                    Err(e) => {
                                        error!("Error opening the target file: {:?}", e)
                                    }
                                }
                            });
                        }
                    }
                }
            }
        } else {
            let chunks = self_.downloaded_chunks.borrow();
            if let Some(files) = chunks.get(&guid) {
                for file in files {
                    if let Some(f) = self_.downloaded_files.borrow_mut().get_mut(file) {
                        let item = match self.get_item(f.asset.clone()) {
                            None => {
                                break;
                            }
                            Some(i) => i,
                        };
                        item.add_downloaded_size(progress);
                        self.emit_by_name("tick", &[]).unwrap();
                        break;
                    }
                }
            }
        }
    }

    fn finalize_file_download(&self, file: String, file_details: DownloadedFile) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        info!("File finished: {}", file);
        self_.downloaded_files.borrow_mut().remove(&file);
        let temp_dir = PathBuf::from(
            self_
                .settings
                .string("temporary-download-directory")
                .to_string(),
        );
        for chunk in file_details.finished_chunks {
            if let Some(ch) = self_.downloaded_chunks.borrow_mut().get_mut(&chunk.guid) {
                ch.retain(|x| !x.eq(&file));
                if ch.is_empty() {
                    let mut p = temp_dir.clone();
                    p.push(&file_details.release);
                    p.push("temp");

                    p.push(format!("{}.chunk", chunk.guid));
                    debug!("Removing chunk {}", p.as_path().to_str().unwrap());
                    if let Err(e) = std::fs::remove_file(p.clone()) {
                        error!("Unable to remove chunk file: {}", e);
                    };
                    if let Err(e) = std::fs::remove_dir(p.parent().unwrap()) {
                        debug!("Unable to remove the temp directory(yet): {}", e)
                    };
                    if let Err(e) = std::fs::remove_dir(p.parent().unwrap().parent().unwrap()) {
                        debug!("Unable to remove the temp directory(yet): {}", e)
                    };
                }
            }
        }
        self_
            .sender
            .send(DownloadMsg::FileExtracted(file_details.asset))
            .unwrap();
    }

    pub fn progress(&self) -> f64 {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let items = self_.download_items.borrow().values().len();
        let mut progress = 0.0;
        for item in self_.download_items.borrow().values() {
            progress += item.progress();
        }
        if items > 0 {
            progress / items as f64
        } else {
            0.0
        }
    }

    pub fn download_thumbnail(
        &self,
        image: egs_api::api::types::asset_info::KeyImage,
        asset: egs_api::api::types::asset_info::AssetInfo,
        sender: gtk4::glib::Sender<crate::ui::messages::Msg>,
    ) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("images");
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        cache_path.push(format!("{}.{}", image.md5, name.unwrap_or(".png")));
        self_.thumbnail_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            }
            if let Ok(response) = reqwest::blocking::get(image.url.clone()) {
                if let Ok(b) = response.bytes() {
                    std::fs::create_dir_all(&cache_path.parent().unwrap()).unwrap();
                    //TODO: Report downloaded size
                    match File::create(cache_path.as_path()) {
                        Ok(mut thumbnail) => {
                            thumbnail.write_all(&b).unwrap();
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                    sender
                        .send(crate::ui::messages::Msg::ProcessAssetInfo(asset))
                        .unwrap();
                }
            };
        })
    }

    pub fn download_image(
        &self,
        image: egs_api::api::types::asset_info::KeyImage,
        asset: String,
        sender: gtk4::glib::Sender<crate::ui::widgets::logged_in::library::image_stack::ImageMsg>,
    ) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("images");
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        cache_path.push(format!("{}.{}", image.md5, name.unwrap_or(".png")));
        let img = image.clone();
        self_.image_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            }
            debug!("Downloading image");
            if let Ok(response) = reqwest::blocking::get(image.url.clone()) {
                if let Ok(b) = response.bytes() {
                    std::fs::create_dir_all(&cache_path.parent().unwrap()).unwrap();
                    //TODO: Report downloaded size
                    match File::create(cache_path.as_path()) {
                        Ok(mut thumbnail) => {
                            thumbnail.write_all(&b).unwrap();
                        }
                        Err(e) => {
                            error!("{:?}", e);
                        }
                    }
                    sender
                        .send(
                            crate::ui::widgets::logged_in::library::image_stack::ImageMsg::LoadImage(
                                asset, img,
                            ),
                        )
                        .unwrap();
                }
            };
        })
    }
}
