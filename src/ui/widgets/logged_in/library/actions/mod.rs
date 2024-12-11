mod add_to_project;
mod create_asset_project;
mod download_detail;
mod local_asset;
mod manage_local_assets;

use crate::models::asset_data::AssetType;
use crate::tools::or::Or;
use crate::ui::widgets::download_manager::asset::Asset;
use adw::prelude::ExpanderRowExt;
use egs_api::api::types::asset_info::AssetInfo;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*, SizeGroupMode};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;

pub enum Action {
    Local,
    Download,
    AddToProject,
    AddToEngine,
    CreateProject,
    Install,
    Play,
}

pub mod imp {
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
        pub size_label: RefCell<gtk4::Label>,
        pub disk_size_label: RefCell<gtk4::Label>,
        pub actions: gio::SimpleActionGroup,
        pub settings: gtk4::gio::Settings,
        pub download_manager: OnceCell<EpicDownloadManager>,
        #[template_child]
        pub select_download_version: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub project_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub new_project_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub engine_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub install_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub local_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub download_details: TemplateChild<download_detail::EpicDownloadDetails>,
        #[template_child]
        pub additional_details: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub add_to_project: TemplateChild<add_to_project::EpicAddToProject>,
        #[template_child]
        pub create_asset_project: TemplateChild<create_asset_project::EpicCreateAssetProject>,
        #[template_child]
        pub local_assets: TemplateChild<manage_local_assets::EpicLocalAssets>,
        pub details_group: gtk4::SizeGroup,
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
                size_label: RefCell::new(gtk4::Label::default()),
                disk_size_label: RefCell::new(gtk4::Label::default()),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                select_download_version: TemplateChild::default(),
                project_group: TemplateChild::default(),
                new_project_group: TemplateChild::default(),
                engine_group: TemplateChild::default(),
                install_group: TemplateChild::default(),
                local_group: TemplateChild::default(),
                download_details: TemplateChild::default(),
                additional_details: TemplateChild::default(),
                add_to_project: TemplateChild::default(),
                create_asset_project: TemplateChild::default(),
                local_assets: TemplateChild::default(),
                settings: gio::Settings::new(crate::config::APP_ID),
                details_group: gtk4::SizeGroup::new(SizeGroupMode::Both),
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
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_actions();
            obj.setup_events();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder("start-download")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("selected-version").build(),
                    glib::ParamSpecString::builder("supported-versions").build(),
                    glib::ParamSpecString::builder("platforms").build(),
                    glib::ParamSpecString::builder("release-date").build(),
                    glib::ParamSpecString::builder("release-notes").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
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
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.add_to_project.set_window(&window.clone());
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

        self_.download_details.set_download_manager(&dm.clone());
        self_.create_asset_project.set_download_manager(&dm.clone());
        self_.add_to_project.set_download_manager(&dm.clone());
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn setup_events(&self) {
        let self_ = self.imp();

        self_.select_download_version.connect_changed(clone!(
            #[weak(rename_to=download_details)]
            self,
            move |_| {
                download_details.version_selected();
            }
        ));

        self_.download_details.connect_local(
            "start-download",
            false,
            clone!(
                #[weak(rename_to=ead)]
                self,
                #[upgrade_or]
                None,
                move |_| {
                    ead.emit_by_name::<()>("start-download", &[]);
                    None
                }
            ),
        );

        self_.local_assets.connect_local(
            "removed",
            false,
            clone!(
                #[weak(rename_to=aa)]
                self,
                #[upgrade_or]
                None,
                move |_| {
                    let self_ = aa.imp();
                    if self_.local_assets.empty() {
                        self_.local_group.set_visible(false);
                        aa.refresh_asset();
                    }
                    None
                }
            ),
        );
    }

