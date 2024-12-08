use crate::models::asset_data::AssetType;
use diesel::dsl::exists;
use diesel::{select, ExpressionMethods, QueryDsl, RunQueryDsl};
use egs_api::api::types::asset_info::AssetInfo;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::{error, info};

pub mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::ui::widgets::logged_in::library::actions;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecUInt};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset_detail.ui")]
    pub struct EpicAssetDetails {
        pub expanded: RefCell<bool>,
        pub downloaded_location: RefCell<Option<std::path::PathBuf>>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        #[template_child]
        pub detail_slider: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub actions_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub download_confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub details: TemplateChild<gtk4::Box>,
        #[template_child]
        pub details_box: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub actions_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        #[template_child]
        pub favorite: TemplateChild<gtk4::Button>,
        #[template_child]
        pub actions_menu: TemplateChild<gtk4::MenuButton>,
        #[template_child]
        pub warning: TemplateChild<gtk4::InfoBar>,
        #[template_child]
        pub warning_message: TemplateChild<gtk4::Label>,
        #[template_child]
        pub images:
            TemplateChild<crate::ui::widgets::logged_in::library::image_stack::EpicImageOverlay>,
        #[template_child]
        pub asset_actions: TemplateChild<actions::EpicAssetActions>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
        pub details_group: gtk4::SizeGroup,
        pub settings: gtk4::gio::Settings,
        position: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetDetails {
        const NAME: &'static str = "EpicAssetDetails";
        type Type = super::EpicAssetDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                downloaded_location: RefCell::new(None),
                asset: RefCell::new(None),
                detail_slider: TemplateChild::default(),
                details_revealer: TemplateChild::default(),
                actions_revealer: TemplateChild::default(),
                download_confirmation_revealer: TemplateChild::default(),
                details: TemplateChild::default(),
                details_box: TemplateChild::default(),
                actions_box: TemplateChild::default(),
                title: TemplateChild::default(),
                favorite: TemplateChild::default(),
                actions_menu: TemplateChild::default(),
                warning: TemplateChild::default(),
                warning_message: TemplateChild::default(),
                images: TemplateChild::default(),
                asset_actions: TemplateChild::default(),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                download_manager: OnceCell::new(),
                details_group: gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal),
                settings: gio::Settings::new(crate::config::APP_ID),
                position: RefCell::new(0),
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

    impl ObjectImpl for EpicAssetDetails {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("expanded").build(),
                    ParamSpecUInt::builder("position")
                        .minimum(0)
                        .default_value(0)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                "position" => {
                    let position = value.get().unwrap();
                    self.position.replace(position);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "position" => self.position.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicAssetDetails {}
    impl BoxImpl for EpicAssetDetails {}
}

glib::wrapper! {
    pub struct EpicAssetDetails(ObjectSubclass<imp::EpicAssetDetails>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicAssetDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAssetDetails {
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
        self_.asset_actions.set_window(&window.clone());
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
        self_.asset_actions.set_download_manager(dm);
        self_.images.set_download_manager(dm);

        self_.asset_actions.connect_local(
            "start-download",
            false,
            clone!(
                #[weak(rename_to=ead)]
                self,
                #[upgrade_or]
                None,
                move |_| {
                    ead.start_download();
                    None
                }
            ),
        );
    }

    fn start_download(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @show_download_confirmation).activate(None);
        glib::timeout_add_seconds_local(
            2,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or_panic]
                move || {
                    obj.hide_confirmation();
                    glib::ControlFlow::Break
                }
            ),
        );
    }

    fn hide_confirmation(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @show_asset_details).activate(None);
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("details", Some(actions));

        action!(
            actions,
            "close",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.collapse();
                }
            )
        );
        action!(
            actions,
            "show_download_details",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_download_details(
                        &crate::ui::widgets::logged_in::library::actions::Action::Download,
                    );
                }
            )
        );

        action!(
            actions,
            "create_project",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_download_details(
                        &crate::ui::widgets::logged_in::library::actions::Action::CreateProject,
                    );
                }
            )
        );

        action!(
            actions,
            "local_assets",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_download_details(
                        &crate::ui::widgets::logged_in::library::actions::Action::Local,
                    );
                }
            )
        );

        action!(
            actions,
            "add_to_project",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_download_details(
                        &crate::ui::widgets::logged_in::library::actions::Action::AddToProject,
                    );
                }
            )
        );

        action!(
            actions,
            "show_download_confirmation",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_download_confirmation();
                }
            )
        );

        action!(
            actions,
            "show_asset_details",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.show_asset_details();
                }
            )
        );

        action!(
            actions,
            "toggle_favorite",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.toggle_favorites();
                }
            )
        );

        self_.warning_message.connect_activate_link(clone!(
            #[weak(rename_to=details)]
            self,
            #[upgrade_or]
            glib::Propagation::Stop,
            move |_, uri| {
                details.process_uri(uri);
                glib::Propagation::Stop
            }
        ));
    }

    fn process_uri(&self, uri: &str) {
        match uri {
            "engines" => {
                let self_ = self.imp();
                if let Some(w) = self_.window.get() {
                    let w_ = w.imp();
                    let l = w_.logged_in_stack.clone();
                    l.switch_tab("engines");
                }
            }
            _ => {
                error!("Unhandled uri clicked: {}", uri);
            }
        }
    }

    fn show_download_details(
        &self,
        action: &crate::ui::widgets::logged_in::library::actions::Action,
    ) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand_set(true);
        self_.actions_revealer.set_reveal_child(true);
        self_.actions_revealer.set_vexpand(true);
        self_.download_confirmation_revealer.set_reveal_child(false);
        self_.download_confirmation_revealer.set_vexpand(false);
        get_action!(self_.actions, @show_asset_details).set_enabled(true);
        self_.asset_actions.set_action(action);
        self_.actions_menu.popdown();
    }

    fn show_download_confirmation(&self) {
        let self_ = self.imp();
        self_.download_confirmation_revealer.set_reveal_child(true);
        self_.download_confirmation_revealer.set_vexpand(true);
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand_set(true);
        self_.actions_revealer.set_reveal_child(false);
        self_.actions_revealer.set_vexpand(false);
        get_action!(self_.actions, @show_download_details).set_enabled(true);
        get_action!(self_.actions, @show_asset_details).set_enabled(true);
    }

    fn show_asset_details(&self) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand_set(false);
        self_.actions_revealer.set_reveal_child(false);
        self_.actions_revealer.set_vexpand(false);
        self_.download_confirmation_revealer.set_reveal_child(false);
        self_.download_confirmation_revealer.set_vexpand(false);
        get_action!(self_.actions, @show_download_details).set_enabled(true);
        get_action!(self_.actions, @show_asset_details).set_enabled(false);
    }

    fn build_box_with_icon_label(label: Option<&str>, icon: &str) -> gtk4::Box {
        let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        b.append(&gtk4::Image::from_icon_name(icon));
        b.append(&gtk4::Label::new(label));
        b
    }

    pub fn set_actions(&self) {
        let self_ = self.imp();
        while let Some(el) = self_.actions_box.first_child() {
            self_.actions_box.remove(&el);
        }

        if let Some(asset) = self.asset() {
            self.create_actions_button(
                "Download",
                "folder-download-symbolic",
                "details.show_download_details",
            );

            self.create_open_vault_button(&asset);

            if let Some(kind) = crate::models::asset_data::AssetData::decide_kind(&asset) {
                match kind {
                    AssetType::Asset => {
                        self_.warning.set_revealed(false);
                        self.create_actions_button(
                            "Add to Project",
                            "folder-new-symbolic",
                            "details.add_to_project",
                        );
                    }
                    AssetType::Project => {
                        self_.warning.set_revealed(false);
                        self.create_actions_button(
                            "Add to Project",
                            "folder-new-symbolic",
                            "details.add_to_project",
                        );
                        self.create_actions_button(
                            "Create Project",
                            "list-add-symbolic",
                            "details.create_project",
                        );
                    }
                    AssetType::Game => {
                        #[cfg(target_os = "linux")]
                        {
                            self_.warning.set_revealed(true);
                            self_.warning_message.set_markup("Games can currently only be downloaded, installing and running them is out of scope of the project right now.");
                        }
                        // self.create_actions_button(
                        //     "Play",
                        //     "media-playback-start-symbolic",
                        //     "details.play_game",
                        // );
                        // self.create_actions_button(
                        //     "Install",
                        //     "system-software-install-symbolic",
                        //     "details.install_game",
                        // );
                    }
                    AssetType::Engine => {
                        #[cfg(target_os = "linux")]
                        {
                            self_.warning.set_revealed(true);
                            self_.warning_message.set_wrap(true);
                            self_.warning_message.set_markup("This is a Windows Build of the Engine. To install Linux version please use the <a href=\"engines\">Engines</a> tab.");
                        }
                    }
                    AssetType::Plugin => {
                        self_.warning.set_revealed(false);
                        // self.create_actions_button(
                        //     "Add to Project",
                        //     "edit-select-all-symbolic",
                        //     "details.add_to_project",
                        // );
                        // self.create_actions_button(
                        //     "Add to Engine",
                        //     "application-x-addon-symbolic",
                        //     "details.add_to_project",
                        // );
                    }
                }
            }
        }
    }

    fn create_open_vault_button(&self, asset: &AssetInfo) {
        let self_ = self.imp();
        if let Some(ris) = &asset.release_info {
            let vaults = self_.settings.strv("unreal-vault-directories");
            for ri in ris {
                if let Some(app) = &ri.app_id {
                    if !crate::models::asset_data::AssetData::downloaded_locations(&vaults, app)
                        .is_empty()
                    {
                        self.create_actions_button(
                            "Local Assets",
                            "folder-open-symbolic",
                            "details.local_assets",
                        );
                        break;
                    }
                }
            }
        }
    }

    fn create_actions_button(&self, label: &str, icon: &str, action_name: &str) {
        let self_ = self.imp();
        let button = gtk4::Button::builder()
            .child(&Self::build_box_with_icon_label(Some(label), icon))
            .action_name(action_name)
            .build();
        button.set_css_classes(&["flat"]);
        self_.actions_box.append(&button);
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();

        if !self.is_expanded() {
            self.set_property("expanded", true);
        }

        if let Some(a) = &*self_.asset.borrow() {
            if asset.id.eq(&a.id) {
                return;
            }
        };

        self_.images.set_property("asset", asset.id.clone());
        self_.asset.replace(Some(asset.clone()));
        self.set_actions();
        self_.asset_actions.set_asset(asset);
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand_set(false);
        self_.actions_revealer.set_reveal_child(false);
        self_.actions_revealer.set_vexpand(false);
        self_.download_confirmation_revealer.set_reveal_child(false);
        self_.download_confirmation_revealer.set_vexpand(false);
        get_action!(self_.actions, @show_download_details).set_enabled(true);
        get_action!(self_.actions, @show_asset_details).set_enabled(false);
        info!("Showing details for {:?}", asset.title);
        if let Some(title) = &asset.title {
            self_
                .title
                .set_markup(&format!("<b><u><big>{title}</big></u></b>"));
        }

        self_.images.clear();

        if let Some(images) = &asset.key_images {
            for image in images {
                if image.width < 300 || image.height < 300 {
                    continue;
                }
                self_.images.add_image(image);
            }
        }

        while let Some(el) = self_.details_box.first_child() {
            self_.details_box.remove(&el);
        }
        if let Some(dev_name) = &asset.developer {
            let text = format!("Developer: {dev_name}");
            self.add_info_row(&text);
        }

        if let Some(categories) = &asset.categories {
            let mut cats: Vec<String> = Vec::new();
            for category in categories {
                let parts = category.path.split('/').collect::<Vec<&str>>();
                if parts.len() > 1 {
                    if cats.is_empty() {
                        cats.push(parts[0].to_string());
                    }
                    cats.push(parts[1..].join("/"));
                }
            }
            if cats.is_empty() {
                for category in categories {
                    cats.push(category.path.clone());
                }
            }
            let cats = &cats.join(", ");
            let text = format!("Categories: {cats}");
            self.add_info_row(&text);
        }

        if let Some(platforms) = &asset.platforms() {
            let platforms = &platforms.join(", ");
            let text = format!("Platforms: {platforms}");
            self.add_info_row(&text);
        }

        if let Some(updated) = &asset.last_modified_date {
            let updated = updated.to_rfc3339();
            let text = format!("Updated: {updated}");
            self.add_info_row(&text);
        }

        if let Some(compatible_apps) = &asset.compatible_apps() {
            if !compatible_apps.is_empty() {
                let compat = &compatible_apps.join(", ").replace("UE_", "");
                let text = format!("Compatible with: {compat}");
                self.add_info_row(&text);
            }
        }

        if let Some(desc) = &asset.long_description {
            let text = &html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
            self.add_info_row(text);
        }

        if let Some(desc) = &asset.technical_details {
            let text = &html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n");
            self.add_info_row(text);
        }
        self.check_favorite();
    }

    fn add_info_row(&self, text: &str) {
        if !&text.is_empty() {
            let self_ = self.imp();
            self_
                .details_box
                .append(&crate::window::EpicAssetManagerWindow::create_info_row(
                    text,
                ));
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.property("expanded")
    }

    pub fn asset(&self) -> Option<AssetInfo> {
        let self_ = self.imp();
        self_.asset.borrow().clone()
    }

    pub fn toggle_favorites(&self) {
        let self_ = self.imp();
        let db = crate::models::database::connection();
        if let Some(asset) = self.asset() {
            if let Ok(mut conn) = db.get() {
                if let Some(fav) = self_.favorite.icon_name() {
                    if fav.eq("starred") {
                        diesel::delete(
                            crate::schema::favorite_asset::table
                                .filter(crate::schema::favorite_asset::asset.eq(asset.id)),
                        )
                        .execute(&mut conn)
                        .expect("Unable to delete favorite from DB");
                        self_.favorite.set_icon_name("non-starred-symbolic");
                    } else {
                        diesel::insert_or_ignore_into(crate::schema::favorite_asset::table)
                            .values(crate::schema::favorite_asset::asset.eq(asset.id))
                            .execute(&mut conn)
                            .expect("Unable to insert favorite to the DB");
                        self_.favorite.set_icon_name("starred");
                    };
                    self.refresh_asset();
                };
            }
        }
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

    pub fn has_asset(&self) -> bool {
        let self_ = self.imp();
        self_.asset.borrow().is_some()
    }

    pub fn check_favorite(&self) {
        let self_ = self.imp();
        let db = crate::models::database::connection();
        if let Ok(mut conn) = db.get() {
            if let Some(asset) = self.asset() {
                let ex: Result<bool, diesel::result::Error> = select(exists(
                    crate::schema::favorite_asset::table
                        .filter(crate::schema::favorite_asset::asset.eq(asset.id)),
                ))
                .get_result(&mut conn);
                if let Ok(fav) = ex {
                    if fav {
                        self_.favorite.set_icon_name("starred");
                    } else {
                        self_.favorite.set_icon_name("non-starred-symbolic");
                    }
                    return;
                }
            }
        }
        self_.favorite.set_icon_name("non-starred-symbolic");
    }

    pub fn position(&self) -> u32 {
        self.property("position")
    }

    pub fn collapse(&self) {
        let self_ = self.imp();
        self.set_property("expanded", false);
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l = w_.logged_in_stack.clone();
            let l_ = l.imp();
            let a = l_.library.imp();
            if let Some(m) = a.asset_grid.model() {
                m.unselect_item(self.position());
            }
        }
    }
}
