use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{glib, CompositeTemplate};

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
        self_.asset.replace(Some(asset));
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
