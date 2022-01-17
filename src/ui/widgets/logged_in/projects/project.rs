use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::logged_in::engines::UnrealEngine;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::new("name", "Name", "Name", None, glib::ParamFlags::READWRITE),
                    ParamSpecString::new(
                        "engine",
                        "Engine",
                        "Engine",
                        None,
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
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "name" => {
                    let name = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<span size=\"xx-large\"><b><u>{}</u></b></span>", l));
                    self.name.replace(name);
                }
                "engine" => {
                    let engine = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<i>{}</i>", l));
                    self.engine.replace(engine);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
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
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
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
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn set_data(&self, data: &crate::models::project_data::ProjectData) {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        self_.data.replace(Some(data.clone()));
        self.set_property("name", &data.name());
        self.set_property("tooltip-text", &data.path());
        match data.uproject() {
            None => {
                self.set_property("engine", "");
            }
            Some(uproject) => match self.associated_engine(&uproject) {
                None => {
                    let db = crate::models::database::connection();
                    let mut last_engine: Option<String> = None;
                    if let Ok(conn) = db.get() {
                        let engines: Result<String, diesel::result::Error> =
                            unreal_project_latest_engine::table
                                .filter(
                                    crate::schema::unreal_project_latest_engine::project
                                        .eq(&data.path().unwrap()),
                                )
                                .select(crate::schema::unreal_project_latest_engine::engine)
                                .first(&conn);
                        if let Ok(last) = engines {
                            last_engine = Some(last);
                        }
                    };
                    match last_engine {
                        None => {
                            self.set_property("engine", "Unknown Engine");
                        }
                        Some(eng) => {
                            match crate::models::engine_data::EngineData::read_engine_version(&eng)
                            {
                                None => {
                                    self.set_property("engine", eng);
                                }
                                Some(version) => {
                                    self.set_property("engine", version.format());
                                }
                            }
                        }
                    }
                }
                Some(eng) => {
                    self.set_property("engine", eng.version.format());
                }
            },
        };

        if let Some(pix) = data.image() {
            self_
                .thumbnail
                .set_custom_image(Some(&gtk4::gdk::Texture::for_pixbuf(&pix)));
        }

        match data.path() {
            None => {
                self_.thumbnail.set_text(None);
            }
            Some(path) => {
                self_.thumbnail.set_text(Some(&path));
            }
        }

        self_.handler.replace(Some(data.connect_local(
            "finished",
            false,
            clone!(@weak self as project, @weak data => @default-return None, move |_| {
                let self_: &imp::EpicProject = imp::EpicProject::from_instance(&project);
                if let Some(pix) = data.image() {
                    self_.thumbnail.set_custom_image(Some(&gtk4::gdk::Texture::for_pixbuf(&pix)));
                }
                None
            }),
        )));
    }

    fn associated_engine(&self, uproject: &Uproject) -> Option<UnrealEngine> {
        let self_: &imp::EpicProject = imp::EpicProject::from_instance(self);
        if let Some(w) = self_.window.get() {
            let w_: &crate::window::imp::EpicAssetManagerWindow =
                crate::window::imp::EpicAssetManagerWindow::from_instance(w);
            let l = w_.logged_in_stack.clone();
            let l_: &crate::ui::widgets::logged_in::imp::EpicLoggedInBox =
                crate::ui::widgets::logged_in::imp::EpicLoggedInBox::from_instance(&l);
            return l_
                .engine
                .engine_from_assoociation(&uproject.engine_association);
        }
        None
    }
}
