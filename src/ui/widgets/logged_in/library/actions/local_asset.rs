use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use log::debug;

pub mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/local_asset.ui")]
    pub struct EpicLocalAsset {
        label: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        pub actions: gio::SimpleActionGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicLocalAsset {
        const NAME: &'static str = "EpicLocalAsset";
        type Type = super::EpicLocalAsset;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                label: RefCell::new(None),
                path: RefCell::new(None),
                actions: gio::SimpleActionGroup::new(),
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

    impl ObjectImpl for EpicLocalAsset {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("delete")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::builder("label").build(),
                    glib::ParamSpecString::builder("path").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "label" => {
                    let label = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    let formatted = label.as_ref().map(|l| format!("<b><u>{l}</u></b>"));
                    self.label.replace(formatted);
                    self.obj().set_property("path", label);
                }
                "path" => {
                    let path = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.path.replace(path);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicLocalAsset {}
    impl BoxImpl for EpicLocalAsset {}
}

glib::wrapper! {
    pub struct EpicLocalAsset(ObjectSubclass<imp::EpicLocalAsset>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicLocalAsset {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicLocalAsset {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        self.insert_action_group("local_asset", Some(&self_.actions));
        action!(
            self_.actions,
            "open",
            clone!(
                #[weak(rename_to=local_asset)]
                self,
                move |_, _| {
                    local_asset.open_path();
                }
            )
        );

        action!(
            self_.actions,
            "delete",
            clone!(
                #[weak(rename_to=local_asset)]
                self,
                move |_, _| {
                    local_asset.delete();
                }
            )
        );
    }

    pub fn delete(&self) {
        self.emit_by_name::<()>("delete", &[]);
    }

    pub fn open_path(&self) {
        if let Some(p) = self.path() {
            debug!("Trying to open {}", p);
            #[cfg(target_os = "linux")]
            {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    crate::tools::open_directory(&p).await;
                });
            };
        }
    }

    pub fn label(&self) -> Option<String> {
        self.property("label")
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }
}
