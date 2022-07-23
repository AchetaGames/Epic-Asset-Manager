use crate::ui::widgets::button_cust::ButtonEpic;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::action;
use std::ffi::OsString;
use std::str::FromStr;

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString, ParamSpecUInt};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engine_detail.ui")]
    pub struct EpicEngineDetails {
        pub expanded: RefCell<bool>,
        #[template_child]
        pub title: TemplateChild<gtk4::Label>,
        #[template_child]
        pub launch_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub details: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub logs: TemplateChild<crate::ui::widgets::logged_in::logs::EpicLogs>,
        #[template_child]
        pub logs_row: TemplateChild<adw::ExpanderRow>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub actions: gio::SimpleActionGroup,
        pub settings: gio::Settings,
        pub data: RefCell<Option<crate::models::engine_data::EngineData>>,
        selected: RefCell<Option<String>>,
        position: RefCell<u32>,
        pub details_group: gtk4::SizeGroup,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineDetails {
        const NAME: &'static str = "EpicEngineDetails";
        type Type = super::EpicEngineDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                expanded: RefCell::new(false),
                title: TemplateChild::default(),
                launch_button: TemplateChild::default(),
                details: TemplateChild::default(),
                details_revealer: TemplateChild::default(),
                confirmation_revealer: TemplateChild::default(),
                confirmation_label: TemplateChild::default(),
                logs: TemplateChild::default(),
                logs_row: TemplateChild::default(),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                data: RefCell::new(None),
                selected: RefCell::new(None),
                position: RefCell::new(0),
                details_group: gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal),
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
                    ParamSpecString::new(
                        "selected",
                        "Selected",
                        "Selected",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
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
                "selected" => {
                    let selected = value.get().unwrap();
                    self.selected.replace(selected);
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
                "selected" => self.selected.borrow().to_value(),
                "position" => self.position.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
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

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("engine_details", Some(actions));

        action!(
            actions,
            "close",
            clone!(@weak self as details => move |_, _| {
                details.collapse();
            })
        );

        action!(
            self_.actions,
            "launch",
            clone!(@weak self as engines => move |_, _| {
                engines.launch_engine();
            })
        );
    }

    fn launch_engine(&self) {
        let path = self.path();
        if let Some(path) = path {
            match Self::get_engine_binary_path(&path) {
                None => {
                    warn!("No path");
                }
                Some(p) => {
                    let context = gtk4::gio::AppLaunchContext::new();
                    context.setenv("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2");
                    let app = gtk4::gio::AppInfo::create_from_commandline(
                        if ashpd::is_sandboxed() {
                            format!("flatpak-spawn --env='GLIBC_TUNABLES=glibc.rtld.dynamic_sort=2' --host \"{}\"", p.to_str().unwrap())
                        } else {
                            format!("\"{}\"", p.to_str().unwrap())
                        },
                        Some("Unreal Engine"),
                        gtk4::gio::AppInfoCreateFlags::NONE,
                    )
                    .unwrap();
                    app.launch(&[], Some(&context))
                        .expect("Failed to launch application");
                }
            }
        };
        self.show_confirmation("<b><big>Engine Launched</big></b>");
    }

    fn show_confirmation(&self, markup: &str) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand(false);
        self_.confirmation_label.set_markup(markup);
        self_.confirmation_revealer.set_reveal_child(true);
        self_.confirmation_revealer.set_vexpand_set(true);
        glib::timeout_add_seconds_local(
            2,
            clone!(@weak self as obj => @default-panic, move || {
                obj.show_details();
                glib::Continue(false)
            }),
        );
    }

    fn show_details(&self) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand(true);
        self_.confirmation_revealer.set_reveal_child(false);
        self_.confirmation_revealer.set_vexpand_set(false);
    }

    pub fn set_data(&self, data: &crate::models::engine_data::EngineData) {
        let self_ = self.imp();
        self.show_details();
        // remove old details
        while let Some(el) = self_.details.first_child() {
            self_.details.remove(&el);
        }

        if !self.is_expanded() {
            self.set_property("expanded", true);
        }

        if let Some(title) = &data.version() {
            self_
                .title
                .set_markup(&format!("<b><u><big>{}</big></u></b>", title));
        }
        self_.launch_button.set_visible(true);
        self_.data.replace(Some(data.clone()));
        self_.logs.clear();
        self_.logs_row.set_visible(true);

        // Path
        if let Some(path) = &data.path() {
            self_.logs.add_path(&format!("{}/Engine", &path));
            let path_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            let label = gtk4::Label::new(Some(path));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            path_box.append(&label);
            let button = gtk4::Button::with_icon_and_label("system-file-manager-symbolic", "Open");
            button.connect_clicked(clone!(@weak self as engine => move |_| {
                engine.open_dir();
            }));
            path_box.append(&button);
            self_
                .details
                .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                    "Path",
                    &path_box,
                    &self_.details_group,
                ));
        }

        if let Some(branch) = &data.branch() {
            self_
                .details
                .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                    "Branch",
                    &gtk4::Label::new(Some(branch)),
                    &self_.details_group,
                ));
        }

        if data.needs_update() {
            self_
                .details
                .append(&crate::window::EpicAssetManagerWindow::create_details_row(
                    "Needs update",
                    &gtk4::Label::new(None),
                    &self_.details_group,
                ));
        }
    }

    fn open_dir(&self) {
        if let Some(p) = self.path() {
            debug!("Trying to open {}", p);
            #[cfg(target_os = "linux")]
            {
                if let Ok(dir) = std::fs::File::open(&format!("{}/Engine", p)) {
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(clone!(@weak self as asset_details => async move {
                        ashpd::desktop::open_uri::open_directory(
                            &ashpd::WindowIdentifier::default(),
                            &dir,
                        )
                        .await.unwrap();
                    }));
                };
            };
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.property("expanded")
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

    fn get_engine_binary_path(path: &str) -> Option<OsString> {
        if let Ok(mut p) = std::path::PathBuf::from_str(path) {
            p.push("Engine");
            p.push("Binaries");
            p.push("Linux");
            let mut test = p.clone();
            test.push("UE4Editor");
            if test.exists() {
                let mut result = OsString::new();
                result.push(test.into_os_string());
                return Some(result);
            }
            let mut test = p.clone();
            test.push("UnrealEditor");
            if test.exists() {
                let mut result = OsString::new();
                result.push(test.into_os_string());
                return Some(result);
            }
            error!("Unable to launch the engine");
        };
        None
    }

    pub fn path(&self) -> Option<String> {
        let self_ = self.imp();
        if let Some(d) = &*self_.data.borrow() {
            return d.path();
        }
        None
    }

    pub fn selected(&self) -> Option<String> {
        self.property("selected")
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
            let e = l_.engines.imp();
            if let Some(m) = e.engine_grid.model() {
                m.unselect_item(self.position());
            }
        }
    }
}
