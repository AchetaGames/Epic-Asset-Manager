use chrono::{DateTime, Utc};
use diesel::dsl::exists;
use diesel::{select, ExpressionMethods, QueryDsl, RunQueryDsl};
use egs_api::api::types::asset_info::AssetInfo;
use gtk4::gdk::Texture;
use gtk4::prelude::ObjectExt;
use gtk4::prelude::SettingsExtManual;
use gtk4::{glib, subclass::prelude::*};
use log::error;
use std::path::PathBuf;

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
    use gtk4::gdk::Texture;
    use gtk4::glib::ParamSpecObject;
    use gtk4::prelude::ToValue;
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
        pub asset: RefCell<Option<AssetInfo>>,
        thumbnail: RefCell<Option<Texture>>,
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
                    glib::ParamSpecString::builder("name").build(),
                    glib::ParamSpecString::builder("id").build(),
                    ParamSpecObject::builder::<Texture>("thumbnail").build(),
                    glib::ParamSpecBoolean::builder("favorite").build(),
                    glib::ParamSpecBoolean::builder("downloaded").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
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
    pub fn new(asset: &AssetInfo, image: Option<Texture>) -> AssetData {
        let data: Self = glib::Object::new::<Self>();
        let self_ = data.imp();

        data.set_property("id", &asset.id);
        data.check_favorite();
        data.set_property("name", &asset.title);
        self_.asset.replace(Some(asset.clone()));
        data.check_downloaded();

        data.configure_kind(asset);

        if let Some(tex) = image {
            data.set_property("thumbnail", tex);
        };
        data
    }

    pub fn decide_kind(asset: &AssetInfo) -> Option<AssetType> {
        if let Some(cat) = &asset.categories {
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
        };
        None
    }

    fn configure_kind(&self, asset: &AssetInfo) {
        let self_ = self.imp();
        Self::decide_kind(asset).map_or_else(
            || {
                self_.kind.replace(None);
            },
            |kind| match kind {
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
        );
    }

    pub fn id(&self) -> String {
        self.property("id")
    }

    pub fn name(&self) -> String {
        self.property("name")
    }

    pub fn favorite(&self) -> bool {
        self.property("favorite")
    }

    pub fn downloaded(&self) -> bool {
        self.property("downloaded")
    }

    pub fn release(&self) -> Option<DateTime<Utc>> {
        let self_ = self.imp();
        (*self_.asset.borrow())
            .as_ref()
            .and_then(|a| match a.latest_release() {
                None => a.last_modified_date,
                Some(ri) => ri.date_added,
            })
    }

    pub fn kind(&self) -> Option<AssetType> {
        let self_ = self.imp();
        (*self_.kind.borrow())
            .as_ref()
            .and_then(|a| match a.as_str() {
                "asset" => Some(AssetType::Asset),
                "games" => Some(AssetType::Game),
                "plugins" => Some(AssetType::Plugin),
                "projects" => Some(AssetType::Project),
                "engines" => Some(AssetType::Engine),
                _ => None,
            })
    }

    pub fn last_modified(&self) -> Option<DateTime<Utc>> {
        let self_ = self.imp();
        (*self_.asset.borrow())
            .as_ref()
            .and_then(|a| a.last_modified_date)
    }

    pub fn image(&self) -> Option<Texture> {
        self.property("thumbnail")
    }

    fn has_category(&self, cat: &str) -> bool {
        if cat.eq("favorites") {
            self.favorite()
        } else if cat.eq("downloaded") {
            self.downloaded()
        } else {
            let self_ = self.imp();
            if let Some(b) = self_.asset.borrow().as_ref() {
                if let Some(categories) = &b.categories {
                    for category in categories {
                        if category
                            .path
                            .to_ascii_lowercase()
                            .contains(&cat.to_ascii_lowercase())
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }

    pub fn check_category(&self, cat: &str) -> bool {
        cat.split(&['|', '&']).next().map_or(false, |c| {
            let result = if c.starts_with('!') {
                let mut chars = c.chars();
                chars.next();
                !self.has_category(chars.as_str())
            } else {
                self.has_category(c)
            };

            cat.chars().nth(c.len()).map_or(result, |operator| {
                let remainder: String = cat.chars().skip(c.len() + 1).collect();
                match operator {
                    '&' => {
                        if result {
                            self.check_category(&remainder)
                        } else {
                            result
                        }
                    }
                    '|' => result || self.check_category(&remainder),
                    _ => {
                        error!("Unimplemented operator");
                        false
                    }
                }
            })
        })
    }

    pub fn check_downloaded(&self) {
        let self_ = self.imp();
        let asset = &*self_.asset.borrow();
        if let Some(ass) = asset {
            if let Some(ris) = &ass.release_info {
                let vaults = self_.settings.strv("unreal-vault-directories");
                for ri in ris {
                    if let Some(app) = &ri.app_id {
                        if !Self::downloaded_locations(&vaults, app).is_empty() {
                            self.set_property("downloaded", true);
                            return;
                        }
                    }
                }
            }
        }
        self.set_property("downloaded", false);
    }

    pub fn downloaded_locations(directories: &glib::StrV, asset_id: &str) -> Vec<PathBuf> {
        let mut result: Vec<PathBuf> = Vec::new();
        for directory in directories {
            let mut path = std::path::PathBuf::from(directory.as_str());
            path.push(asset_id);
            path.push("data");
            if path.exists() {
                result.push(path);
            }
        }
        result
    }

    pub fn check_favorite(&self) {
        let db = crate::models::database::connection();
        if let Ok(mut conn) = db.get() {
            let ex: Result<bool, diesel::result::Error> = select(exists(
                crate::schema::favorite_asset::table
                    .filter(crate::schema::favorite_asset::asset.eq(self.id())),
            ))
            .get_result(&mut conn);
            if let Ok(fav) = ex {
                self.set_property("favorite", fav);
                return;
            }
        }
        self.set_property("favorite", false);
    }

    pub fn refresh(&self) {
        self.check_favorite();
        self.check_downloaded();
        self.emit_by_name::<()>("refreshed", &[]);
    }
}
