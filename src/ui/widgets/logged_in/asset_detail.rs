use adw::prelude::ActionRowExt;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, CompositeTemplate};
use log::info;

pub(crate) mod imp {
    use super::*;
    use gtk::glib::ParamSpec;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset_detail.ui")]
    pub struct EpicAssetDetails {
        pub expanded: RefCell<bool>,
        pub asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        #[template_child]
        pub detail_slider: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub details: TemplateChild<gtk::Box>,
        #[template_child]
        pub details_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub images: TemplateChild<crate::ui::widgets::logged_in::image_stack::EpicImageOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAssetDetails {
        const NAME: &'static str = "EpicAssetDetails";
        type Type = super::EpicAssetDetails;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                asset: RefCell::new(None),
                detail_slider: TemplateChild::default(),
                details: TemplateChild::default(),
                details_box: TemplateChild::default(),
                title: TemplateChild::default(),
                images: TemplateChild::default(),
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
        @extends gtk::Widget, gtk::Box;
}

impl EpicAssetDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLoggedInBox")
    }

    pub fn set_asset(&self, asset: egs_api::api::types::asset_info::AssetInfo) {
        let self_: &imp::EpicAssetDetails = imp::EpicAssetDetails::from_instance(self);
        self_.asset.replace(Some(asset.clone()));
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
        let size_group_labels = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        let size_group_prefix = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);

        if let Some(dev_name) = &asset.developer {
            let row = adw::ActionRowBuilder::new().activatable(true).build();
            let title = gtk::LabelBuilder::new().label("Developer").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk::LabelBuilder::new()
                .label(&dev_name)
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details_box.append(&row)
        }

        if let Some(platforms) = &asset.platforms() {
            let row = adw::ActionRowBuilder::new().activatable(true).build();
            let title = gtk::LabelBuilder::new().label("Platforms").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk::LabelBuilder::new()
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
            let title = gtk::LabelBuilder::new().label("Compatible with").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk::LabelBuilder::new()
                .label(&compatible_apps.join(", ").replace("UE_", ""))
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details_box.append(&row)
        }

        if let Some(desc) = &asset.long_description {
            let label = gtk::LabelBuilder::new().wrap(true).xalign(0.0).build();
            label.set_markup(&html2pango::matrix_html_to_markup(desc).replace("\n\n", "\n"));
            self_.details_box.append(&label);
        }

        if let Some(desc) = &asset.technical_details {
            let label = gtk::LabelBuilder::new().wrap(true).xalign(0.0).build();
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
        return false;
    }
}