    fn refresh_asset(&self) {
        let self_ = self.imp();
        if let Some(asset) = self.asset() {
            if let Some(w) = self_.window.get() {
                let w_ = w.imp();
                let l = w_.logged_in_stack.clone();
                let l_ = l.imp();
                l_.library.refresh_asset(&asset.id);
            }
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("asset_actions", Some(actions));
    }

    pub fn set_action(&self, action: &Action) {
        let self_ = self.imp();
        match action {
            Action::Download => {}
            Action::AddToProject => {}
            Action::AddToEngine => {}
            Action::CreateProject => {}
            Action::Install => {}
            Action::Play => {}
            Action::Local => {}
        }
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
        if let Some(a) = &*self_.asset.borrow() {
            if asset.id.eq(&a.id) {
                return;
            }
        };

        while let Some(el) = self_.additional_details.first_child() {
            self_.additional_details.remove(&el);
        }
        self_.asset.replace(Some(asset.clone()));
        self_.download_details.set_asset(&asset.clone());
        self_.add_to_project.set_asset(&asset.clone());
        self_.create_asset_project.set_asset(&asset.clone());
        self_.local_assets.set_asset(&asset.clone());
        self_.select_download_version.remove_all();
        self_.local_group.set_visible(false);
        let vaults = self_.settings.strv("unreal-vault-directories");
        if let Some(releases) = asset.sorted_releases() {
            for (id, release) in releases.iter().enumerate() {
                if let Some(app) = &release.app_id {
                    if !crate::models::asset_data::AssetData::downloaded_locations(&vaults, app)
                        .is_empty()
                    {
                        self_.local_group.set_visible(true);
                    }
                }
                self_.select_download_version.append(
                    Some(release.id.as_ref().unwrap_or(&String::new())),
                    &format!(
                        "{}{}",
                        release
                            .version_title
                            .as_ref()
                            .unwrap_or(&String::new())
                            .or(release.app_id.as_ref().unwrap_or(&String::new())),
                        if id == 0 { " (latest)" } else { "" }
                    ),
                );
            }
            self_.select_download_version.set_active(Some(0));
        }

        if let Some(kind) = crate::models::asset_data::AssetData::decide_kind(asset) {
            self_.project_group.set_visible(false);
            self_.new_project_group.set_visible(false);
            self_.engine_group.set_visible(false);
            self_.install_group.set_visible(false);
            match kind {
                AssetType::Asset => {
                    self_.project_group.set_visible(true);
                }
                AssetType::Project => {
                    self_.project_group.set_visible(true);
                    self_.new_project_group.set_visible(true);
                }
                AssetType::Game | AssetType::Engine | AssetType::Plugin => {}
            };
        }

        // TODO: Switch to self.add_detail_exp()
        let size_label = gtk4::Label::new(Some("Loading..."));
        self.add_detail("Download Size:", &size_label);
        self_.size_label.replace(size_label);

        let size_label = gtk4::Label::new(Some("Loading..."));
        self.add_detail("Size:", &size_label);
        self_.disk_size_label.replace(size_label);
    }

    fn process_download_manifest(
        &self,
        release_id: &str,
        manifests: Vec<egs_api::api::types::download_manifest::DownloadManifest>,
    ) {
        let self_ = self.imp();
        if let Some(id) = self_.select_download_version.active_id() {
            if release_id.eq(&id) {
                if let Some(manifest) = manifests.into_iter().next() {
                    self_.add_to_project.set_manifest(&manifest);
                    self_.download_details.set_manifest(&manifest);
                    self_.create_asset_project.set_manifest(&manifest);
                    self_.size_label.borrow().set_label(&format!(
                        "{:.2}",
                        byte_unit::Byte::from_u128(manifest.total_download_size())
                            .unwrap_or_default()
                            .get_appropriate_unit(byte_unit::UnitType::Decimal)
                    ));

                    self_.disk_size_label.borrow().set_label(&format!(
                        "{:.2}",
                        byte_unit::Byte::from_u128(manifest.total_size())
                            .unwrap_or_default()
                            .get_appropriate_unit(byte_unit::UnitType::Decimal)
                    ));
                }
            }
        };
    }

    fn add_detail(&self, label: &str, widget: &impl IsA<gtk4::Widget>) {
        let self_ = self.imp();
        self_
            .additional_details
            .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                label, widget,
            ));
    }

    fn add_detail_exp(&self, text: &str) {
        let self_ = self.imp();
        self_
            .additional_details
            .append(&crate::window::EpicAssetManagerWindow::create_info_row(
                text,
            ));
    }

    pub fn version_selected(&self) {
        let self_ = self.imp();
        while let Some(el) = self_.additional_details.first_child() {
            self_.additional_details.remove(&el);
        }
        if let Some(id) = self_.select_download_version.active_id() {
            self.set_property("selected-version", id.to_string());
            self_
                .download_details
                .set_property("selected-version", id.to_string());
            self_
                .create_asset_project
                .set_property("selected-version", id.to_string());
            self_
                .add_to_project
                .set_property("selected-version", id.to_string());
            if let Some(asset_info) = &*self_.asset.borrow() {
                if let Some(release) = asset_info.release_info(&id) {
                    if let Some(ref compatible) = release.compatible_apps {
                        let text = &compatible.join(", ").replace("UE_", "");
                        let text = format!("Supported versions: {text}");
                        self.add_detail_exp(&text);
                    }
                    if let Some(ref platforms) = release.platform {
                        let text = &platforms.join(", ");
                        let text = format!("Platforms: {text}");
                        self.add_detail_exp(&text);
                    }
                    if let Some(ref date) = release.date_added {
                        let text = &date.naive_local().format("%F").to_string();
                        let text = format!("Release Date: {text}");
                        self.add_detail_exp(&text);
                    }
                    if let Some(ref note) = release.release_note {
                        if !note.is_empty() {
                            let text = &note;
                            let text = format!("Release Note: {text}");
                            self.add_detail_exp(&text);
                        }
                    }
                    let (sender, receiver) = async_channel::unbounded::<(
                        String,
                        Vec<egs_api::api::types::download_manifest::DownloadManifest>,
                    )>();

                    glib::spawn_future_local(clone!(
                        #[weak(rename_to=asset_actions)]
                        self,
                        #[upgrade_or_panic]
                        async move {
                            while let Ok((id, manifest)) = receiver.recv().await {
                                asset_actions.process_download_manifest(&id, manifest);
                            }
                        }
                    ));

                    if let Some(dm) = self_.download_manager.get() {
                        dm.download_asset_manifest(id.to_string(), asset_info.clone(), sender);
                    }
                    if let Some(release) = release.app_id {
                        self_.local_assets.update_local_versions(&release);
                    }
                }
            }
        }
    }

    pub fn asset(&self) -> Option<AssetInfo> {
        let self_ = self.imp();
        self_.asset.borrow().clone()
    }

    pub fn has_asset(&self) -> bool {
        let self_ = self.imp();
        self_.asset.borrow().is_some()
    }
}
