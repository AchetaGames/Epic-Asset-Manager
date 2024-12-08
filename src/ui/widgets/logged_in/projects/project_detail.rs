use crate::models::project_data::Uproject;
use crate::schema::unreal_project_latest_engine;
use crate::ui::widgets::button_cust::ButtonEpic;
use crate::ui::widgets::logged_in::engines::UnrealEngine;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*, ComboBoxText};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::debug;
use std::path::PathBuf;

pub mod imp {
    use super::*;
    use crate::models::project_data::Uproject;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString, ParamSpecUInt};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

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
        #[template_child]
        pub details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub logs: TemplateChild<crate::ui::widgets::logged_in::logs::EpicLogs>,
        #[template_child]
        pub plugins: TemplateChild<crate::ui::widgets::logged_in::plugins::EpicPlugins>,
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
                details_revealer: TemplateChild::default(),
                confirmation_revealer: TemplateChild::default(),
                logs: TemplateChild::default(),
                plugins: TemplateChild::default(),
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
                    ParamSpecBoolean::builder("expanded").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecUInt::builder("position")
                        .minimum(0)
                        .default_value(0)
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "position" => self.position.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
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
        glib::Object::new()
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("project_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.collapse();
                }
            )
        );

        action!(
            actions,
            "launch_project",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.launch_engine();
                }
            )
        );
    }

    fn open_dir(&self) {
        if let Some(p) = self.path() {
            debug!("Trying to open {}", p);
            let ctx = glib::MainContext::default();
            ctx.spawn_local(async move {
                crate::tools::open_directory(&p).await;
            });
        }
    }

    fn launch_engine(&self) {
        let path = self.path().unwrap();
        let engine = self.engine();
        if let Some(eng) = engine {
            if let Some(p) = eng.get_engine_binary_path() {
                let db = crate::models::database::connection();
                if let Ok(mut conn) = db.get() {
                    diesel::replace_into(unreal_project_latest_engine::table)
                        .values((
                            unreal_project_latest_engine::project.eq(path.clone()),
                            unreal_project_latest_engine::engine.eq(eng.path),
                        ))
                        .execute(&mut conn)
                        .expect("Unable to insert last engine to the DB");
                };
                let context = gio::AppLaunchContext::new();
                context.setenv("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2");
                let ctx = glib::MainContext::default();
                ctx.spawn_local(async move {
                    let app = gio::AppInfo::create_from_commandline(
                        if ashpd::is_sandboxed().await {
                            format!(
                                "flatpak-spawn --env='GLIBC_TUNABLES=glibc.rtld.dynamic_sort=2' --host \"{}\" \"{}\"",
                                p.to_str().unwrap(),
                                path
                            )
                        } else {
                            format!("\"{p:?}\" \"{path}\"")
                        },
                        Some("Unreal Engine"),
                        gio::AppInfoCreateFlags::NONE,
                    ).unwrap();
                    app.launch(&[], Some(&context)).expect("Failed to launch application");
                });
            }
        };
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand(false);
        self_.confirmation_revealer.set_reveal_child(true);
        self_.confirmation_revealer.set_vexpand_set(true);
        glib::timeout_add_seconds_local(
            2,
            clone!(
                #[weak(rename_to=obj)]
                self,
                #[upgrade_or_panic]
                move || {
                    obj.show_details();
                    glib::ControlFlow::Break
                }
            ),
        );
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self_.logs.set_window(window);
    }

    fn set_launch_enabled(&self, enabled: bool) {
        let self_ = self.imp();
        get_action!(self_.actions, @launch_project).set_enabled(enabled);
    }

    fn show_details(&self) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand(true);
        self_.confirmation_revealer.set_reveal_child(false);
        self_.confirmation_revealer.set_vexpand_set(false);
    }

    pub fn set_project(&self, project: &Uproject, path: Option<String>) {
        let self_ = self.imp();
        self.show_details();
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
        self_.logs.clear();

        let pathbuf = PathBuf::from(path.unwrap());
        self_.title.set_markup(&format!(
            "<b><u><big>{}</big></u></b>",
            pathbuf.file_stem().unwrap().to_str().unwrap()
        ));

        let parent = pathbuf.parent().unwrap();
        if let Some(p) = parent.to_str() {
            self_.logs.add_path(p);
        }

        // Engine
        let combo = ComboBoxText::new();
        let associated = self.associated_engine(project);
        self.set_launch_enabled(false);
        let db = crate::models::database::connection();
        let mut last_engine: Option<String> = None;
        if let Ok(mut conn) = db.get() {
            let engines: Result<String, diesel::result::Error> =
                unreal_project_latest_engine::table
                    .filter(unreal_project_latest_engine::project.eq(&self.path().unwrap()))
                    .select(unreal_project_latest_engine::engine)
                    .first(&mut conn);
            if let Ok(last) = engines {
                last_engine = Some(last);
            }
        };

        self.populate_engines(&combo, &associated, &mut last_engine);

        combo.connect_changed(clone!(
            #[weak(rename_to=detail)]
            self,
            move |c| {
                detail.engine_selected(c);
            }
        ));
        if let Some(engine) = associated {
            combo.set_active_id(Some(&engine.path));
        } else if let Some(last) = last_engine {
            combo.set_active_id(Some(&last));
        };
        // TODO: Change the project config based on the engine selected

        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                "Engine:", &combo,
            ));

        // Engine Association
        let text = &project.engine_association;
        let text = format!("Engine Association: {text}");
        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_info_row(
                &text,
            ));

        // Path
        let text = parent.to_str().unwrap();
        let text = format!("Path: {text}");
        let button = gtk4::Button::with_icon_and_label("folder-open-symbolic", "Open");
        button.connect_clicked(clone!(
            #[weak(rename_to=project)]
            self,
            move |_| {
                project.open_dir();
            }
        ));
        self_
            .details
            .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                &text, &button,
            ));
    }

    fn populate_engines(
        &self,
        combo: &ComboBoxText,
        associated: &Option<UnrealEngine>,
        last_engine: &mut Option<String>,
    ) {
        let mut paths = std::collections::HashSet::new();
        for engine in self.available_engines() {
            if !paths.insert(engine.path.clone()) {
                continue;
            };
            combo.append(
                Some(&engine.path),
                &format!(
                    "{}{}",
                    engine.version.format(),
                    match associated {
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
    }

    fn engine_selected(&self, combo: &ComboBoxText) {
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
