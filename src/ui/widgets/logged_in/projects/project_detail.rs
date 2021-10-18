use std::path::PathBuf;

use adw::traits::ActionRowExt;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};

use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::logged_in::engines::UnrealEngine;

pub(crate) mod imp {
    use std::cell::RefCell;

    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;

    use crate::models::project_data::Uproject;
    use crate::window::EpicAssetManagerWindow;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/project_detail.ui")]
    pub struct UnrealProjectDetails {
        pub expanded: RefCell<bool>,
        #[template_child]
        pub detail_slider: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub details: TemplateChild<gtk4::Box>,
        #[template_child]
        pub details_box: TemplateChild<gtk4::Box>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        path: RefCell<Option<String>>,
        pub uproject: RefCell<Option<Uproject>>,
        pub engine: RefCell<Option<UnrealEngine>>,
        pub settings: gio::Settings,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnrealProjectDetails {
        const NAME: &'static str = "UnrealProjectDetails";
        type Type = super::UnrealProjectDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                detail_slider: TemplateChild::default(),
                details: TemplateChild::default(),
                details_box: TemplateChild::default(),
                title: TemplateChild::default(),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                path: RefCell::new(None),
                uproject: RefCell::new(None),
                engine: RefCell::new(None),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for UnrealProjectDetails {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "expanded",
                        "expanded",
                        "Is expanded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "path",
                        "Path",
                        "Path",
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
                "expanded" => {
                    let expanded = value.get().unwrap();
                    self.expanded.replace(expanded);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for UnrealProjectDetails {}
    impl BoxImpl for UnrealProjectDetails {}
}

glib::wrapper! {
    pub struct UnrealProjectDetails(ObjectSubclass<imp::UnrealProjectDetails>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for UnrealProjectDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl UnrealProjectDetails {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn setup_actions(&self) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        let actions = &self_.actions;
        self.insert_action_group("project_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.set_property("expanded", false).unwrap();
            })
        );

        action!(
            actions,
            "launch_project",
            clone!(@weak self as details => move |_, _| {
                details.launch_engine();
            })
        );
    }

    fn launch_engine(&self) {
        let path = self.path().unwrap();
        let engine = self.engine();
        if let Some(eng) = engine {
            if let Some(p) = eng.get_engine_binary_path() {
                let db = crate::models::database::connection();
                if let Ok(conn) = db.get() {
                    diesel::replace_into(unreal_project_latest_engine::table)
                        .values((
                            crate::schema::unreal_project_latest_engine::project.eq(path.clone()),
                            crate::schema::unreal_project_latest_engine::engine.eq(eng.path),
                        ))
                        .execute(&conn)
                        .expect("Unable to insert last engine to the DB");
                };
                let context = gtk4::gio::AppLaunchContext::new();
                context.setenv("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2");
                let app = gtk4::gio::AppInfo::create_from_commandline(
                    format!("\"{:?}\" \"{}\"", p, path),
                    Some("Unreal Engine"),
                    gtk4::gio::AppInfoCreateFlags::NONE,
                )
                .unwrap();
                app.launch(&[], Some(&context))
                    .expect("Failed to launch application");
            }
        };
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    fn set_launch_enabled(&self, enabled: bool) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        get_action!(self_.actions, @launch_project).set_enabled(enabled);
    }

    pub fn set_project(
        &self,
        project: &crate::models::project_data::Uproject,
        path: Option<String>,
    ) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        self.set_property("path", &path).unwrap();
        self_.uproject.replace(Some(project.clone()));
        while let Some(el) = self_.details_box.first_child() {
            self_.details_box.remove(&el);
        }
        if path.is_none() {
            return;
        }

        let pathbuf = PathBuf::from(path.unwrap());
        self_.title.set_markup(&format!(
            "<b><u><big>{}</big></u></b>",
            pathbuf.file_stem().unwrap().to_str().unwrap().to_string()
        ));

        let size_group_labels = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
        let size_group_prefix = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);

        // Engine
        let row = adw::ActionRowBuilder::new().activatable(true).build();
        let title = gtk4::LabelBuilder::new().label("Engine").build();
        size_group_prefix.add_widget(&title);
        row.add_prefix(&title);
        let combo = gtk4::ComboBoxText::new();
        let associated = self.associated_engine(project);
        self.set_launch_enabled(false);
        let db = crate::models::database::connection();
        let mut last_engine: Option<String> = None;
        if let Ok(conn) = db.get() {
            let engines: Result<String, diesel::result::Error> =
                unreal_project_latest_engine::table
                    .filter(
                        crate::schema::unreal_project_latest_engine::project
                            .eq(&self.path().unwrap()),
                    )
                    .select(crate::schema::unreal_project_latest_engine::engine)
                    .first(&conn);
            if let Ok(last) = engines {
                last_engine = Some(last);
            }
        };
        for engine in self.available_engines() {
            combo.append(
                Some(&engine.path),
                &format!(
                    "{}{}",
                    engine.version.format(),
                    match associated.clone() {
                        None => {
                            last_engine.clone().map_or("", |last| {
                                if engine.path.eq(&last) {
                                    " (last)"
                                } else {
                                    ""
                                }
                            })
                        }
                        Some(eng) => {
                            if eng.path.eq(&engine.path) {
                                " (current)"
                            } else if let Some(last) = last_engine.clone() {
                                if engine.path.eq(&last) {
                                    " (last)"
                                } else {
                                    ""
                                }
                            } else {
                                ""
                            }
                        }
                    }
                ),
            );
        }

        combo.connect_changed(clone!(@weak self as detail => move |c| {
            if let Some(com) = c.active_id() { detail.engine_selected(&com); }

        }));
        if let Some(engine) = associated {
            combo.set_active_id(Some(&engine.path));
        } else if let Some(last) = last_engine {
            combo.set_active_id(Some(&last));
        };
        // TODO: Change the project config based on the engine selected
        size_group_labels.add_widget(&combo);
        row.add_suffix(&combo);
        self_.details_box.append(&row);

        // Path
        let row = adw::ActionRowBuilder::new().activatable(true).build();
        let title = gtk4::LabelBuilder::new().label("Path").build();
        size_group_prefix.add_widget(&title);
        row.add_prefix(&title);
        let label = gtk4::LabelBuilder::new()
            .label(pathbuf.parent().unwrap().to_str().unwrap())
            .wrap(true)
            .xalign(0.0)
            .build();
        size_group_labels.add_widget(&label);
        row.add_suffix(&label);
        self_.details_box.append(&row);

        // Engine Association
        let row = adw::ActionRowBuilder::new().activatable(true).build();
        let title = gtk4::LabelBuilder::new()
            .label("Engine Association")
            .build();
        size_group_prefix.add_widget(&title);
        row.add_prefix(&title);
        let label = gtk4::LabelBuilder::new()
            .label(&project.engine_association)
            .wrap(true)
            .xalign(0.0)
            .build();
        size_group_labels.add_widget(&label);
        row.add_suffix(&label);
        self_.details_box.append(&row);

        if !self.is_expanded() {
            self.set_property("expanded", true).unwrap();
        }
    }

