use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod imp {
    use super::*;
    use gtk4::gdk::Texture;
    use gtk4::glib::{ParamSpecObject, SignalHandlerId};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset.ui")]
    pub struct EpicAsset {
        id: RefCell<Option<String>>,
        label: RefCell<Option<String>>,
        favorite: RefCell<bool>,
        pub downloaded: RefCell<bool>,
        pub kind: RefCell<Option<String>>,
        pub action_label: RefCell<String>,
        thumbnail: RefCell<Option<Texture>>,
        #[template_child]
        pub image: TemplateChild<adw::Avatar>,
        #[template_child]
        pub action_button: TemplateChild<gtk4::Button>,
        pub data: RefCell<Option<crate::models::asset_data::AssetData>>,
        pub handler: RefCell<Option<SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAsset {
        const NAME: &'static str = "EpicAsset";
        type Type = super::EpicAsset;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                label: RefCell::new(None),
                favorite: RefCell::new(false),
                downloaded: RefCell::new(false),
                kind: RefCell::new(None),
                action_label: RefCell::new("Download".to_string()),
                thumbnail: RefCell::new(None),
                image: TemplateChild::default(),
                action_button: TemplateChild::default(),
                data: RefCell::new(None),
                handler: RefCell::new(None),
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

    impl ObjectImpl for EpicAsset {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_button();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![
                        glib::subclass::Signal::builder("download-requested")
                            .flags(glib::SignalFlags::ACTION)
                            .build(),
                        glib::subclass::Signal::builder("add-to-project-requested")
                            .flags(glib::SignalFlags::ACTION)
                            .build(),
                        glib::subclass::Signal::builder("create-project-requested")
                            .flags(glib::SignalFlags::ACTION)
                            .build(),
                    ]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("label").build(),
                    glib::ParamSpecString::builder("id").build(),
                    ParamSpecObject::builder::<Texture>("thumbnail").build(),
                    glib::ParamSpecBoolean::builder("favorite").build(),
                    glib::ParamSpecBoolean::builder("downloaded").build(),
                    glib::ParamSpecString::builder("kind").build(),
                    glib::ParamSpecString::builder("action-label").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "label" => {
                    let label = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.label.replace(label);
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
                    let downloaded: bool = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.downloaded.replace(downloaded);
                    self.obj().update_action_label();
                }
                "kind" => {
                    let kind = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.kind.replace(kind);
                    self.obj().update_action_label();
                }
                "action-label" => {
                    let action_label: String = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.action_label.replace(action_label);
                }
                "thumbnail" => {
                    let thumbnail: Option<Texture> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");

                    self.thumbnail.replace(thumbnail.clone());
                    thumbnail.map_or_else(
                        || {
                            self.image.set_icon_name(Some("ue-logo-symbolic"));
                        },
                        |t| {
                            self.image.set_custom_image(Some(&t));
                            // self.image.set_paintable(Some(&t));
                        },
                    );
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "favorite" => self.favorite.borrow().to_value(),
                "downloaded" => self.downloaded.borrow().to_value(),
                "kind" => self.kind.borrow().to_value(),
                "action-label" => self.action_label.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicAsset {}
    impl BoxImpl for EpicAsset {}
}

glib::wrapper! {
    pub struct EpicAsset(ObjectSubclass<imp::EpicAsset>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicAsset {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAsset {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup_button(&self) {
        let self_ = self.imp();
        self_.action_button.connect_clicked(clone!(
            #[weak(rename_to=asset)]
            self,
            move |_| {
                asset.on_action_clicked();
            }
        ));
    }

    fn update_action_label(&self) {
        let self_ = self.imp();
        let downloaded = *self_.downloaded.borrow();
        let kind = self_.kind.borrow().clone();

        let label = if !downloaded {
            "Download".to_string()
        } else {
            match kind.as_deref() {
                Some("projects") => "Create Project".to_string(),
                _ => "Add to Project".to_string(),
            }
        };

        self_.action_label.replace(label.clone());
        self.notify("action-label");
    }

    fn on_action_clicked(&self) {
        let self_ = self.imp();
        let downloaded = *self_.downloaded.borrow();
        let kind = self_.kind.borrow().clone();

        // Emit signal for parent to handle the action
        if !downloaded {
            self.emit_by_name::<()>("download-requested", &[]);
        } else {
            match kind.as_deref() {
                Some("projects") => {
                    self.emit_by_name::<()>("create-project-requested", &[]);
                }
                _ => {
                    self.emit_by_name::<()>("add-to-project-requested", &[]);
                }
            }
        }
    }

    pub fn set_data(&self, data: &crate::models::asset_data::AssetData) {
        use crate::models::asset_data::AssetType;

        let self_ = self.imp();
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        self_.data.replace(Some(data.clone()));
        self.set_property("label", data.name());
        self.set_property("thumbnail", data.image());
        self.set_property("favorite", data.favorite());

        // Set kind before downloaded so action_label updates correctly
        let kind_str = match data.kind() {
            Some(AssetType::Asset) => Some("asset".to_string()),
            Some(AssetType::Project) => Some("projects".to_string()),
            Some(AssetType::Game) => Some("games".to_string()),
            Some(AssetType::Engine) => Some("engines".to_string()),
            Some(AssetType::Plugin) => Some("plugins".to_string()),
            None => None,
        };
        self.set_property("kind", kind_str);
        self.set_property("downloaded", data.downloaded());

        self_.handler.replace(Some(data.connect_local(
            "refreshed",
            false,
            clone!(
                #[weak(rename_to=asset)]
                self,
                #[weak]
                data,
                #[upgrade_or]
                None,
                move |_| {
                    asset.set_property("favorite", data.favorite());
                    asset.set_property("downloaded", data.downloaded());
                    None
                }
            ),
        )));
    }
}
