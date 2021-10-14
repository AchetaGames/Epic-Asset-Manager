use std::path::PathBuf;

use adw::traits::ActionRowExt;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};

use crate::models::engine_data::EngineData;
use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::logged_in::engines::UnrealEngine;
use std::ffi::OsString;
use std::ops::Deref;
use std::str::FromStr;

pub(crate) mod imp {
    use std::cell::RefCell;

    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;

    use crate::models::project_data::Uproject;
    use crate::window::EpicAssetManagerWindow;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engine_detail.ui")]
    pub struct EpicEngineDetails {
        pub expanded: RefCell<bool>,
        #[template_child]
        pub detail_slider: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        #[template_child]
        pub launch_button: TemplateChild<gtk4::Button>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        pub settings: gio::Settings,
        pub data: RefCell<Option<crate::models::engine_data::EngineData>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineDetails {
        const NAME: &'static str = "EpicEngineDetails";
        type Type = super::EpicEngineDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                detail_slider: TemplateChild::default(),
                title: TemplateChild::default(),
                launch_button: Default::default(),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                data: RefCell::new(None),
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

    impl ObjectImpl for EpicEngineDetails {
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

    impl WidgetImpl for EpicEngineDetails {}
    impl BoxImpl for EpicEngineDetails {}
}

glib::wrapper! {
    pub struct EpicEngineDetails(ObjectSubclass<imp::EpicEngineDetails>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEngineDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngineDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn setup_actions(&self) {
        let self_: &imp::EpicEngineDetails = imp::EpicEngineDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("engine_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.set_property("expanded", false).unwrap();
            })
        );

        action!(
            self_.actions,
            "launch",
            clone!(@weak self as engines => move |_, _| {
                let path = engines.path();
                if let Some(path) = path {
                    match Self::get_engine_binary_path(&path) {
                        None => { warn!("No path");}
                        Some(p) => {
                            let context = gtk4::gio::AppLaunchContext::new();
                            context.setenv("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2");
                            let app = gtk4::gio::AppInfo::create_from_commandline(
                                p,
                                Some("Unreal Engine"),
                                gtk4::gio::AppInfoCreateFlags::NONE,
                            ).unwrap();
                            app.launch(&[], Some(&context)).expect("Failed to launch application");
                        }
                    }
                };
            })
        );
    }

    pub fn set_data(&self, data: crate::models::engine_data::EngineData) {
        let self_: &imp::EpicEngineDetails = imp::EpicEngineDetails::from_instance(self);
        if let Some(title) = &data.version() {
            self_
                .title
                .set_markup(&format!("<b><u><big>{}</big></u></b>", title));
        }
        self_.launch_button.set_visible(true);

        self_.data.replace(Some(data));
    }

    pub fn add_engine(&self) {
        let self_: &imp::EpicEngineDetails = imp::EpicEngineDetails::from_instance(self);
        self_.data.replace(None);
        self_.launch_button.set_visible(false);
        self_
            .title
            .set_markup("<b><u><big>Add Engine</big></u></b>");
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicEngineDetails = imp::EpicEngineDetails::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    fn get_engine_binary_path(path: &str) -> Option<OsString> {
        if let Ok(mut p) = std::path::PathBuf::from_str(path) {
            p.push("Engine");
            p.push("Binaries");
            p.push("Linux");
            let mut test = p.clone();
            test.push("UE4Editor");
            if test.exists() {
                return Some(test.into_os_string());
            } else {
                let mut test = p.clone();
                test.push("UnrealEditor");
                if test.exists() {
                    return Some(test.into_os_string());
                } else {
                    error!("Unable to launch the engine")
                }
            }
        };
        None
    }

    fn path(&self) -> Option<String> {
        let self_: &imp::EpicEngineDetails = imp::EpicEngineDetails::from_instance(self);
        if let Some(d) = self_.data.borrow().deref() {
            return d.path();
        }
        None
    }

    fn is_expanded(&self) -> bool {
        if let Ok(value) = self.property("expanded") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }
}
