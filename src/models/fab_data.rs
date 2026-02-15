use diesel::dsl::exists;
use diesel::{select, ExpressionMethods, QueryDsl, RunQueryDsl};
use egs_api::api::types::fab_library::FabAsset;
use gtk4::gdk::Texture;
use gtk4::prelude::ObjectExt;
use gtk4::prelude::SettingsExtManual;
use gtk4::{glib, subclass::prelude::*};
use log::error;
use std::path::PathBuf;

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
    pub struct FabData {
        id: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        favorite: RefCell<bool>,
        downloaded: RefCell<bool>,
        downloading: RefCell<bool>,
        download_progress: RefCell<f64>,
        download_speed: RefCell<String>,
        pub asset: RefCell<Option<FabAsset>>,
        thumbnail: RefCell<Option<Texture>>,
        pub settings: gtk4::gio::Settings,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for FabData {
        const NAME: &'static str = "FabData";
        type Type = super::FabData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                name: RefCell::new(None),
                favorite: RefCell::new(false),
                downloaded: RefCell::new(false),
                downloading: RefCell::new(false),
                download_progress: RefCell::new(0.0),
                download_speed: RefCell::new(String::new()),
                asset: RefCell::new(None),
                thumbnail: RefCell::new(None),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
            }
        }
    }

    impl ObjectImpl for FabData {
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
                    glib::ParamSpecBoolean::builder("downloading").build(),
                    glib::ParamSpecDouble::builder("download-progress")
                        .minimum(0.0)
                        .maximum(1.0)
                        .default_value(0.0)
                        .build(),
                    glib::ParamSpecString::builder("download-speed").build(),
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
                "downloading" => {
                    let downloading = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.downloading.replace(downloading);
                }
                "download-progress" => {
                    let progress = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.download_progress.replace(progress);
                }
                "download-speed" => {
                    let speed: Option<String> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.download_speed.replace(speed.unwrap_or_default());
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
                "downloading" => self.downloading.borrow().to_value(),
                "download-progress" => self.download_progress.borrow().to_value(),
                "download-speed" => self.download_speed.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct FabData(ObjectSubclass<imp::FabData>);
}

impl FabData {
    pub fn new(asset: &FabAsset, image: Option<Texture>) -> FabData {
        let data: Self = glib::Object::new::<Self>();
        let self_ = data.imp();

        data.set_property("id", &asset.asset_id);
        data.check_favorite();
        data.set_property("name", &asset.title);
        self_.asset.replace(Some(asset.clone()));
        data.check_downloaded();

        if let Some(tex) = image {
            data.set_property("thumbnail", tex);
        };
        data
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
                for category in &b.categories {
                    if let Some(name) = &category.name {
                        if name
                            .to_ascii_lowercase()
                            .contains(&cat.to_ascii_lowercase())
                        {
                            return true;
                        }
                    }
                    // Also check category id
                    if category
                        .id
                        .to_ascii_lowercase()
                        .contains(&cat.to_ascii_lowercase())
                    {
                        return true;
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
            let vaults = self_.settings.strv("unreal-vault-directories");
            if !Self::downloaded_locations(&vaults, &ass.asset_id).is_empty() {
                self.set_property("downloaded", true);
                return;
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

    pub fn downloading(&self) -> bool {
        self.property("downloading")
    }

    pub fn set_downloading(&self, downloading: bool) {
        self.set_property("downloading", downloading);
        self.emit_by_name::<()>("refreshed", &[]);
    }

    pub fn download_progress(&self) -> f64 {
        self.property("download-progress")
    }

    pub fn set_download_progress(&self, progress: f64) {
        self.set_property("download-progress", progress);
        self.emit_by_name::<()>("refreshed", &[]);
    }

    pub fn download_speed(&self) -> String {
        self.property("download-speed")
    }

    pub fn set_download_speed(&self, speed: &str) {
        self.set_property("download-speed", speed);
    }

    /// Set both progress and speed together, emitting only one refresh signal
    pub fn set_download_info(&self, progress: f64, speed: &str) {
        self.set_property("download-progress", progress);
        self.set_property("download-speed", speed);
        self.emit_by_name::<()>("refreshed", &[]);
    }
}
