use egs_api::api::types::fab_search::FabSearchListing;
use gtk4::gdk::Texture;
use gtk4::prelude::ObjectExt;
use gtk4::{glib, subclass::prelude::*};

mod imp {
    use super::*;
    use gtk4::glib::ParamSpecObject;
    use gtk4::prelude::ToValue;
    use std::cell::RefCell;

    #[derive(Debug)]
    pub struct FabSearchData {
        uid: RefCell<Option<String>>,
        title: RefCell<Option<String>>,
        seller_name: RefCell<Option<String>>,
        listing_type: RefCell<Option<String>>,
        category_name: RefCell<Option<String>>,
        is_free: RefCell<bool>,
        thumbnail: RefCell<Option<Texture>>,
        pub listing: RefCell<Option<FabSearchListing>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FabSearchData {
        const NAME: &'static str = "FabSearchData";
        type Type = super::FabSearchData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                uid: RefCell::new(None),
                title: RefCell::new(None),
                seller_name: RefCell::new(None),
                listing_type: RefCell::new(None),
                category_name: RefCell::new(None),
                is_free: RefCell::new(false),
                thumbnail: RefCell::new(None),
                listing: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for FabSearchData {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("refreshed")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("uid").build(),
                    glib::ParamSpecString::builder("title").build(),
                    glib::ParamSpecString::builder("seller-name").build(),
                    glib::ParamSpecString::builder("listing-type").build(),
                    glib::ParamSpecString::builder("category-name").build(),
                    glib::ParamSpecBoolean::builder("is-free").build(),
                    ParamSpecObject::builder::<Texture>("thumbnail").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "uid" => {
                    let uid = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.uid.replace(uid);
                }
                "title" => {
                    let title = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.title.replace(title);
                }
                "seller-name" => {
                    let seller_name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.seller_name.replace(seller_name);
                }
                "listing-type" => {
                    let listing_type = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.listing_type.replace(listing_type);
                }
                "category-name" => {
                    let category_name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.category_name.replace(category_name);
                }
                "is-free" => {
                    let is_free = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.is_free.replace(is_free);
                }
                "thumbnail" => {
                    let thumbnail = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.thumbnail.replace(thumbnail);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "uid" => self.uid.borrow().to_value(),
                "title" => self.title.borrow().to_value(),
                "seller-name" => self.seller_name.borrow().to_value(),
                "listing-type" => self.listing_type.borrow().to_value(),
                "category-name" => self.category_name.borrow().to_value(),
                "is-free" => self.is_free.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct FabSearchData(ObjectSubclass<imp::FabSearchData>);
}

impl FabSearchData {
    pub fn new(listing: &FabSearchListing, image: Option<Texture>) -> FabSearchData {
        let data: Self = glib::Object::new::<Self>();
        let self_ = data.imp();

        data.set_property("uid", &listing.uid);
        data.set_property("title", listing.title.clone().unwrap_or_default());
        data.set_property(
            "seller-name",
            listing
                .user
                .as_ref()
                .and_then(|user| user.seller_name.clone())
                .unwrap_or_default(),
        );
        data.set_property(
            "listing-type",
            listing.listing_type.clone().unwrap_or_default(),
        );
        data.set_property(
            "category-name",
            listing
                .category
                .as_ref()
                .and_then(|cat| cat.name.clone())
                .unwrap_or_default(),
        );
        data.set_property("is-free", listing.is_free.unwrap_or(false));
        self_.listing.replace(Some(listing.clone()));

        if let Some(tex) = image {
            data.set_property("thumbnail", tex);
        };
        data
    }

    pub fn uid(&self) -> String {
        self.property("uid")
    }

    pub fn title(&self) -> String {
        self.property("title")
    }

    pub fn seller_name(&self) -> String {
        self.property("seller-name")
    }

    pub fn listing_type(&self) -> String {
        self.property("listing-type")
    }

    pub fn category_name(&self) -> String {
        self.property("category-name")
    }

    pub fn is_free(&self) -> bool {
        self.property("is-free")
    }

    pub fn image(&self) -> Option<Texture> {
        self.property("thumbnail")
    }

    pub fn listing(&self) -> Option<FabSearchListing> {
        self.imp().listing.borrow().clone()
    }

    pub fn refresh(&self) {
        self.emit_by_name::<()>("refreshed", &[]);
    }
}
