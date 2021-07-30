use crate::models::category_data::CategoryData;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*, Label};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::{debug, error};
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
pub enum DownloadMsg {}

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk::gio;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use threadpool::ThreadPool;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_manager.ui")]
    pub struct EpicDownloadManager {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_pool: ThreadPool,
        pub sender: gtk::glib::Sender<super::DownloadMsg>,
        pub receiver: RefCell<Option<gtk::glib::Receiver<super::DownloadMsg>>>,
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

        self.insert_action_group("download", Some(&self_.actions));
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as download_manager => @default-panic, move |msg| {
                let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(&download_manager);
                glib::Continue(true)
            }),
        );
    }

    pub fn add_asset_download(
        &self,
        release_id: String,
        asset: egs_api::api::types::asset_info::AssetInfo,
    ) {
        debug!("Adding download: {:?}", asset.title);
        let self_: &imp::EpicDownloadManager = imp::EpicDownloadManager::from_instance(self);
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
