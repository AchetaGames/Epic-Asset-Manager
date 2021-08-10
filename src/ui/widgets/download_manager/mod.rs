mod download_item;

use crate::tools::asset_info::Search;
use crate::ui::widgets::download_manager::download_item::EpicDownloadItem;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, info, warn};
use reqwest::Url;
use sha1::{Digest, Sha1};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    ProcessItemThumbnail(String, Vec<u8>),
    StartAssetDownload(
        String,
        egs_api::api::types::download_manifest::DownloadManifest,
    ),
    PerformAssetDownload(
        String,
        String,
        String,
        egs_api::api::types::download_manifest::FileManifestList,
    ),
    PerformChunkDownload(Url, PathBuf, String),
    ChunkDownloadProgress(String, u128, bool),
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
    use gtk::gio;
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
        pub sender: gtk::glib::Sender<super::DownloadMsg>,
        pub receiver: RefCell<Option<gtk::glib::Receiver<super::DownloadMsg>>>,
        pub download_items: RefCell<
            HashMap<String, crate::ui::widgets::download_manager::download_item::EpicDownloadItem>,
        >,
        pub downloaded_files: RefCell<HashMap<String, super::DownloadedFile>>,
        pub downloaded_chunks: RefCell<HashMap<String, Vec<String>>>,
        #[template_child]
        pub downloads: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadManager {
        const NAME: &'static str = "EpicDownloadManager";
        type Type = super::EpicDownloadManager;
        type ParentType = gtk::Box;

        fn new() -> Self {
            let (sender, receiver) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);
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
                downloads: TemplateChild::default(),
                thumbnail_pool: ThreadPool::with_name("Thumbnail Pool".to_string(), 5),
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
    }

    impl WidgetImpl for EpicDownloadManager {}
    impl BoxImpl for EpicDownloadManager {}
}

glib::wrapper! {
    pub struct EpicDownloadManager(ObjectSubclass<imp::EpicDownloadManager>)
        @extends gtk::Widget, gtk::Box;
}

impl EpicDownloadManager {
    pub fn new() -> Self {
        let stack: Self = glib::Object::new(&[]).expect("Failed to create EpicDownloadManager");

        stack
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        // Do not run this twice
        if let Some(_) = self_.window.get() {
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
        match msg {
            DownloadMsg::ProcessItemThumbnail(id, image) => {
                let item = match self.get_item(id) {
                    None => return,
                    Some(i) => i,
                };
                let pixbuf_loader = gtk::gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&image).unwrap();
                pixbuf_loader.close().ok();

                if let Some(pix) = pixbuf_loader.pixbuf() {
                    item.set_property("thumbnail", &pix).unwrap();
                };
            }
            DownloadMsg::StartAssetDownload(id, manifest) => {
                debug!("Got download manifest {}", manifest.app_name_string);
                self.start_download_asset(id, manifest);
            }
            DownloadMsg::PerformAssetDownload(id, release, name, manifest) => {
                self.download_asset_file(id, release, name, manifest);
            }
            DownloadMsg::PerformChunkDownload(link, path, guid) => {
                self.download_chunk(link, path, guid);
            }
            DownloadMsg::ChunkDownloadProgress(guid, size, finished) => {
                self.chunk_progress_report(guid, size, finished);
            }
        }
    }

