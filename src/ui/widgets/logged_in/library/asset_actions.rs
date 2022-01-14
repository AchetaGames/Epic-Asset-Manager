use crate::models::asset_data::AssetType;
use crate::tools::or::Or;
use adw::prelude::ExpanderRowExt;
use egs_api::api::types::asset_info::AssetInfo;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;

pub enum Action {
    Download,
    AddToProject,
    AddToEngine,
    CreateProject,
    Install,
    Play,
}

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset_actions.ui")]
    pub struct EpicAssetActions {
        supported_versions: RefCell<Option<String>>,
        selected_version: RefCell<Option<String>>,
        platforms: RefCell<Option<String>>,
        release_date: RefCell<Option<String>>,
        release_notes: RefCell<Option<String>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        #[template_child]
        pub version_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_download_version: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub supported_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub platforms_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub release_date_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub release_notes_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub asset_details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub download_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub project_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub new_project_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub engine_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub install_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub asset_actions_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub download_details: TemplateChild<
            crate::ui::widgets::logged_in::library::download_detail::EpicDownloadDetails,
        >,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetActions {
        const NAME: &'static str = "EpicAssetActions";
        type Type = super::EpicAssetActions;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                supported_versions: RefCell::new(None),
                selected_version: RefCell::new(None),
                platforms: RefCell::new(None),
                release_date: RefCell::new(None),
                release_notes: RefCell::new(None),
                asset: RefCell::new(None),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                version_row: TemplateChild::default(),
                select_download_version: TemplateChild::default(),
                supported_row: TemplateChild::default(),
                platforms_row: TemplateChild::default(),
                release_date_row: TemplateChild::default(),
                release_notes_row: TemplateChild::default(),
                asset_details_revealer: TemplateChild::default(),
                download_row: TemplateChild::default(),
                project_row: TemplateChild::default(),
                new_project_row: TemplateChild::default(),
                engine_row: TemplateChild::default(),
                install_row: TemplateChild::default(),
                asset_actions_button: TemplateChild::default(),
                download_details: TemplateChild::default(),
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

    impl ObjectImpl for EpicAssetActions {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
            obj.setup_events();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "start-download",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "selected-version",
                        "selected_version",
                        "selected_version",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "supported-versions",
                        "supported_versions",
                        "supported_versions",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "platforms",
                        "platforms",
                        "platforms",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "release-date",
                        "release_date",
                        "release_date",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "release-notes",
                        "release_notes",
                        "release_notes",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "supported-versions" => {
                    let supported_versions = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.supported_versions.replace(supported_versions);
                }
                "selected-version" => {
                    let selected_version = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.selected_version.replace(selected_version);
                }
                "platforms" => {
                    let platforms = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.platforms.replace(platforms);
                }
                "release-date" => {
                    let release_date = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.release_date.replace(release_date);
                }
                "release-notes" => {
                    let release_notes = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.release_notes.replace(release_notes);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "supported-versions" => self.supported_versions.borrow().to_value(),
                "selected-version" => self.selected_version.borrow().to_value(),
                "platforms" => self.platforms.borrow().to_value(),
                "release-date" => self.release_date.borrow().to_value(),
                "release-notes" => self.release_notes.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicAssetActions {}
    impl BoxImpl for EpicAssetActions {}
}

glib::wrapper! {
    pub struct EpicAssetActions(ObjectSubclass<imp::EpicAssetActions>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicAssetActions {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAssetActions {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create Epic Asset Actions")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    pub fn set_download_manager(
        &self,
        dm: &crate::ui::widgets::download_manager::EpicDownloadManager,
    ) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }

        self_.download_details.set_download_manager(&dm.clone());
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_events(&self) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);

        self_.select_download_version.connect_changed(
            clone!(@weak self as download_details => move |_| {
                download_details.version_selected();
            }),
        );

        self_
            .download_details
            .connect_local(
                "start-download",
                false,
                clone!(@weak self as ead => @default-return None, move |_| {
                    ead.emit_by_name("start-download", &[]).unwrap();
                    None
                }),
            )
            .unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("asset_actions", Some(actions));

        action!(
            actions,
            "show",
            clone!(@weak self as download_details => move |_, _| {
                let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(&download_details);
                if self_.asset_details_revealer.reveals_child() {
                    self_.asset_details_revealer.set_reveal_child(false);
                    self_.asset_actions_button.set_icon_name("go-down-symbolic");
                    self_.asset_actions_button.set_tooltip_text(Some("Show details"));
                } else {
                    self_.asset_details_revealer.set_reveal_child(true);
                    self_.asset_actions_button.set_icon_name("go-up-symbolic");
                    self_.asset_actions_button.set_tooltip_text(Some("Hide details"));
                }
            })
        );
    }

    pub fn set_action(&self, action: Action) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        match action {
            Action::Download => {
                self_.download_row.set_expanded(true);
            }
            Action::AddToProject => {
                self_.project_row.set_expanded(true);
            }
            Action::AddToEngine => {
                self_.engine_row.set_expanded(true);
            }
            Action::CreateProject => {
                self_.new_project_row.set_expanded(true);
            }
            Action::Install => {
                self_.install_row.set_expanded(true);
            }
            Action::Play => {}
        }
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        if let Some(a) = &*self_.asset.borrow() {
            if asset.id.eq(&a.id) {
                return;
            }
        };

        self_.asset.replace(Some(asset.clone()));
        self_.download_details.set_asset(&asset.clone());
        self_.select_download_version.remove_all();
        if let Some(releases) = asset.sorted_releases() {
            for (id, release) in releases.iter().enumerate() {
                self_.select_download_version.append(
                    Some(release.id.as_ref().unwrap_or(&"".to_string())),
                    &format!(
                        "{}{}",
                        release
                            .version_title
                            .as_ref()
                            .unwrap_or(&"".to_string())
                            .or(release.app_id.as_ref().unwrap_or(&"".to_string())),
                        if id == 0 { " (latest)" } else { "" }
                    ),
                );
            }
            self_.select_download_version.set_active(Some(0));
            if releases.len() <= 1 {
                self_.version_row.set_visible(false);
            } else {
                self_.version_row.set_visible(true);
            }
        }

        if let Some(kind) = crate::models::asset_data::AssetData::decide_kind(&asset) {
            self_.project_row.set_visible(false);
            self_.new_project_row.set_visible(false);
            self_.engine_row.set_visible(false);
            self_.install_row.set_visible(false);
            self_.download_row.set_expanded(false);
            self_.project_row.set_expanded(false);
            self_.new_project_row.set_expanded(false);
            self_.engine_row.set_expanded(false);
            self_.install_row.set_expanded(false);
            match kind {
                AssetType::Asset => {
                    self_.project_row.set_visible(true);
                }
                AssetType::Project => {
                    self_.project_row.set_visible(true);
                    self_.new_project_row.set_visible(true);
                }
                AssetType::Game => {
                    self_.install_row.set_visible(true);
                }
                AssetType::Engine => {}
                AssetType::Plugin => {
                    self_.project_row.set_visible(true);
                    self_.engine_row.set_visible(true);
                }
            };
        }
    }

    pub fn version_selected(&self) {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        if let Some(id) = self_.select_download_version.active_id() {
            self.set_property("selected-version", id.to_string())
                .unwrap();
            self_
                .download_details
                .set_property("selected-version", id.to_string())
                .unwrap();
            if let Some(asset_info) = &*self_.asset.borrow() {
                if let Some(release) = asset_info.release_info(&id.to_string()) {
                    if let Some(ref compatible) = release.compatible_apps {
                        self.set_property(
                            "supported-versions",
                            &compatible.join(", ").replace("UE_", ""),
                        )
                        .unwrap();
                        self_.supported_row.get().set_visible(true);
                    } else {
                        self_.supported_row.get().set_visible(false);
                    }
                    if let Some(ref platforms) = release.platform {
                        self.set_property("platforms", &platforms.join(", "))
                            .unwrap();
                        self_.platforms_row.get().set_visible(true);
                    } else {
                        self_.platforms_row.get().set_visible(false);
                    }
                    if let Some(ref date) = release.date_added {
                        self.set_property(
                            "release-date",
                            &date.naive_local().format("%F").to_string(),
                        )
                        .unwrap();
                        self_.release_date_row.get().set_visible(true);
                    } else {
                        self_.release_date_row.get().set_visible(false);
                    }
                    if let Some(ref note) = release.release_note {
                        if note.is_empty() {
                            self_.release_notes_row.get().set_visible(false);
                        } else {
                            self_.release_notes_row.get().set_visible(true);
                            self.set_property("release-notes", &note).unwrap();
                        }
                    } else {
                        self_.release_notes_row.get().set_visible(false);
                    }
                }
            }
        }
    }

    pub fn asset(&self) -> Option<AssetInfo> {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        self_.asset.borrow().clone()
    }

    pub fn has_asset(&self) -> bool {
        let self_: &imp::EpicAssetActions = imp::EpicAssetActions::from_instance(self);
        self_.asset.borrow().is_some()
    }
}
