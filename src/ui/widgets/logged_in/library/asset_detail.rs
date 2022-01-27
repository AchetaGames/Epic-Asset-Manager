use crate::models::asset_data::AssetType;
use adw::prelude::ActionRowExt;
use diesel::dsl::exists;
use diesel::{select, ExpressionMethods, QueryDsl, RunQueryDsl};
use egs_api::api::types::asset_info::AssetInfo;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::info;

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset_detail.ui")]
    pub struct EpicAssetDetails {
        pub expanded: RefCell<bool>,
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
        pub details_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub actions_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        #[template_child]
        pub favorite: TemplateChild<gtk4::Button>,
        #[template_child]
        pub actions_menu: TemplateChild<gtk4::MenuButton>,
        #[template_child]
        pub images:
            TemplateChild<crate::ui::widgets::logged_in::library::image_stack::EpicImageOverlay>,
        #[template_child]
        pub asset_actions:
            TemplateChild<crate::ui::widgets::logged_in::library::asset_actions::EpicAssetActions>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        pub download_manager: OnceCell<EpicDownloadManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetDetails {
        const NAME: &'static str = "EpicAssetDetails";
        type Type = super::EpicAssetDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
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
                images: TemplateChild::default(),
                asset_actions: TemplateChild::default(),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
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

    impl ObjectImpl for EpicAssetDetails {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecBoolean::new(
                    "expanded",
                    "expanded",
                    "Is expanded",
                    false,
                    glib::ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
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
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
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
            clone!(@weak self as ead => @default-return None, move |_| {
                let self_ = ead.imp();
                get_action!(self_.actions, @show_download_confirmation).activate(None);
                glib::timeout_add_seconds_local(
                    2,
                    clone!(@weak ead as obj => @default-panic, move || {
                        let self_ = obj.imp();
                        get_action!(self_.actions, @show_asset_details).activate(None);
                        glib::Continue(false)
                    }),
                );
                None
            }),
        );
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.set_property("expanded", false);
            })
        );

        action!(
            actions,
            "show_download_details",
            clone!(@weak self as details => move |_, _| {
                let self_ = details.imp();
                self_.details_revealer.set_reveal_child(false);
                self_.details_revealer.set_vexpand_set(true);
                self_.actions_revealer.set_reveal_child(true);
                self_.actions_revealer.set_vexpand(true);
                self_.download_confirmation_revealer.set_reveal_child(false);
                self_.download_confirmation_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(false);
                get_action!(self_.actions, @show_asset_details).set_enabled(true);
                self_.asset_actions.set_action(crate::ui::widgets::logged_in::library::asset_actions::Action::Download);
                self_.actions_menu.popdown()
            })
        );

        action!(
            actions,
            "show_download_confirmation",
            clone!(@weak self as details => move |_, _| {
                let self_ = details.imp();
                self_.download_confirmation_revealer.set_reveal_child(true);
                self_.download_confirmation_revealer.set_vexpand(true);
                self_.details_revealer.set_reveal_child(false);
                self_.details_revealer.set_vexpand_set(true);
                self_.actions_revealer.set_reveal_child(false);
                self_.actions_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(true);
                get_action!(self_.actions, @show_asset_details).set_enabled(true);
            })
        );

        action!(
            actions,
            "show_asset_details",
            clone!(@weak self as details => move |_, _| {
                let self_ = details.imp();
                self_.details_revealer.set_reveal_child(true);
                self_.details_revealer.set_vexpand_set(false);
                self_.actions_revealer.set_reveal_child(false);
                self_.actions_revealer.set_vexpand(false);
                self_.download_confirmation_revealer.set_reveal_child(false);
                self_.download_confirmation_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(true);
                get_action!(self_.actions, @show_asset_details).set_enabled(false);
            })
        );

        action!(
            actions,
            "toggle_favorite",
            clone!(@weak self as details => move |_, _| {
                details.toggle_favorites();
            })
        );
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
            if let Some(kind) = crate::models::asset_data::AssetData::decide_kind(&asset) {
                match kind {
                    AssetType::Asset => {
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Add to Project"),
                                "edit-select-all-symbolic",
                            ))
                            .action_name("details.add_to_project")
                            .build();
                        self_.actions_box.append(&button);
                    }
                    AssetType::Project => {
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Add to Project"),
                                "edit-select-all-symbolic",
                            ))
                            .action_name("details.add_to_project")
                            .build();
                        self_.actions_box.append(&button);
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Create Project"),
                                "folder-new-symbolic",
                            ))
                            .action_name("details.create_project")
                            .build();
                        self_.actions_box.append(&button);
                    }
                    AssetType::Game => {
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Play"),
                                "media-playback-start-symbolic",
                            ))
                            .action_name("details.play_game")
                            .build();
                        self_.actions_box.append(&button);
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Install"),
                                "system-software-install-symbolic",
                            ))
                            .action_name("details.install_game")
                            .build();
                        self_.actions_box.append(&button);
                    }
                    AssetType::Engine => {}
                    AssetType::Plugin => {
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Add to Project"),
                                "edit-select-all-symbolic",
                            ))
                            .action_name("details.add_to_project")
                            .build();
                        self_.actions_box.append(&button);
                        let button = gtk4::Button::builder()
                            .child(&Self::build_box_with_icon_label(
                                Some("Add to Engine"),
                                "application-x-addon-symbolic",
                            ))
                            .action_name("details.add_to_project")
                            .build();
                        self_.actions_box.append(&button);
                    }
                }
            }
        }
        let button = gtk4::Button::builder()
            .child(&Self::build_box_with_icon_label(
                Some("Download"),
                "folder-download-symbolic",
            ))
            .action_name("details.show_download_details")
            .build();
        self_.actions_box.append(&button);
    }

    pub fn set_asset(&self, asset: &egs_api::api::types::asset_info::AssetInfo) {
        let self_ = self.imp();
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
                .set_markup(&format!("<b><u><big>{}</big></u></b>", title));
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
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Developer").build();

            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(dev_name)
                .wrap(true)
                .xalign(0.0)
                .build();

            row.add_suffix(&label);
            self_.details_box.append(&row);
        }

        if let Some(categories) = &asset.categories {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Categories").build();

            row.add_prefix(&title);

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
            let label = gtk4::Label::builder()
                .label(&cats.join(", "))
                .wrap(true)
                .xalign(0.0)
                .build();

            row.add_suffix(&label);
            self_.details_box.append(&row);
        }

        if let Some(platforms) = &asset.platforms() {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Platforms").build();

            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(&platforms.join(", "))
                .wrap(true)
                .xalign(0.0)
                .build();

            row.add_suffix(&label);
            self_.details_box.append(&row);
        }

        if let Some(updated) = &asset.last_modified_date {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Updated").build();

            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(&updated.to_rfc3339())
                .wrap(true)
                .xalign(0.0)
                .build();

            row.add_suffix(&label);
            self_.details_box.append(&row);
        }

        if let Some(compatible_apps) = &asset.compatible_apps() {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Compatible with").build();

            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(&compatible_apps.join(", ").replace("UE_", ""))
                .wrap(true)
                .xalign(0.0)
                .build();

            row.add_suffix(&label);
            self_.details_box.append(&row);
        }

        if let Some(desc) = &asset.long_description {
            let label = gtk4::Label::builder().wrap(true).xalign(0.0).build();
            label.set_markup(&html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n"));
            self_.details_box.append(&label);
        }

        if let Some(desc) = &asset.technical_details {
            let label = gtk4::Label::builder().wrap(true).xalign(0.0).build();
            label.set_markup(&html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n"));
            self_.details_box.append(&label);
        }

        if !self.is_expanded() {
            self.set_property("expanded", true);
        }

        self.check_favorite();
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
            if let Ok(conn) = db.get() {
                if let Some(fav) = self_.favorite.icon_name() {
                    if fav.eq("starred") {
                        diesel::delete(
                            crate::schema::favorite_asset::table
                                .filter(crate::schema::favorite_asset::asset.eq(asset.id.clone())),
                        )
                        .execute(&conn)
                        .expect("Unable to delete favorite from DB");
                        self_.favorite.set_icon_name("non-starred-symbolic");
                    } else {
                        diesel::insert_or_ignore_into(crate::schema::favorite_asset::table)
                            .values(crate::schema::favorite_asset::asset.eq(asset.id.clone()))
                            .execute(&conn)
                            .expect("Unable to insert favorite to the DB");
                        self_.favorite.set_icon_name("starred");
                    };
                    match self_.window.get() {
                        None => {}
                        Some(w) => {
                            let w_ = w.imp();
                            let l = w_.logged_in_stack.clone();
                            let l_ = l.imp();
                            l_.library.refresh_asset(&asset.id);
                        }
                    }
                };
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
        if let Ok(conn) = db.get() {
            match self.asset() {
                None => {}
                Some(asset) => {
                    let ex: Result<bool, diesel::result::Error> = select(exists(
                        crate::schema::favorite_asset::table
                            .filter(crate::schema::favorite_asset::asset.eq(asset.id)),
                    ))
                    .get_result(&conn);
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
        }
        self_.favorite.set_icon_name("non-starred-symbolic");
    }
}