    fn engine_selected(&self, path: &str) {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        for engine in self.available_engines() {
            if engine.path.eq(path) {
                self.set_launch_enabled(true);
                self_.engine.replace(Some(engine));
            }
        }
    }

    fn associated_engine(&self, uproject: &Uproject) -> Option<UnrealEngine> {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
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

    fn available_engines(&self) -> Vec<UnrealEngine> {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        if let Some(w) = self_.window.get() {
            let w_: &crate::window::imp::EpicAssetManagerWindow =
                crate::window::imp::EpicAssetManagerWindow::from_instance(w);
            let l = w_.logged_in_stack.clone();
            let l_: &crate::ui::widgets::logged_in::imp::EpicLoggedInBox =
                crate::ui::widgets::logged_in::imp::EpicLoggedInBox::from_instance(&l);
            return l_.engine.engines();
        }
        Vec::new()
    }

    fn is_expanded(&self) -> bool {
        if let Ok(value) = self.property("expanded") {
            if let Ok(id_opt) = value.get::<bool>() {
                return id_opt;
            }
        };
        false
    }

    pub fn uproject(&self) -> Option<Uproject> {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        self_.uproject.borrow().clone()
    }

    fn engine(&self) -> Option<UnrealEngine> {
        let self_: &imp::UnrealProjectDetails = imp::UnrealProjectDetails::from_instance(self);
        self_.engine.borrow().clone()
    }

    fn path(&self) -> Option<String> {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }
}
