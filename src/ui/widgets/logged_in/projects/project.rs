use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::logged_in::engines::UnrealEngine;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecString, SignalHandlerId};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/project.ui")]
    pub struct EpicProject {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        name: RefCell<Option<String>>,
        engine: RefCell<Option<String>>,
        pub data: RefCell<Option<crate::models::project_data::ProjectData>>,
        pub handler: RefCell<Option<SignalHandlerId>>,
        #[template_child]
        pub thumbnail: TemplateChild<adw::Avatar>,
        pub settings: gtk4::gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicProject {
        const NAME: &'static str = "EpicProject";
        type Type = super::EpicProject;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                name: RefCell::new(None),
                engine: RefCell::new(None),
                data: RefCell::new(None),
                handler: RefCell::new(None),
                thumbnail: TemplateChild::default(),
                settings: gtk4::gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for EpicProject {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("name").build(),
                    ParamSpecString::builder("engine").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
                "name" => {
                    let name = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("{l}"));
                    self.name.replace(name);
                }
                "engine" => {
                    let engine = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("{l}"));
                    self.engine.replace(engine);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "engine" => self.engine.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicProject {}
    impl BoxImpl for EpicProject {}
}

glib::wrapper! {
    pub struct EpicProject(ObjectSubclass<imp::EpicProject>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicProject {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicProject {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
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
        let self_ = self.imp();
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_data(&self, data: &crate::models::project_data::ProjectData) {
        let self_ = self.imp();
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        self_.data.replace(Some(data.clone()));
        self.set_property("name", data.name());
        self.set_property("tooltip-text", data.path());
        data.uproject().map_or_else(
            || {
                self.set_property("engine", "");
            },
            |uproject| match self.associated_engine(&uproject) {
                None => {
                    let db = crate::models::database::connection();
                    #[allow(clippy::collection_is_never_read)]
                    let mut last_engine: Option<String> = None;
                    if let Ok(mut conn) = db.get() {
                        let engines: Result<String, diesel::result::Error> =
                            unreal_project_latest_engine::table
                                .filter(
                                    crate::schema::unreal_project_latest_engine::project
                                        .eq(&data.path().unwrap()),
                                )
                                .select(crate::schema::unreal_project_latest_engine::engine)
                                .first(&mut conn);
                        if let Ok(last) = engines {
                            last_engine = Some(last);
                        }
                    };
                    last_engine.map_or_else(
                        || {
                            self.set_property("engine", "Unknown Engine");
                        },
                        |eng| {
                            crate::models::engine_data::EngineData::read_engine_version(&eng)
                                .map_or_else(
                                    || {
                                        self.set_property("engine", eng);
                                    },
                                    |version| {
                                        self.set_property("engine", version.format());
                                    },
                                );
                        },
                    );
                }
                Some(eng) => {
                    self.set_property("engine", eng.version.format());
                }
            },
        );

        if let Some(pix) = data.image() {
            self_.thumbnail.set_custom_image(Some(&pix));
        }

        data.path().map_or_else(
            || {
                self_.thumbnail.set_text(None);
            },
            |path| {
                self_.thumbnail.set_text(Some(&path));
            },
        );

        self_.handler.replace(Some(data.connect_local(
            "finished",
            false,
            clone!(
                #[weak(rename_to=project)]
                self,
                #[weak]
                data,
                #[upgrade_or]
                None,
                move |_| {
                    project.finished(&data);
                    None
                }
            ),
        )));
    }

    fn finished(&self, data: &crate::models::project_data::ProjectData) {
        let self_ = self.imp();
        if let Some(pix) = data.image() {
            self_.thumbnail.set_custom_image(Some(&pix));
        }
    }

    fn associated_engine(&self, uproject: &Uproject) -> Option<UnrealEngine> {
        let self_ = self.imp();
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l_ = w_.logged_in_stack.imp();
            return l_
                .engines
                .engine_from_assoociation(&uproject.engine_association);
        }
        None
    }
}
