use crate::models::category_data::CategoryData;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*, Label};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk::gio;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/download_manager.ui")]
    pub struct EpicDownloadManager {
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicDownloadManager {
        const NAME: &'static str = "EpicDownloadManager";
        type Type = super::EpicDownloadManager;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
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

    pub fn add_download(&self) {
        println!("Adding download");
    }
}
