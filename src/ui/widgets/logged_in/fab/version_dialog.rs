use adw::prelude::*;
use adw::subclass::prelude::*;
use egs_api::api::types::fab_library::FabAsset;
use gtk4::{self, glib, CompositeTemplate, StringList};
use log::debug;
use std::cell::RefCell;

pub mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/fab_version_dialog.ui")]
    pub struct EpicFabVersionDialog {
        pub fab_asset: RefCell<Option<FabAsset>>,
        pub version_names: RefCell<Vec<String>>,
        pub artifact_ids: RefCell<Vec<String>>,
        pub platform_names: RefCell<Vec<Vec<String>>>,
        #[template_child]
        pub version_dropdown: TemplateChild<gtk4::DropDown>,
        #[template_child]
        pub platform_dropdown: TemplateChild<gtk4::DropDown>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub download_button: TemplateChild<gtk4::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicFabVersionDialog {
        const NAME: &'static str = "EpicFabVersionDialog";
        type Type = super::EpicFabVersionDialog;
        type ParentType = adw::Window;

        fn new() -> Self {
            Self {
                fab_asset: RefCell::new(None),
                version_names: RefCell::new(Vec::new()),
                artifact_ids: RefCell::new(Vec::new()),
                platform_names: RefCell::new(Vec::new()),
                version_dropdown: TemplateChild::default(),
                platform_dropdown: TemplateChild::default(),
                cancel_button: TemplateChild::default(),
                download_button: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicFabVersionDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_events();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("version-selected")
                        .param_types([String::static_type(), String::static_type()])
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EpicFabVersionDialog {}
    impl WindowImpl for EpicFabVersionDialog {}
    impl AdwWindowImpl for EpicFabVersionDialog {}
}

glib::wrapper! {
    pub struct EpicFabVersionDialog(ObjectSubclass<imp::EpicFabVersionDialog>)
        @extends gtk4::Widget, gtk4::Window, adw::Window,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Default for EpicFabVersionDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicFabVersionDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup_events(&self) {
        let self_ = self.imp();

        self_.version_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.update_platforms();
            }
        ));

        self_.cancel_button.connect_clicked(glib::clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.close();
            }
        ));

        self_.download_button.connect_clicked(glib::clone!(
            #[weak(rename_to=dialog)]
            self,
            move |_| {
                dialog.on_download_clicked();
            }
        ));
    }

    pub fn set_fab_asset(&self, fab_asset: &FabAsset) {
        let self_ = self.imp();

        let mut version_names = Vec::new();
        let mut artifact_ids = Vec::new();
        let mut platform_names = Vec::new();

        for pv in &fab_asset.project_versions {
            let name = if pv.engine_versions.is_empty() {
                pv.artifact_id.clone()
            } else {
                pv.engine_versions.join(", ").replace("UE_", "")
            };
            version_names.push(name);
            artifact_ids.push(pv.artifact_id.clone());
            platform_names.push(pv.target_platforms.clone());
        }

        let version_strs: Vec<&str> = version_names.iter().map(|s| s.as_str()).collect();
        let version_model = StringList::new(&version_strs);
        self_.version_dropdown.set_model(Some(&version_model));

        self_.version_names.replace(version_names);
        self_.artifact_ids.replace(artifact_ids);
        self_.platform_names.replace(platform_names);
        self_.fab_asset.replace(Some(fab_asset.clone()));

        self.update_platforms();
    }

    fn update_platforms(&self) {
        let self_ = self.imp();
        let idx = self_.version_dropdown.selected() as usize;
        let platforms = self_.platform_names.borrow();
        if let Some(plats) = platforms.get(idx) {
            let platform_model =
                StringList::new(&plats.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            self_.platform_dropdown.set_model(Some(&platform_model));
        }
    }

    fn on_download_clicked(&self) {
        let self_ = self.imp();
        let version_idx = self_.version_dropdown.selected() as usize;
        let platform_idx = self_.platform_dropdown.selected() as usize;

        let artifact_ids = self_.artifact_ids.borrow();
        let platforms = self_.platform_names.borrow();

        if let Some(artifact_id) = artifact_ids.get(version_idx) {
            if let Some(plats) = platforms.get(version_idx) {
                let platform = plats
                    .get(platform_idx)
                    .cloned()
                    .unwrap_or_else(|| "Windows".to_string());
                debug!(
                    "FAB version selected: artifact_id={}, platform={}",
                    artifact_id, platform
                );
                self.emit_by_name::<()>("version-selected", &[&artifact_id, &platform]);
                self.close();
            }
        }
    }

    pub fn selected_artifact_id(&self) -> Option<String> {
        let self_ = self.imp();
        let idx = self_.version_dropdown.selected() as usize;
        self_.artifact_ids.borrow().get(idx).cloned()
    }

    pub fn selected_platform(&self) -> Option<String> {
        let self_ = self.imp();
        let version_idx = self_.version_dropdown.selected() as usize;
        let platform_idx = self_.platform_dropdown.selected() as usize;
        let platforms = self_.platform_names.borrow();
        platforms
            .get(version_idx)
            .and_then(|p| p.get(platform_idx))
            .cloned()
    }
}
