use adw::prelude::ActionRowExt;
use gtk4::cairo::glib::GString;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::info;
use sha1::digest::generic_array::typenum::private::IsEqualPrivate;
use std::ops::Deref;

pub(crate) mod imp {
    use super::*;
    use crate::ui::widgets::download_manager::EpicDownloadManager;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::ParamSpec;
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
        pub download_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub download_confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub details: TemplateChild<gtk4::Box>,
        #[template_child]
        pub details_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        #[template_child]
        pub favorite: TemplateChild<gtk4::Button>,
        #[template_child]
        pub images:
            TemplateChild<crate::ui::widgets::logged_in::library::image_stack::EpicImageOverlay>,
        #[template_child]
        pub download_details: TemplateChild<
            crate::ui::widgets::logged_in::library::download_detail::EpicDownloadDetails,
        >,
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
                download_revealer: TemplateChild::default(),
                download_confirmation_revealer: TemplateChild::default(),
                details: TemplateChild::default(),
                details_box: TemplateChild::default(),
                title: TemplateChild::default(),
                favorite: TemplateChild::default(),
                images: TemplateChild::default(),
                download_details: TemplateChild::default(),
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
                vec![ParamSpec::new_boolean(
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
        let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(self);
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
        let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }

        self_.download_manager.set(dm.clone()).unwrap();
        self_.download_details.set_download_manager(dm);
        self_.images.set_download_manager(dm);

        self_
            .download_details
            .connect_local(
                "start-download",
                false,
                clone!(@weak self as ead => @default-return None, move |_| {
                    let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&ead);
                    get_action!(self_.actions, @show_download_confirmation).activate(None);
                    glib::timeout_add_seconds_local(
                        2,
                        clone!(@weak ead as obj => @default-panic, move || {
                            let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&obj);
                            get_action!(self_.actions, @show_asset_details).activate(None);
                            glib::Continue(false)
                        }),
                    );
                    None
                }),
            )
            .unwrap();
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.set_property("expanded", false).unwrap();
            })
        );

        action!(
            actions,
            "show_download_details",
            clone!(@weak self as details => move |_, _| {
                let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&details);
                self_.details_revealer.set_reveal_child(false);
                self_.details_revealer.set_vexpand_set(true);
                self_.download_revealer.set_reveal_child(true);
                self_.download_revealer.set_vexpand(true);
                self_.download_confirmation_revealer.set_reveal_child(false);
                self_.download_confirmation_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(false);
                get_action!(self_.actions, @show_asset_details).set_enabled(true);
            })
        );

        action!(
            actions,
            "show_download_confirmation",
            clone!(@weak self as details => move |_, _| {
                let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&details);
                self_.download_confirmation_revealer.set_reveal_child(true);
                self_.download_confirmation_revealer.set_vexpand(true);
                self_.details_revealer.set_reveal_child(false);
                self_.details_revealer.set_vexpand_set(true);
                self_.download_revealer.set_reveal_child(false);
                self_.download_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(true);
                get_action!(self_.actions, @show_asset_details).set_enabled(true);
            })
        );

        action!(
            actions,
            "show_asset_details",
            clone!(@weak self as details => move |_, _| {
                let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&details);
                self_.details_revealer.set_reveal_child(true);
                self_.details_revealer.set_vexpand_set(false);
                self_.download_revealer.set_reveal_child(false);
                self_.download_revealer.set_vexpand(false);
                self_.download_confirmation_revealer.set_reveal_child(false);
                self_.download_confirmation_revealer.set_vexpand(false);
                get_action!(self_.actions, @show_download_details).set_enabled(true);
                get_action!(self_.actions, @show_asset_details).set_enabled(false);
            })
        );

        action!(
            actions,
            "toggle_favorite",
            clone!(@weak self as details => move |btn, _| {
                let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(&details);
                if let Some(fav) = self_.favorite.icon_name() {
                    if fav.eq("starred") {
                        self_.favorite.set_icon_name("non-starred-symbolic")
                    } else {
                        self_.favorite.set_icon_name("starred")
                    }
                };
            })
        );
    }

    pub fn set_asset(&self, asset: egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(self);
        if let Some(a) = self_.asset.borrow().deref() {
            if asset.id.eq(&a.id) {
                return;
            }
        };
        self_
            .images
            .set_property("asset", asset.id.clone())
            .unwrap();
        self_.asset.replace(Some(asset.clone()));
        self_.download_details.set_asset(asset.clone());
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand_set(false);
        self_.download_revealer.set_reveal_child(false);
        self_.download_revealer.set_vexpand(false);
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
            self_.details_box.remove(&el)
        }
        let size_group_labels = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
        let size_group_prefix = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);

        if let Some(dev_name) = &asset.developer {
            let row = adw::ActionRowBuilder::new().activatable(true).build();
            let title = gtk4::LabelBuilder::new().label("Developer").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk4::LabelBuilder::new()
                .label(dev_name)
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details_box.append(&row)
        }

        if let Some(platforms) = &asset.platforms() {
            let row = adw::ActionRowBuilder::new().activatable(true).build();
            let title = gtk4::LabelBuilder::new().label("Platforms").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk4::LabelBuilder::new()
                .label(&platforms.join(", "))
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details_box.append(&row)
        }

        if let Some(compatible_apps) = &asset.compatible_apps() {
            let row = adw::ActionRowBuilder::new().activatable(true).build();
            let title = gtk4::LabelBuilder::new().label("Compatible with").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk4::LabelBuilder::new()
                .label(&compatible_apps.join(", ").replace("UE_", ""))
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details_box.append(&row)
        }

        if let Some(desc) = &asset.long_description {
            let label = gtk4::LabelBuilder::new().wrap(true).xalign(0.0).build();
            label.set_markup(&html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n"));
            self_.details_box.append(&label);
        }

        if let Some(desc) = &asset.technical_details {
            let label = gtk4::LabelBuilder::new().wrap(true).xalign(0.0).build();
            label.set_markup(&html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n"));
            self_.details_box.append(&label);
        }

        if !self.is_expanded() {
            self.set_property("expanded", true).unwrap();
        }
    }

    pub fn is_expanded(&self) -> bool {
        if let Ok(value) = self.property("expanded") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }
}
