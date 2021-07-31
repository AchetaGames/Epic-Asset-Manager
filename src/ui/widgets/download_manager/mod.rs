mod download_item;

use crate::models::category_data::CategoryData;
use crate::tools::asset_info::Search;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*, Label};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error};
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    ProcessItemThumbnail(String, Vec<u8>),
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
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_pool: ThreadPool,
        pub thumbnail_pool: ThreadPool,
        pub sender: gtk::glib::Sender<super::DownloadMsg>,
        pub receiver: RefCell<Option<gtk::glib::Receiver<super::DownloadMsg>>>,
        pub download_items: RefCell<
            HashMap<String, crate::ui::widgets::download_manager::download_item::EpicDownloadItem>,
        >,
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
                window: OnceCell::new(),
                sender,
                download_pool: ThreadPool::with_name("Download Pool".to_string(), 5),
                receiver: RefCell::new(Some(receiver)),
                download_items: RefCell::new(HashMap::new()),
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
                let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(&download_manager);
                download_manager.update(msg);
                glib::Continue(true)
            }),
        );
    }

    pub fn update(&self, msg: DownloadMsg) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        match msg {
            DownloadMsg::ProcessItemThumbnail(id, image) => {
                let mut items = self_.download_items.borrow_mut();
                match items.get_mut(&id) {
                    None => {}
                    Some(i) => {
                        let pixbuf_loader = gtk::gdk_pixbuf::PixbufLoader::new();
                        pixbuf_loader.write(&image).unwrap();
                        pixbuf_loader.close().ok();

                        if let Some(pix) = pixbuf_loader.pixbuf() {
                            i.set_property("thumbnail", &pix).unwrap();
                        };
                    }
                };
            }
        }
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
                if let Some(w) = self_.window.get() {
                    let win_ = w.data();
                    let cache_dir = win_
                        .model
                        .settings
                        .string("cache-directory")
                        .to_string()
                        .clone();
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
                                        let desired =
                                            (width as f64 * percent, height as f64 * percent);
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
    }

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
        item.set_property("label", asset.title.clone());
        item.set_property("status", "initializing".to_string());
        self.load_thumbnail(release_id.clone(), asset.thumbnail());

        self_.downloads.append(&item);
        if let Some(window) = self_.window.get() {
            let win_: &crate::window::imp::EpicAssetManagerWindow = window.data();
            let mut eg = win_.model.epic_games.clone();
            let (sender, receiver) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);
            receiver.attach(
                None,
                clone!(@weak self as download_manager => @default-panic, move |manifest:egs_api::api::types::download_manifest::DownloadManifest| {
                    let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(&download_manager);
                    println!("Got download manifest {}", manifest.app_name_string);
                    glib::Continue(false)
                }),
            );

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
                        if let Ok(d) = Runtime::new()
                            .unwrap()
                            .block_on(eg.asset_download_manifest(manifest))
                        {
                            sender.send(d).unwrap();
                        };
                    };
                }
                debug!("Download Manifest requests took {:?}", start.elapsed());
            });
        }
    }
}
