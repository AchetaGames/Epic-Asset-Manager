use crate::tools::epic_web::EpicWeb;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/epic_download.ui")]
    pub struct EpicEngineDownload {
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineDownload {
        const NAME: &'static str = "EpicEngineDownload";
        type Type = super::EpicEngineDownload;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
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

    impl ObjectImpl for EpicEngineDownload {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for EpicEngineDownload {}
    impl BoxImpl for EpicEngineDownload {}
}

glib::wrapper! {
    pub struct EpicEngineDownload(ObjectSubclass<imp::EpicEngineDownload>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEngineDownload {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngineDownload {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        //let mut web = EpicWeb::new();
        //web.start_session(code.clone());
        //if web.validate_eula() {
        //    web.run_query("https://www.unrealengine.com/api/blobs/linux".to_string());
        //}
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }
}
