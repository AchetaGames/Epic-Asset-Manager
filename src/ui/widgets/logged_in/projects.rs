use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::info;

pub(crate) mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/projects.ui")]
    pub struct EpicProjectsBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub settings: gtk4::gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicProjectsBox {
        const NAME: &'static str = "EpicProjectsBox";
        type Type = super::EpicProjectsBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicProjectsBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.load_projects();
        }
    }

    impl WidgetImpl for EpicProjectsBox {}
    impl BoxImpl for EpicProjectsBox {}
}

glib::wrapper! {
    pub struct EpicProjectsBox(ObjectSubclass<imp::EpicProjectsBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicProjectsBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicProjectsBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn load_projects(&self) {
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        for dir in self_.settings.strv("unreal-projects-directories") {
            info!("Checking directory {}", dir);
            let path = std::path::PathBuf::from(dir.to_string());
            match path.read_dir() {
                Ok(rd) => {
                    for d in rd {
                        match d {
                            Ok(entry) => {
                                let path = entry.path();
                                println!("got path: {:?}", path.into_os_string());
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicProjectsBox = imp::EpicProjectsBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }
}
