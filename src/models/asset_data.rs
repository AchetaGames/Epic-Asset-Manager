use chrono::{DateTime, Utc};
use diesel::dsl::exists;
use diesel::{select, ExpressionMethods, QueryDsl, RunQueryDsl};
use egs_api::api::types::asset_info::AssetInfo;
use glib::ObjectExt;
use gtk4::gdk_pixbuf::prelude::PixbufLoaderExt;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::gio::prelude::SettingsExt;
use gtk4::{gdk_pixbuf, glib, subclass::prelude::*};

pub enum AssetType {
    Asset,
    Project,
    Game,
    Engine,
    Plugin,
}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::gdk_pixbuf::prelude::StaticType;
    use gtk4::gdk_pixbuf::Pixbuf;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct AssetData {
        id: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        favorite: RefCell<bool>,
        downloaded: RefCell<bool>,
        pub kind: RefCell<Option<String>>,
        pub(crate) asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        thumbnail: RefCell<Option<Pixbuf>>,
        pub settings: gtk4::gio::Settings,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for AssetData {
        const NAME: &'static str = "AssetData";
        type Type = super::AssetData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                name: RefCell::new(None),
                favorite: RefCell::new(false),
                downloaded: RefCell::new(false),
                kind: RefCell::new(None),
                asset: RefCell::new(None),
                thumbnail: RefCell::new(None),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for AssetData {
        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "refreshed",
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
                        "name",
                        "Name",
                        "Name",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "id",
                        "ID",
                        "ID",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_object(
                        "thumbnail",
                        "Thumbnail",
                        "Thumbnail",
                        Pixbuf::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boolean(
                        "favorite",
                        "favorite",
                        "Is favorite",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boolean(
                        "downloaded",
                        "downloaded",
                        "Is Downloaded",
                        false,
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
                "name" => {
                    let name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.name.replace(name);
                }
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "favorite" => {
                    let favorite = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.favorite.replace(favorite);
                }
                "downloaded" => {
                    let downloaded = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.downloaded.replace(downloaded);
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

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "favorite" => self.favorite.borrow().to_value(),
                "downloaded" => self.downloaded.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the AssetData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct AssetData(ObjectSubclass<imp::AssetData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl AssetData {
    pub fn new(asset: &egs_api::api::types::asset_info::AssetInfo, image: &[u8]) -> AssetData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create AssetData");
        let self_: &imp::AssetData = imp::AssetData::from_instance(&data);

        data.set_property("id", &asset.id).unwrap();
        data.check_favorite();
        data.set_property("name", &asset.title).unwrap();
        self_.asset.replace(Some(asset.clone()));
        data.check_downloaded();
        let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
        pixbuf_loader.write(image).unwrap();
        pixbuf_loader.close().ok();

        data.configure_kind(asset);

        if let Some(pix) = pixbuf_loader.pixbuf() {
            data.set_property("thumbnail", &pix).unwrap();
        };
        data
    }

    pub fn decide_kind(asset: &AssetInfo) -> Option<AssetType> {
        return if let Some(cat) = &asset.categories {
            for c in cat {
                match c.path.as_str() {
                    "assets" => {
                        return Some(AssetType::Asset);
                    }
                    "games" => {
                        return Some(AssetType::Game);
                    }
                    "plugins" => {
                        return Some(AssetType::Plugin);
                    }
                    "projects" => {
                        return Some(AssetType::Project);
                    }
                    "engines" => {
                        return Some(AssetType::Engine);
                    }
                    _ => {}
                };
            }
            None
        } else {
            None
        };
    }

    fn configure_kind(&self, asset: &AssetInfo) {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        match Self::decide_kind(asset) {
            None => {
                self_.kind.replace(None);
            }
            Some(kind) => match kind {
                AssetType::Asset => {
                    self_.kind.replace(Some("asset".to_string()));
                }
                AssetType::Project => {
                    self_.kind.replace(Some("projects".to_string()));
                }
                AssetType::Game => {
                    self_.kind.replace(Some("games".to_string()));
                }
                AssetType::Engine => {
                    self_.kind.replace(Some("engines".to_string()));
                }
                AssetType::Plugin => {
                    self_.kind.replace(Some("plugins".to_string()));
                }
            },
        }
    }

    pub fn id(&self) -> String {
        if let Ok(value) = self.property("id") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn name(&self) -> String {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn favorite(&self) -> bool {
        if let Ok(value) = self.property("favorite") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }

    pub fn downloaded(&self) -> bool {
        if let Ok(value) = self.property("downloaded") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }

    pub fn release(&self) -> Option<DateTime<Utc>> {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        match &*self_.asset.borrow() {
            Some(a) => match a.latest_release() {
                None => a.last_modified_date,
                Some(ri) => ri.date_added,
            },
            None => None,
        }
    }

    pub fn kind(&self) -> Option<AssetType> {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        match &*self_.kind.borrow() {
            Some(a) => match a.as_str() {
                "asset" => Some(AssetType::Asset),
                "games" => Some(AssetType::Game),
                "plugins" => Some(AssetType::Plugin),
                "projects" => Some(AssetType::Project),
                "engines" => Some(AssetType::Engine),
                _ => None,
            },
            None => None,
        }
    }

    pub fn last_modified(&self) -> Option<DateTime<Utc>> {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        match &*self_.asset.borrow() {
            Some(a) => a.last_modified_date,
            None => None,
        }
    }

    pub fn image(&self) -> Option<Pixbuf> {
        if let Ok(value) = self.property("thumbnail") {
            if let Ok(id_opt) = value.get::<Pixbuf>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn check_category(&self, cat: &str) -> bool {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        if cat.eq("favorites") {
            self.favorite()
        } else if cat.eq("downloaded") {
            self.downloaded()
        } else if cat.starts_with("!other") {
            match self_.asset.borrow().as_ref() {
                None => false,
                Some(b) => {
                    for category in b.categories.as_ref().unwrap() {
                        for split in cat.split('|') {
                            if category
                                .path
                                .to_ascii_lowercase()
                                .contains(&split.to_ascii_lowercase())
                            {
                                return false;
                            }
                        }
                    }
                    true
                }
            }
        } else {
            match self_.asset.borrow().as_ref() {
                None => false,
                Some(b) => {
                    for category in b.categories.as_ref().unwrap() {
                        for split in cat.split('|') {
                            if category
                                .path
                                .to_ascii_lowercase()
                                .contains(&split.to_ascii_lowercase())
                            {
                                return true;
                            }
                        }
                    }
                    false
                }
            }
        }
    }

    pub fn check_downloaded(&self) {
        let self_: &imp::AssetData = imp::AssetData::from_instance(self);
        let asset = &*self_.asset.borrow();
        if let Some(ass) = asset {
            for vault in self_.settings.strv("unreal-vault-directories") {
                let pathbuf = std::path::PathBuf::from(&vault);
                if let Some(ris) = &ass.release_info {
                    for ri in ris {
                        let mut p = pathbuf.clone();
                        if let Some(app) = &ri.app_id {
                            p.push(&app);
                            p.push("data");
                            if p.exists() {
                                self.set_property("downloaded", true).unwrap();
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn check_favorite(&self) {
        let db = crate::models::database::connection();
        if let Ok(conn) = db.get() {
            let ex: Result<bool, diesel::result::Error> = select(exists(
                crate::schema::favorite_asset::table
                    .filter(crate::schema::favorite_asset::asset.eq(self.id())),
            ))
            .get_result(&conn);
            if let Ok(fav) = ex {
                self.set_property("favorite", fav).unwrap();
                return;
            }
        }
        self.set_property("favorite", false).unwrap();
    }

    pub fn refresh(&self) {
        self.check_favorite();
        self.emit_by_name("refreshed", &[]).unwrap();
    }
}