    fn get_item(&self, id: String) -> Option<EpicDownloadItem> {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let mut items = self_.download_items.borrow_mut();
        return match items.get_mut(&id) {
            Some(i) => Some(i.clone()),
            None => None,
        };
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
                let cache_dir = self_.settings.string("cache-directory").to_string().clone();
                let mut cache_path = std::path::PathBuf::from(cache_dir);
                let sender = self_.sender.clone();
                cache_path.push("images");
                let name = Path::new(t.url.path()).extension().and_then(OsStr::to_str);
                cache_path.push(format!("{}.{}", t.md5, name.unwrap_or(&".png")));
                self_.thumbnail_pool.execute(move || {
                    match File::open(cache_path.as_path()) {
                        Ok(mut f) => {
                            let metadata = std::fs::metadata(&cache_path.as_path())
                                .expect("unable to read metadata");
                            let mut buffer = vec![0; metadata.len() as usize];
                            f.read(&mut buffer).expect("buffer overflow");
                            let pixbuf_loader = gtk::gdk_pixbuf::PixbufLoader::new();
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
                                                gtk::gdk_pixbuf::InterpType::Bilinear,
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
                            println!("Need to load image");
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
        if let Some(window) = self_.window.get() {
            let win_: &crate::window::imp::EpicAssetManagerWindow = window.data();
            let mut eg = win_.model.epic_games.clone();
            let sender = self_.sender.clone();
            let id = release_id.clone();
            self_.download_pool.execute(move || {
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
                        if let Ok(d) = Runtime::new()
                            .unwrap()
                            .block_on(eg.asset_download_manifest(manifest))
                        {
                            debug!("Got asset download manifest");
                            sender.send(DownloadMsg::StartAssetDownload(id, d)).unwrap();
                            // TODO cache download manifest
                        };
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
        dm: egs_api::api::types::download_manifest::DownloadManifest,
    ) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let item = match self.get_item(id.clone()) {
            None => return,
            Some(i) => i,
        };
        let vault_dir = match self_.settings.strv("unreal-vault-directories").get(0) {
            None => {
                return;
            }
            Some(s) => s.to_string(),
        };
        let mut target = std::path::PathBuf::from(vault_dir);
        target.push(dm.app_name_string.clone());
        let t = target.clone();
        let manifest = dm.clone();
        self_.download_pool.execute(move || {
            std::fs::create_dir_all(t.clone()).expect("Unable to create target directory");
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
        item.set_property("status", "waiting for download slot".to_string())
            .unwrap();
        let item_c = item.clone();

        for (filename, manifest) in dm.files() {
            info!("Starting download of {}", filename);
            let r_id = id.clone();
            let r_name = dm.app_name_string.clone();
            let f_name = filename.clone();
            let sender = self_.sender.clone();

            let m = manifest.clone();
            let full_path = target.clone().as_path().join("data").join(filename);
            self_
                .download_pool
                .execute(move || match File::open(full_path.clone()) {
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
                            sender
                                .send(DownloadMsg::PerformAssetDownload(r_id, r_name, f_name, m))
                                .unwrap();
                        };
                    }
                    Err(_) => {
                        sender
                            .send(DownloadMsg::PerformAssetDownload(r_id, r_name, f_name, m))
                            .unwrap();
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
        let item = match self.get_item(id.clone()) {
            None => return,
            Some(i) => i,
        };
        let vault_dir = match self_.settings.strv("unreal-vault-directories").get(0) {
            None => {
                return;
            }
            Some(s) => s.to_string(),
        };
        let mut target = std::path::PathBuf::from(vault_dir);
        target.push(release.clone());
        target.push("temp");
        let full_filename = format!("{}/{}/{}", id.clone(), release.clone(), filename.clone());
        self_.downloaded_files.borrow_mut().insert(
            full_filename.clone(),
            DownloadedFile {
                asset: id.clone(),
                release,
                name: filename.clone(),
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
                    let link = chunk.link.unwrap();
                    let mut p = target.clone();
                    let g = chunk.guid.clone();
                    p.push(format!("{}.chunk", g));
                    sender
                        .send(DownloadMsg::PerformChunkDownload(link, p, g))
                        .unwrap();
                }
                Some(files) => files.push(full_filename.clone()),
            }
        }
    }

    /// Download Chunks
    fn download_chunk(&self, link: Url, p: PathBuf, g: String) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let sender = self_.sender.clone();
        self_.download_pool.execute(move || {
            debug!(
                "Downloading chunk {} from {} to {:?}",
                g,
                link.to_string(),
                p
            );
            std::fs::create_dir_all(p.parent().unwrap().clone()).unwrap();
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
                    downloaded.clone(),
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
            let mut chunks = self_.downloaded_chunks.borrow_mut();
            if let Some(files) = chunks.get(&guid) {
                let vault_dir = match self_.settings.strv("unreal-vault-directories").get(0) {
                    None => {
                        return;
                    }
                    Some(s) => s.to_string(),
                };
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
                        }
                    }
                }
            }
        } else {
            debug!("Got progress report from {}, current: {}", guid, progress);
        }
    }
}
