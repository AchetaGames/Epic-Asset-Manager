use crate::ui::widgets::download_manager::docker::Docker;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::{debug, error};
use regex::Regex;
use std::collections::HashMap;
use std::thread;
use version_compare::Cmp;

#[derive(Debug, Clone)]
pub enum Msg {
    EngineVersions(HashMap<String, Vec<String>>),
    Error(String),
    ManifestSize(u64),
}

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use gtk4::glib::{ParamSpec, ParamSpecString};
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/docker_download.ui")]
    pub struct DockerEngineDownload {
        #[template_child]
        pub details: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_label: TemplateChild<gtk4::Label>,
        pub details_group: gtk4::SizeGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub actions: gio::SimpleActionGroup,
        pub docker_versions: RefCell<Option<HashMap<String, Vec<String>>>>,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub settings: gio::Settings,
        selected: RefCell<Option<String>>,
        download_size: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DockerEngineDownload {
        const NAME: &'static str = "DockerEngineDownload";
        type Type = super::DockerEngineDownload;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                details: TemplateChild::default(),
                details_revealer: TemplateChild::default(),
                confirmation_revealer: TemplateChild::default(),
                confirmation_label: TemplateChild::default(),
                details_group: gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                docker_versions: RefCell::new(None),
                sender,
                receiver: RefCell::new(Some(receiver)),
                settings: gio::Settings::new(crate::config::APP_ID),
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

    impl ObjectImpl for DockerEngineDownload {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_messaging();
            obj.setup_actions();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("selected").build(),
                    ParamSpecString::builder("download-size").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
            match pspec.name() {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "selected" => self.selected.borrow().to_value(),
                "download-size" => self.download_size.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for DockerEngineDownload {}

    impl BoxImpl for DockerEngineDownload {}
}

glib::wrapper! {
    pub struct DockerEngineDownload(ObjectSubclass<imp::DockerEngineDownload>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for DockerEngineDownload {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerEngineDownload {
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
        self.update_docker();
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
        self.insert_action_group("docker_download", Some(actions));

        action!(
            actions,
            "install",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.install_engine();
                }
            )
        );
    }

    fn install_engine(&self) {
        let self_ = self.imp();
        if let Some(ver) = self.selected() {
            if let Some(dm) = self_.download_manager.get() {
                dm.download_engine_from_docker(&ver);
                self.show_confirmation("Install initialized, see headerbar for details");
            }
        }
    }

    fn show_confirmation(&self, markup: &str) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand(false);
        self_.confirmation_label.set_markup(markup);
        self_.confirmation_revealer.set_reveal_child(true);
        self_.confirmation_revealer.set_vexpand_set(true);
        self_.confirmation_revealer.set_vexpand(true);
        glib::timeout_add_seconds_local(
            5,
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

    fn show_details(&self) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(true);
        self_.details_revealer.set_vexpand(true);
        self_.confirmation_revealer.set_reveal_child(false);
        self_.confirmation_revealer.set_vexpand_set(false);
        self_.confirmation_revealer.set_vexpand(false);
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        glib::spawn_future_local(clone!(
            #[weak(rename_to=docker)]
            self,
            #[upgrade_or_panic]
            async move {
                while let Ok(response) = receiver.recv().await {
                    docker.update(response);
                }
            }
        ));
    }

    fn type_selected(&self, check: &gtk4::CheckButton, combo: &gtk4::ComboBoxText) {
        let self_ = self.imp();
        if let Some(selected) = combo.active_id() {
            if let Some(ver) = &*self_.docker_versions.borrow() {
                if let Some(v) = ver.get(selected.as_str()) {
                    for label in v {
                        if (label.contains("slim") && !check.is_active())
                            || (!label.contains("slim") && check.is_active())
                        {
                            self.set_property("selected", label);
                            self.docker_manifest();
                            return;
                        }
                    }
                }
            }
        };
    }

    pub fn update_docker(&self) {
        debug!("Trying to query docker API for images");
        #[cfg(target_os = "linux")]
        {
            let self_ = self.imp();
            if let Some(window) = self_.window.get() {
                let win_ = window.imp();
                (*win_.model.borrow().dclient.borrow())
                    .as_ref()
                    .map_or_else(
                        || {
                            self_.docker_versions.replace(None);
                            self.add_engine();
                        },
                        |dclient| {
                            let client = dclient.clone();
                            let sender = self_.sender.clone();
                            thread::spawn(move || {
                                let re = Regex::new(r"dev-(?:slim-)?(\d\.\d+.\d+)").unwrap();
                                let mut result: HashMap<String, Vec<String>> = HashMap::new();

                                match client.get_tags("epicgames/unreal-engine", None) {
                                    Ok(tags) => {
                                        for tag in tags {
                                            if re.is_match(&tag) {
                                                for cap in re.captures_iter(&tag) {
                                                    #[allow(clippy::option_if_let_else)]
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
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to get tags: {e:?}");
                                        sender
                                            .send_blocking(Msg::Error(format!(
                                                "Failed to get tags: {e:?}"
                                            )))
                                            .unwrap();
                                    }
                                }

                                sender.send_blocking(Msg::EngineVersions(result)).unwrap();
                            });
                        },
                    );
            }
        }
    }

    pub fn add_engine(&self) {
        let self_ = self.imp();
        #[cfg(target_os = "linux")]
        {
            // remove old details
            while let Some(el) = self_.details.first_child() {
                self_.details.remove(&el);
            }
            (*self_.docker_versions.borrow()).as_ref().map_or_else(|| {
                let label = gtk4::Label::builder()
                    .use_markup(true)
                    .css_classes(["heading"])
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(8)
                    .margin_bottom(8)
                    .label("Please configure github token in <a href=\"preferences\">Preferences</a>")
                    .build();
                label.connect_activate_link(clone!(#[weak(rename_to=details)] self, #[upgrade_or] glib::Propagation::Stop, move |_, uri| {
                    details.open_preferences(uri);
                    glib::Propagation::Stop
                }));

                self_.details.append(&label);
                get_action!(self_.actions, @install).set_enabled(false);
            }, |versions| {
                let combo = gtk4::ComboBoxText::new();
                self_
                    .details
                    .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                        "Available Versions:",
                        &combo,
                    ));

                let check = gtk4::CheckButton::builder() // Maybe use GtkSwitch instead?
                    .active(true)
                    .hexpand(true)
                    .build();
                let row = gtk4::ComboBoxText::new();
                self_
                    .details
                    .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                        "Include Template Projects and Debug symbols",
                        &check,
                    ));

                combo.connect_changed(
                    clone!(#[weak(rename_to=detail)] self, #[weak] check, move |c| {
                        detail.version_selected(c, &check);
                    }),
                );

                check.connect_toggled(
                    clone!(#[weak(rename_to=detail)] self, #[weak] combo, move |c| {
                        detail.type_selected(c, &combo);
                    }),
                );

                let mut version: Vec<&String> = versions.keys().collect();
                version.sort_by(|a, b| version_compare::compare(b, a).map_or(std::cmp::Ordering::Equal, |cmp| match cmp {
                                   Cmp::Eq | Cmp::Le | Cmp::Ge => std::cmp::Ordering::Equal,
                                   Cmp::Ne | Cmp::Lt => std::cmp::Ordering::Less,
                                   Cmp::Gt => std::cmp::Ordering::Greater,
                               }));

                for ver in version {
                    combo.append(Some(ver), ver);
                    if combo.active_id().is_none() {
                        combo.set_active_id(Some(ver));
                    }
                }

                // TODO: Switch to create_info_row()
                let size_label = gtk4::Label::builder()
                    .name("size_label")
                    .label("unknown")
                    .build();
                size_label
                    .bind_property("label", self, "download-size")
                    .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
                    .build();

                self_
                    .details
                    .append(&crate::window::EpicAssetManagerWindow::create_widget_row(
                        "Download Size:",
                        &size_label,
                    ));
            });
        }
    }

    fn open_preferences(&self, uri: &str) {
        let self_ = self.imp();
        if uri.eq("preferences") {
            if let Some(w) = self_.window.get() {
                let pref = w.show_preferences();
                pref.switch_to_tab("github");
            }
        };
    }

    fn version_selected(&self, combo: &gtk4::ComboBoxText, check: &gtk4::CheckButton) {
        let self_ = self.imp();
        check.set_sensitive(false);
        if let Some(selected) = combo.active_id() {
            if let Some(ver) = &*self_.docker_versions.borrow() {
                if let Some(v) = ver.get(selected.as_str()) {
                    check.set_active(true);
                    for label in v {
                        if label.contains("slim") {
                            check.set_sensitive(true);
                        } else {
                            self.set_property("selected", label);
                            self.docker_manifest();
                        }
                    }
                }
            }
        };
    }

    pub fn update(&self, msg: Msg) {
        let self_ = self.imp();
        match msg {
            Msg::EngineVersions(ver) => {
                if let Some(w) = self_.window.get() {
                    w.clear_notification("ghcr authentication");
                }
                self.updated_docker_versions(&ver);
            }
            Msg::ManifestSize(size) => {
                let byte = byte_unit::Byte::from_u64(size)
                    .get_appropriate_unit(byte_unit::UnitType::Decimal);
                self_.settings.strv("unreal-engine-directories").first().map_or_else(|| if let Some(w) = self_.window.get() {
                                  w.add_notification("missing engine config", "Unable to install engine missing Unreal Engine Directories configuration", gtk4::MessageType::Error);
                                  get_action!(self_.actions, @install).set_enabled(false);
                              }, |p| {
                              let mut path = std::path::Path::new(p.as_str());
                              while !path.exists() {
                                          path = match path.parent() {
                                                      None => break,
                                                      Some(p) => p,
                                                  }
                                      }
                              if fs2::available_space(path).unwrap_or_default() < size {
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
                          });
                self.set_property("download-size", Some(format!("{byte:.2}")));
            }
            Msg::Error(_error) => {
                if let Some(w) = self_.window.get() {
                    w.add_notification("ghcr authentication", "Unable to authenticate to ghcr please check your setup(did you link with Epic Account?)", gtk4::MessageType::Error);
                    get_action!(self_.actions, @install).set_enabled(false);
                }
            }
        };
    }

    fn updated_docker_versions(&self, versions: &HashMap<String, Vec<String>>) {
        let self_ = self.imp();
        self_.docker_versions.replace(Some(versions.clone()));
        self.add_engine();
    }

    pub fn selected(&self) -> Option<String> {
        self.property("selected")
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
                                sender.send_blocking(Msg::ManifestSize(size)).unwrap();
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
}
