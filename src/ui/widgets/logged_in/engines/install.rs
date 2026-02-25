use gtk4::subclass::prelude::*;
use gtk4::{self, gio};
use gtk4::{glib, CompositeTemplate};

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/install.ui")]
    pub struct EpicEngineInstall {
        #[template_child]
        pub epic: TemplateChild<
            crate::ui::widgets::logged_in::engines::epic_download::EpicEngineDownload,
        >,
        #[template_child]
        pub docker: TemplateChild<
            crate::ui::widgets::logged_in::engines::docker_download::DockerEngineDownload,
        >,
        pub actions: gio::SimpleActionGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineInstall {
        const NAME: &'static str = "EpicEngineInstall";
        type Type = super::EpicEngineInstall;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                epic: TemplateChild::default(),
                docker: TemplateChild::default(),
                actions: gio::SimpleActionGroup::new(),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
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

    impl ObjectImpl for EpicEngineInstall {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EpicEngineInstall {}

    impl BoxImpl for EpicEngineInstall {}
}

glib::wrapper! {
    pub struct EpicEngineInstall(ObjectSubclass<imp::EpicEngineInstall>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for EpicEngineInstall {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngineInstall {
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
        self_.epic.set_window(window);
        self_.docker.set_window(window);
        self.update_docker();
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
        self_.epic.set_download_manager(dm);
        self_.docker.set_download_manager(dm);
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn update_docker(&self) {
        let self_ = self.imp();
        self_.docker.update_docker();
    }

    pub fn add_engine(&self) {
        let self_ = self.imp();
        self_.docker.add_engine();
    }
}
