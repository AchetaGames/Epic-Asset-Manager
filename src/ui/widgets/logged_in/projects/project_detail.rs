use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use std::path::PathBuf;

use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::logged_in::engines::UnrealEngine;

pub(crate) mod imp {
    use std::cell::RefCell;

    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString, ParamSpecUInt};
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
        pub details: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub actions: gio::SimpleActionGroup,
        path: RefCell<Option<String>>,
        pub uproject: RefCell<Option<Uproject>>,
        pub engine: RefCell<Option<UnrealEngine>>,
        pub settings: gio::Settings,
        pub details_group: gtk4::SizeGroup,
        position: RefCell<u32>,
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
                title: TemplateChild::default(),
                window: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                path: RefCell::new(None),
                uproject: RefCell::new(None),
                engine: RefCell::new(None),
                settings: gio::Settings::new(crate::config::APP_ID),
                details_group: gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal),
                position: RefCell::new(0),
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
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::new(
                        "expanded",
                        "expanded",
                        "Is expanded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new("path", "Path", "Path", None, glib::ParamFlags::READWRITE),
                    ParamSpecUInt::new(
                        "position",
                        "position",
                        "item_position",
                        0,
                        u32::MAX,
                        0,
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
                "position" => {
                    let position = value.get().unwrap();
                    self.position.replace(position);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "position" => self.position.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
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
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("project_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.collapse();
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
                    if ashpd::is_sandboxed() {
                        format!(
                            "flatpak-spawn --host \"{}\" \"{}\"",
                            p.to_str().unwrap(),
                            path
                        )
                    } else {
                        format!("\"{:?}\" \"{}\"", p, path)
                    },
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
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
    }

    fn set_launch_enabled(&self, enabled: bool) {
        let self_ = self.imp();
        get_action!(self_.actions, @launch_project).set_enabled(enabled);
    }

    pub fn set_project(
        &self,
        project: &crate::models::project_data::Uproject,
        path: Option<String>,
    ) {
        let self_ = self.imp();
        self.set_property("path", &path);
        if !self.is_expanded() {
            self.set_property("expanded", true);
        }
        self_.uproject.replace(Some(project.clone()));
        while let Some(el) = self_.details.first_child() {
            self_.details.remove(&el);
        }
        if path.is_none() {
            return;
        }

        let pathbuf = PathBuf::from(path.unwrap());
        self_.title.set_markup(&format!(
            "<b><u><big>{}</big></u></b>",
            pathbuf.file_stem().unwrap().to_str().unwrap()
        ));

        // Engine
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
            detail.engine_selected(c);
        }));
        if let Some(engine) = associated {
            combo.set_active_id(Some(&engine.path));
        } else if let Some(last) = last_engine {
            combo.set_active_id(Some(&last));
        };
        // TODO: Change the project config based on the engine selected

        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                "Engine",
                &combo,
                &self_.details_group,
            ));

        // Path
        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                "Path",
                &gtk4::Label::new(Some(pathbuf.parent().unwrap().to_str().unwrap())),
                &self_.details_group,
            ));

        // Engine Association
        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                "Engine Association",
                &gtk4::Label::new(Some(&project.engine_association)),
                &self_.details_group,
            ));
    }

    fn engine_selected(&self, combo: &gtk4::ComboBoxText) {
        if let Some(eng) = combo.active_id() {
            let self_ = self.imp();
            for engine in self.available_engines() {
                if engine.path.eq(&eng) {
                    self.set_launch_enabled(true);
                    self_.engine.replace(Some(engine));
                }
            }
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

    fn available_engines(&self) -> Vec<UnrealEngine> {
        let self_ = self.imp();
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l_ = w_.logged_in_stack.imp();
            return l_.engines.engines();
        }
        Vec::new()
    }

    fn is_expanded(&self) -> bool {
        self.property("expanded")
    }

    pub fn uproject(&self) -> Option<Uproject> {
        let self_ = self.imp();
        self_.uproject.borrow().clone()
    }

    fn engine(&self) -> Option<UnrealEngine> {
        let self_ = self.imp();
        self_.engine.borrow().clone()
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn position(&self) -> u32 {
        self.property("position")
    }

    pub fn collapse(&self) {
        let self_ = self.imp();
        self.set_property("expanded", false);
        if let Some(w) = self_.window.get() {
            let w_ = w.imp();
            let l = w_.logged_in_stack.clone();
            let l_ = l.imp();
            let p = l_.projects.imp();
            if let Some(m) = p.projects_grid.model() {
                m.unselect_item(self.position());
            }
        }
    }
}
