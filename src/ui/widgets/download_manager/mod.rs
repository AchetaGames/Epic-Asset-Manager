pub mod asset;
pub mod docker;
mod download_item;
pub mod epic_file;

use crate::ui::widgets::download_manager::asset::Asset;
use crate::ui::widgets::download_manager::docker::Docker;
use crate::ui::widgets::download_manager::download_item::EpicDownloadItem;
use crate::ui::widgets::download_manager::epic_file::EpicFile;
use glib::clone;
use gtk4::gdk::Texture;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, glib, prelude::*, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error, info, warn};
use reqwest::Url;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Msg {
    ProcessItemThumbnail(String, Texture),
    StartAssetDownload(
        String,
        Vec<egs_api::api::types::download_manifest::DownloadManifest>,
    ),
    StartFabAssetDownload(
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
    PauseChunk(Url, PathBuf, String),
    CancelChunk(Url, PathBuf, String),
    ChunkDownloadProgress(String, u128, bool),
    FinalizeFileDownload(String, asset::DownloadedFile),
    FileAlreadyDownloaded(String, u128, String, String),
    FileExtracted(String),
    PerformDockerEngineDownload(String, u64, Vec<(String, u64)>),
    DockerDownloadProgress(String, u64),
    DockerBlobFinished(String, String),
    DockerBlobFailed(String, (String, u64)),
    DockerExtractionFinished(String),
    DockerCanceled(String, (String, u64)),
    DockerPaused(String, (String, u64)),
    EpicDownloadStart(String, String, u64),
    EpicCanceled(String),
    EpicPaused(String),
    EpicFileFinished(String),
    EpicFileExtracted(String),
    EpicFileExtractionProgress(String, u64),
    EpicDownloadProgress(String, u64),
    IOError(String),
}

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Init,
    Downloaded,
    Extracting,
    Extracted,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PostDownloadAction {
    Copy(String, bool),
    NoVault,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ThreadMessages {
    Cancel,
    Pause,
}

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::gio;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use threadpool::ThreadPool;

    #[derive(Debug, Default)]
    pub struct DownloadState {
        pub download_items: HashMap<String, EpicDownloadItem>,
        pub downloaded_files: HashMap<String, super::asset::DownloadedFile>,
        pub downloaded_chunks: HashMap<String, Vec<String>>,
        pub asset_guids: HashMap<String, Vec<String>>,
        pub paused_asset_chunks: HashMap<String, Vec<(Url, PathBuf)>>,
        pub paused_docker_digests: HashMap<String, Vec<(String, u64)>>,
        pub thread_senders: HashMap<String, Vec<std::sync::mpsc::Sender<super::ThreadMessages>>>,
        pub chunk_urls: HashMap<String, Vec<Url>>,
        pub docker_digests: HashMap<String, Vec<(String, super::DownloadStatus)>>,
    }

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
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub state: RefCell<DownloadState>,
        #[template_child]
        pub downloads: TemplateChild<gtk4::ListBox>,
        has_children: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadManager {
        const NAME: &'static str = "EpicDownloadManager";
        type Type = super::EpicDownloadManager;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                actions: gio::SimpleActionGroup::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                window: OnceCell::new(),
                sender,
                download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
                receiver: RefCell::new(Some(receiver)),
                state: RefCell::new(DownloadState::default()),
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
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_messaging();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("tick")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecBoolean::builder("has-items").build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "has-items" => {
                    let has_children = value.get().unwrap();
                    self.has_children.replace(has_children);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
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
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for EpicDownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

fn cached_download(
    url: &reqwest::Url,
    md5: &str,
    extension: &str,
    cache_dir: &str,
) -> Result<PathBuf, ()> {
    let mut cache_path = PathBuf::from(cache_dir);
    cache_path.push("images");
    cache_path.push(format!("{}.{}", md5, extension));
    if !crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(());
    }
    if let Ok(response) = reqwest::blocking::get(url.clone()) {
        if let Ok(b) = response.bytes() {
            if let Some(parent) = cache_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    error!("Unable to create cache directory {:?}: {}", parent, e);
                    return Err(());
                }
            } else {
                error!("Cache path missing parent: {:?}", cache_path);
                return Err(());
            }
            //TODO: Report downloaded size
            match File::create(cache_path.as_path()) {
                Ok(mut thumbnail) => {
                    if let Err(e) = thumbnail.write_all(&b) {
                        error!("Unable to write thumbnail {:?}: {}", cache_path, e);
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
            return Ok(cache_path);
        }
    }
    Err(())
}

impl EpicDownloadManager {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();

        action!(
            self_.actions,
            "close",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    let self_: &imp::EpicDownloadManager =
                        imp::EpicDownloadManager::from_obj(&details);
                    if let Some(w) = self_.window.get() {
                        w.show_logged_in();
                    }
                }
            )
        );

        self.insert_action_group("download_manager", Some(&self_.actions));
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=download_manager)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(response) = receiver.recv().await {
                    download_manager.update(response);
                }
            }
        ));
    }

    pub fn update(&self, msg: Msg) {
        let self_ = self.imp();
        match msg {
            Msg::ProcessItemThumbnail(id, image) => {
                let Some(item) = self.get_item(&id) else {
                    return;
                };
                item.set_property("thumbnail", Some(image));
            }
            Msg::StartAssetDownload(id, manifest) => {
                self.start_download_asset(&id, &manifest);
            }
            Msg::StartFabAssetDownload(id, manifest) => {
                self.start_download_asset(&id, &manifest);
            }
            Msg::PerformAssetDownload(id, release, name, manifest) => {
                self.download_asset_file(id, release, name, manifest);
            }
            Msg::PerformChunkDownload(link, path, guid) => {
                self.download_chunk(link, path, guid);
            }
            Msg::RedownloadChunk(link, path, guid) => {
                self.redownload_chunk(&link, path, &guid);
            }
            Msg::ChunkDownloadProgress(guid, size, finished) => {
                self.chunk_progress_report(&guid, size, finished);
            }
            Msg::FinalizeFileDownload(file, file_details) => {
                self.finalize_file_download(&file, file_details);
            }
            Msg::FileAlreadyDownloaded(id, progress, fullname, filename) => {
                self.file_already_extracted(id, progress, fullname, filename);
            }
            Msg::FileExtracted(id) => {
                let Some(item) = self.get_item(&id) else {
                    return;
                };
                item.file_processed();
                self.emit_by_name::<()>("tick", &[]);
            }
            Msg::PerformDockerEngineDownload(version, size, digests) => {
                self.perform_docker_blob_downloads(&version, size, digests);
            }
            Msg::DockerDownloadProgress(version, progress) => {
                self.docker_download_progress(&version, progress);
            }
            Msg::DockerBlobFinished(version, digest) => {
                debug!("Finished download of {} digest {}", version, digest);
                self.docker_blob_finished(&version, &digest);
            }
            Msg::DockerBlobFailed(version, digest) => {
                self.download_docker_digest(&version, digest);
            }
            Msg::DockerExtractionFinished(version) => {
                self.docker_extraction_finished(&version);
            }
            Msg::IOError(e) => {
                if let Some(w) = self_.window.get() {
                    w.add_notification(
                        "iodownloaderror",
                        &format!("Unable to download file: {e}"),
                        gtk4::MessageType::Error,
                    );
                }
            }
            Msg::PauseChunk(url, path, guid) => {
                self.pause_asset_chunk(url, path, guid);
            }
            Msg::CancelChunk(_url, path, guid) => {
                self.remove_chunk(path, guid);
            }
            Msg::DockerCanceled(version, digest) => {
                self.cancel_docker_digest(&version, digest);
            }
            Msg::DockerPaused(version, digest) => {
                self.pause_docker_digest(version, digest);
            }
            Msg::EpicDownloadStart(version, url, size) => {
                self.perform_file_download(&url, size, &version);
            }
            Msg::EpicCanceled(_) | Msg::EpicPaused(_) => {}
            Msg::EpicDownloadProgress(ver, size) => {
                self.epic_download_progress(&ver, size);
            }
            Msg::EpicFileFinished(version) => self.epic_file_finished(&version),
            Msg::EpicFileExtracted(version) => {
                self.epic_file_extracted(&version);
            }
            Msg::EpicFileExtractionProgress(version, data) => {
                self.epic_file_extraction_progress(&version, data);
            }
        }
    }

    fn get_item(&self, id: &str) -> Option<EpicDownloadItem> {
        let self_ = self.imp();
        let state = self_.state.borrow();
        state.download_items.get(id).cloned()
    }

    fn finish(&self, item: &download_item::EpicDownloadItem) {
        let self_: &imp::EpicDownloadManager = self.imp();
        if let Some(mut child) = self_.downloads.first_child() {
            loop {
                let row = child.clone().downcast::<gtk4::ListBoxRow>().unwrap();
                if let Some(i) = row.first_child() {
                    if i.eq(item) {
                        self_.downloads.remove(&row);
                        break;
                    }
                }
                if let Some(next_child) = row.next_sibling() {
                    child = next_child;
                } else {
                    break;
                }
            }
        }
        self.set_property("has-items", self_.downloads.first_child().is_some());
        self.emit_by_name::<()>("tick", &[]);
    }

    fn unreal_vault_dir(&self, asset: &str) -> Option<String> {
        let self_ = self.imp();
        if let Some(i) = self.get_item(asset) {
            if let Some(target) = i.target() {
                return Some(target);
            }
        };
        self_
            .settings
            .strv("unreal-vault-directories")
            .first()
            .map(std::string::ToString::to_string)
    }

    fn finalize_file_download(&self, file: &str, file_details: asset::DownloadedFile) {
        let self_ = self.imp();
        info!("File finished: {}", file);
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
        let mut empty_chunks = Vec::new();
        {
            let mut state = self_.state.borrow_mut();
            state.downloaded_files.remove(file);
            for chunk in file_details.finished_chunks {
                if let Some(ch) = state.downloaded_chunks.get_mut(&chunk.guid) {
                    ch.retain(|x| !x.eq(file));
                    if ch.is_empty() {
                        empty_chunks.push(chunk.guid);
                    }
                }
            }
        }
        for guid in empty_chunks {
            let mut p = temp_dir.clone();
            p.push(&file_details.release);
            p.push("temp");

            p.push(format!("{guid}.chunk"));
            debug!("Removing chunk {}", p.as_path().to_string_lossy());
            if let Err(e) = std::fs::remove_file(p.clone()) {
                error!("Unable to remove chunk file: {}", e);
            };
            if let Some(parent) = p.parent() {
                if let Err(e) = std::fs::remove_dir(parent) {
                    debug!("Unable to remove the temp directory(yet): {}", e);
                };
                if let Some(grandparent) = parent.parent() {
                    if let Err(e) = std::fs::remove_dir(grandparent) {
                        debug!("Unable to remove the temp directory(yet): {}", e);
                    };
                }
            }
        }
        self_
            .sender
            .send_blocking(Msg::FileExtracted(file_details.asset))
            .ok();
    }

    pub fn progress(&self) -> f32 {
        let self_ = self.imp();
        let state = self_.state.borrow();
        let items = state.download_items.values().len();
        let mut progress = 0.0_f32;
        for item in state.download_items.values() {
            progress += item.progress();
        }
        if items > 0 {
            progress / items as f32
        } else {
            0.0_f32
        }
    }

    pub fn download_thumbnail(
        &self,
        image: egs_api::api::types::asset_info::KeyImage,
        asset: egs_api::api::types::asset_info::AssetInfo,
        sender: async_channel::Sender<crate::ui::messages::Msg>,
    ) {
        let self_ = self.imp();
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        let extension = name.unwrap_or("png").to_string();
        self_.thumbnail_pool.execute(move || {
            if cached_download(
                &image.url,
                &image.md5,
                extension.as_str(),
                cache_dir.as_str(),
            )
            .is_ok()
            {
                let _ = sender.send_blocking(crate::ui::messages::Msg::ProcessAssetInfo(asset));
            };
        });
    }

    pub fn add_thread_sender(&self, key: String, sender: std::sync::mpsc::Sender<ThreadMessages>) {
        let self_ = self.imp();
        self_
            .state
            .borrow_mut()
            .thread_senders
            .entry(key)
            .or_default()
            .push(sender);
    }

    pub fn send_to_thread_sender(&self, key: &str, msg: &ThreadMessages) {
        let self_ = self.imp();
        if let Some(senders) = self_.state.borrow_mut().thread_senders.remove(key) {
            for sender in senders {
                if sender.send(msg.clone()).is_err() {
                    warn!("Unable to send message {:?} to {}", msg, key);
                };
            }
        }
    }

    pub fn download_image(
        &self,
        image: egs_api::api::types::asset_info::KeyImage,
        asset: String,
        sender: async_channel::Sender<crate::ui::widgets::logged_in::library::image_stack::Msg>,
    ) {
        let self_ = self.imp();
        let cache_dir = self_.settings.string("cache-directory").to_string();
        let name = Path::new(image.url.path())
            .extension()
            .and_then(OsStr::to_str);
        let extension = name.unwrap_or("png").to_string();
        let img = image.clone();
        self_.image_pool.execute(move || {
            if crate::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                debug!("Downloading image");
            }
            if cached_download(
                &image.url,
                &image.md5,
                extension.as_str(),
                cache_dir.as_str(),
            )
            .is_ok()
            {
                sender
                    .send_blocking(
                        crate::ui::widgets::logged_in::library::image_stack::Msg::LoadImage(
                            asset, img,
                        ),
                    )
                    .ok();
            };
        });
    }
}
