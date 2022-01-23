use crate::ui::widgets::download_manager::docker::Docker;
use adw::prelude::ActionRowExt;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsString;
use std::str::FromStr;
use std::thread;
use version_compare::Cmp;

#[derive(Debug, Clone)]
pub enum DockerMsg {
    DockerEngineVersions(HashMap<String, Vec<String>>),
    DockerManifestSize(u64),
}

pub(crate) mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

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
        #[template_child]
        pub install_button: TemplateChild<gtk4::Button>,
        #[template_child]
        pub details: TemplateChild<gtk4::Box>,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub actions: gio::SimpleActionGroup,
        pub settings: gio::Settings,
        pub data: RefCell<Option<crate::models::engine_data::EngineData>>,
        pub sender: gtk4::glib::Sender<super::DockerMsg>,
        pub receiver: RefCell<Option<gtk4::glib::Receiver<super::DockerMsg>>>,
        pub docker_versions: RefCell<Option<HashMap<String, Vec<String>>>>,
        selected: RefCell<Option<String>>,
        download_size: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineDetails {
        const NAME: &'static str = "EpicEngineDetails";
        type Type = super::EpicEngineDetails;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = gtk4::glib::MainContext::channel(gtk4::glib::PRIORITY_DEFAULT);
            Self {
                expanded: RefCell::new(false),
                detail_slider: TemplateChild::default(),
                title: TemplateChild::default(),
                launch_button: TemplateChild::default(),
                install_button: TemplateChild::default(),
                details: TemplateChild::default(),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                settings: gio::Settings::new(crate::config::APP_ID),
                sender,
                receiver: RefCell::new(Some(receiver)),
                data: RefCell::new(None),
                docker_versions: RefCell::new(None),
                selected: RefCell::new(None),
                download_size: RefCell::new(None),
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
            obj.setup_messaging();
        }

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
                    ParamSpecString::new(
                        "download-size",
                        "Download Size",
                        "Download Size",
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
                "selected" => {
                    let selected = value.get().unwrap();
                    self.selected.replace(selected);
                }
                "download-size" => {
                    let size = value.get().unwrap();
                    self.download_size.replace(size);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "expanded" => self.expanded.borrow().to_value(),
                "selected" => self.selected.borrow().to_value(),
                "download-size" => self.download_size.borrow().to_value(),
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
                details.set_property("expanded", false);
            })
        );

        action!(
            actions,
            "install",
            clone!(@weak self as details => move |_, _| {
                let self_ = details.imp();
                if let Some(ver) = details.selected() {
                    if let Some(dm) = self_.download_manager.get() {
                        dm.download_engine_from_docker(&ver);
                    }
                }
            })
        );

        action!(
            self_.actions,
            "launch",
            clone!(@weak self as engines => move |_, _| {
                engines.launch_engine()
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
                    // let ctx = glib::MainContext::default();
                    // ctx.spawn_local(clone!(@weak engines => async move {
                    //     let connection = zbus::Connection::session().await.unwrap();
                    //     let proxy = FlatpakProxy::new(&connection).await.unwrap();
                    //     let mut paths = p.to_str().unwrap().split("/").collect::<Vec<&str>>();
                    //     paths.pop().unwrap();
                    //     let res = proxy
                    //         .spawn(
                    //             paths.join("/"),
                    //             &vec![p.to_str().unwrap()],
                    //             HashMap::new(),
                    //             [("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2")]
                    //              .iter().cloned().collect(),
                    //             enumflags2::BitFlags::empty(),
                    //             SpawnOptions::default(),
                    //         ).await
                    //         .unwrap();
                    //     debug!("Launched process: {}", res);
                    // }));

                    let context = gtk4::gio::AppLaunchContext::new();
                    context.setenv("GLIBC_TUNABLES", "glibc.rtld.dynamic_sort=2");
                    let app = gtk4::gio::AppInfo::create_from_commandline(
                        if ashpd::is_sandboxed() {
                            format!("flatpak-spawn --host \"{}\"", p.to_str().unwrap())
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
    }

    pub fn set_data(&self, data: &crate::models::engine_data::EngineData) {
        let self_ = self.imp();
        // remove old details
        while let Some(el) = self_.details.first_child() {
            self_.details.remove(&el);
        }
        if let Some(title) = &data.version() {
            self_
                .title
                .set_markup(&format!("<b><u><big>{}</big></u></b>", title));
        }
        self_.launch_button.set_visible(true);
        self_.install_button.set_visible(false);
        self_.data.replace(Some(data.clone()));
        let size_group_labels = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
        let size_group_prefix = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);

        if let Some(path) = &data.path() {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Path").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(path)
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details.append(&row);
        }

        if let Some(branch) = &data.branch() {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Branch").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            let label = gtk4::Label::builder()
                .label(branch)
                .wrap(true)
                .xalign(0.0)
                .build();
            size_group_labels.add_widget(&label);
            row.add_suffix(&label);
            self_.details.append(&row);
        }

        if data.needs_update() {
            let row = adw::ActionRow::builder().activatable(true).build();
            let title = gtk4::Label::builder().label("Needs update").build();
            size_group_prefix.add_widget(&title);
            row.add_prefix(&title);
            self_.details.append(&row);
        }
    }

    pub fn add_engine(&self) {
        let self_ = self.imp();
        #[cfg(target_os = "linux")]
        {
            self_.data.replace(None);
            self_.launch_button.set_visible(false);
            self_.install_button.set_visible(true);
            self_
                .title
                .set_markup("<b><u><big>Add Engine</big></u></b>");

            // remove old details
            while let Some(el) = self_.details.first_child() {
                self_.details.remove(&el);
            }
            if let Some(versions) = &*self_.docker_versions.borrow() {
                let size_group_labels = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);
                let size_group_prefix = gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal);

                let combo = gtk4::ComboBoxText::new();
                combo.set_hexpand(true);
                let row = adw::ActionRow::builder().activatable(true).build();
                let title = gtk4::Label::builder().label("Available Versions").build();
                size_group_prefix.add_widget(&title);
                row.add_prefix(&title);
                size_group_labels.add_widget(&combo);
                row.add_suffix(&combo);
                self_.details.append(&row);

                let row = adw::ActionRow::builder().activatable(true).build();
                row.set_tooltip_markup(Some(
                    "Include <b>Template Projects</b> and <b>Debug symbols</b>?",
                ));
                let title = gtk4::Label::builder().label("Additional Content").build();
                let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
                b.append(&title);
                let info = gtk4::Image::from_icon_name("dialog-information-symbolic");
                b.append(&info);
                size_group_prefix.add_widget(&b);
                row.add_prefix(&b);
                let check = gtk4::CheckButton::builder()
                    .active(true)
                    .hexpand(true)
                    .build();
                size_group_labels.add_widget(&check);
                row.add_suffix(&check);
                self_.details.append(&row);
                combo.connect_changed(
                    clone!(@weak self as detail, @weak check as check => move |c| {
                        let self_ = detail.imp();
                        check.set_sensitive(false);
                        if let Some(selected) = c.active_id() {
                            if let Some(ver) = &*self_.docker_versions.borrow() {
                                if let Some(v) = ver.get(selected.as_str()) {
                                    check.set_active(true);
                                    for label in v {
                                        if label.contains("slim") {
                                            check.set_sensitive(true);
                                        } else {
                                            detail.set_property("selected", label);
                                            detail.docker_manifest();
                                        }
                                    }
                                }
                            }
                        };
                    }),
                );

                check.connect_toggled(
                    clone!(@weak self as detail, @weak combo as combo => move |c| {
                        let self_ = detail.imp();
                        if let Some(selected) = combo.active_id() {
                            if let Some(ver) = &*self_.docker_versions.borrow() {
                                if let Some(v) = ver.get(selected.as_str()) {
                                    for label in v {
                                        if (label.contains("slim") && !c.is_active())
                                            || (!label.contains("slim") && c.is_active())
                                        {
                                            detail.set_property("selected", label);
                                            detail.docker_manifest();
                                            return;
                                        }
                                    }
                                }
                            }
                        };
                    }),
                );

                let mut version: Vec<&String> = versions.keys().into_iter().collect();
                version.sort_by(|a, b| match version_compare::compare(b, a) {
                    Ok(cmp) => match cmp {
                        Cmp::Eq | Cmp::Le | Cmp::Ge => std::cmp::Ordering::Equal,
                        Cmp::Ne | Cmp::Lt => std::cmp::Ordering::Less,
                        Cmp::Gt => std::cmp::Ordering::Greater,
                    },
                    Err(_) => std::cmp::Ordering::Equal,
                });

                for ver in version {
                    combo.append(Some(ver), ver);
                    if combo.active_id().is_none() {
                        combo.set_active_id(Some(ver));
                    }
                }

                let row = adw::ActionRow::builder()
                    .activatable(true)
                    .name("size_row")
                    .build();
                let title = gtk4::Label::builder().label("Download Size").build();
                let size_label = gtk4::Label::builder()
                    .name("size_label")
                    .halign(gtk4::Align::Start)
                    .hexpand(true)
                    .label("unknown")
                    .build();
                size_label
                    .bind_property("label", self, "download-size")
                    .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
                    .build();
                size_group_prefix.add_widget(&title);
                row.add_prefix(&title);
                size_group_labels.add_widget(&size_label);
                row.add_suffix(&size_label);
                self_.details.append(&row);
            } else {
                let label = gtk4::Label::builder()
                    .halign(gtk4::Align::Center)
                    .hexpand(true)
                    .use_markup(true)
                    .label("<b>Please configure github token in <a href=\"preferences\">Preferences</a></b>")
                    .build();
                label.connect_activate_link(clone!(@weak self as details => @default-return gtk4::Inhibit(true), move |_, uri| {
                    let self_ = details.imp();
                    if uri.eq("preferences") {
                        if let Some(w) = self_.window.get() {
                            let pref = w.show_preferences();
                            pref.switch_to_tab("github");
                        }
                    };
                    gtk4::Inhibit(true)
                }));

                self_.details.append(&label);
                get_action!(self_.actions, @install).set_enabled(false);
            }
        }
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_ = self.imp();
        // Do not run this twice
        if self_.window.get().is_some() {
            return;
        }

        self_.window.set(window.clone()).unwrap();
        self.update_docker();
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as engine => @default-panic, move |msg| {
                engine.update(msg);
                glib::Continue(true)
            }),
        );
    }

    pub fn update(&self, msg: DockerMsg) {
        let self_ = self.imp();
        match msg {
            DockerMsg::DockerEngineVersions(ver) => {
                self.updated_docker_versions(&ver);
            }
            DockerMsg::DockerManifestSize(size) => {
                let byte = byte_unit::Byte::from_bytes(size as u128).get_appropriate_unit(false);
                match self_.settings.strv("unreal-engine-directories").get(0) {
                    None => {
                        if let Some(w) = self_.window.get() {
                            w.add_notification("missing engine config", "Unable to install engine missing Unreal Engine Directories configuration", gtk4::MessageType::Error);
                            get_action!(self_.actions, @install).set_enabled(false);
                        }
                    }
                    Some(p) => {
                        if fs2::available_space(std::path::Path::new(p)).unwrap_or_default() < size
                        {
                            if let Some(w) = self_.window.get() {
                                w.add_notification("no space left on device engine", "Not enough space left in the Engine directory for install, please choose a different one.", gtk4::MessageType::Error);
                            }
                            get_action!(self_.actions, @install).set_enabled(false);
                        } else {
                            if let Some(w) = self_.window.get() {
                                w.clear_notification("no space left on device engine");
                            }
                            get_action!(self_.actions, @install).set_enabled(true);
                        }
                    }
                };
                self.set_property("download-size", Some(byte.format(1)));
            }
        };
    }

    fn updated_docker_versions(&self, versions: &HashMap<String, Vec<String>>) {
        let self_ = self.imp();
        self_.docker_versions.replace(Some(versions.clone()));
        if self_.data.borrow().is_none() {
            self.add_engine();
        }
    }

    #[cfg(target_os = "linux")]
    pub fn docker_manifest(&self) {
        let self_ = self.imp();
        get_action!(self_.actions, @install).set_enabled(true);
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            if let Some(dclient) = &*win_.model.borrow().dclient.borrow() {
                let client = dclient.clone();
                let version = self.selected();
                if version.is_none() {
                    return;
                }
                let version = version.unwrap();
                let sender = self_.sender.clone();
                thread::spawn(move || {
                    match client.get_manifest("epicgames/unreal-engine", &version) {
                        Ok(manifest) => match manifest.download_size() {
                            Ok(size) => {
                                sender.send(DockerMsg::DockerManifestSize(size)).unwrap();
                            }
                            Err(e) => {
                                error!("Unable to get manifest size: {:?}", e);
                            }
                        },
                        Err(e) => {
                            error!("Unable to get docker manifest {:?}", e);
                        }
                    };
                });
            }
        }
    }

    pub fn update_docker(&self) {
        debug!("Trying to query docker API for images");
        #[cfg(target_os = "linux")]
        {
            let self_ = self.imp();
            if let Some(window) = self_.window.get() {
                let win_ = window.imp();
                if let Some(dclient) = &*win_.model.borrow().dclient.borrow() {
                    let client = dclient.clone();
                    let sender = self_.sender.clone();
                    thread::spawn(move || {
                        let re = Regex::new(r"dev-(?:slim-)?(\d\.\d+.\d+)").unwrap();
                        let mut result: HashMap<String, Vec<String>> = HashMap::new();

                        client
                            .get_tags("epicgames/unreal-engine", None)
                            .unwrap()
                            .into_iter()
                            .for_each(|tag| {
                                if re.is_match(&tag) {
                                    for cap in re.captures_iter(&tag) {
                                        match result.get_mut(&cap[1]) {
                                            None => {
                                                result.insert(
                                                    cap[1].to_string(),
                                                    vec![tag.to_string()],
                                                );
                                            }
                                            Some(v) => {
                                                v.push(tag.to_string());
                                            }
                                        }
                                    }
                                }
                            });
                        sender
                            .send(DockerMsg::DockerEngineVersions(result))
                            .unwrap();
                    });
                } else {
                    self_.docker_versions.replace(None);
                    if self_.data.borrow().is_none() {
                        self.add_engine();
                    }
                };
            }
        }
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

    fn path(&self) -> Option<String> {
        let self_ = self.imp();
        if let Some(d) = &*self_.data.borrow() {
            return d.path();
        }
        None
    }

    pub fn selected(&self) -> Option<String> {
        self.property("selected")
    }

    // fn is_expanded(&self) -> bool {
    //     if let Ok(value) = self.property("expanded") {
    //         if let Ok(id_opt) = value.get::<bool>() {
    //             return id_opt;
    //         }
    //     };
    //     false
    // }
}
