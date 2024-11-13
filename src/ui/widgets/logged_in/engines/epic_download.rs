use crate::tools::epic_web::EpicWeb;
use crate::ui::widgets::download_manager::epic_file::EpicFile;
use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, gio, prelude::*};
use gtk4::{glib, CompositeTemplate};
use gtk_macros::{action, get_action};
use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use tokio::runtime::Builder;
use version_compare::Cmp;

#[derive(Debug, Clone)]
pub enum Msg {
    EULAValid(bool),
    Versions(Vec<Blob>),
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponse {
    pub blobs: Vec<Blob>,
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    pub name: String,
    pub created_at: String,
    pub size: u64,
    pub url: String,
}

pub mod imp {
    use super::*;
    use crate::window::EpicAssetManagerWindow;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/epic_download.ui")]
    pub struct EpicEngineDownload {
        #[template_child]
        pub details: TemplateChild<gtk4::ListBox>,
        #[template_child]
        pub details_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_revealer: TemplateChild<gtk4::Revealer>,
        #[template_child]
        pub confirmation_label: TemplateChild<gtk4::Label>,
        #[template_child]
        pub eula_stack: TemplateChild<gtk4::Stack>,
        #[template_child]
        pub version_selector: TemplateChild<gtk4::ComboBoxText>,
        #[template_child]
        pub versions_row: TemplateChild<gtk4::ListBoxRow>,
        #[template_child]
        pub size_row: TemplateChild<gtk4::ListBoxRow>,
        #[template_child]
        pub size_value: TemplateChild<gtk4::Label>,
        pub details_group: gtk4::SizeGroup,
        pub window: OnceCell<EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        pub actions: gio::SimpleActionGroup,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub engine_versions: RefCell<Option<HashMap<String, Blob>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngineDownload {
        const NAME: &'static str = "EpicEngineDownload";
        type Type = super::EpicEngineDownload;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
            Self {
                details: TemplateChild::default(),
                details_revealer: TemplateChild::default(),
                confirmation_revealer: TemplateChild::default(),
                confirmation_label: TemplateChild::default(),
                eula_stack: TemplateChild::default(),
                version_selector: TemplateChild::default(),
                versions_row: TemplateChild::default(),
                size_row: TemplateChild::default(),
                size_value: TemplateChild::default(),
                details_group: gtk4::SizeGroup::new(gtk4::SizeGroupMode::Horizontal),
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                actions: gio::SimpleActionGroup::new(),
                sender,
                receiver: RefCell::new(Some(receiver)),
                engine_versions: RefCell::new(None),
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

    impl ObjectImpl for EpicEngineDownload {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_messaging();
            obj.setup_actions();
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for EpicEngineDownload {}
    impl BoxImpl for EpicEngineDownload {}
}

glib::wrapper! {
    pub struct EpicEngineDownload(ObjectSubclass<imp::EpicEngineDownload>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEngineDownload {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngineDownload {
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
        self.validate_eula();
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

    fn show_confirmation(&self, markup: &str) {
        let self_ = self.imp();
        self_.details_revealer.set_reveal_child(false);
        self_.details_revealer.set_vexpand(false);
        self_.confirmation_label.set_markup(markup);
        self_.confirmation_revealer.set_reveal_child(true);
        self_.confirmation_revealer.set_vexpand_set(true);
        self_.confirmation_revealer.set_vexpand(true);
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
                while let Ok(msg) = receiver.recv().await {
                    docker.update(msg);
                }
            }
        ));
    }

    pub fn setup_widgets(&self) {
        let self_ = self.imp();
        self_.version_selector.connect_changed(clone!(
            #[weak(rename_to=detail)]
            self,
            move |_| {
                detail.version_selected();
            }
        ));
    }

    pub fn version_selected(&self) {
        let self_ = self.imp();
        if let Some(selected) = self_.version_selector.active_id() {
            if let Some(versions) = &*self_.engine_versions.borrow() {
                if let Some(version) = versions.get(selected.as_str()) {
                    let byte = byte_unit::Byte::from_u64(version.size)
                        .get_appropriate_unit(byte_unit::UnitType::Decimal);
                    self_.size_value.set_label(&format!("{byte:.2}"));
                }
            }
        }
    }

    pub fn setup_actions(&self) {
        let self_ = self.imp();
        let actions = &self_.actions;
        self.insert_action_group("epic_download", Some(actions));

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

        action!(
            actions,
            "revalidate_eula",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.validate_eula();
                }
            )
        );

        action!(
            actions,
            "browser",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.open_eula_browser();
                }
            )
        );

        action!(
            actions,
            "download",
            clone!(
                #[weak(rename_to=details)]
                self,
                move |_, _| {
                    details.open_download_browser();
                }
            )
        );
        get_action!(self_.actions, @install).set_enabled(false);
    }

    pub fn install_engine(&self) {
        let self_ = self.imp();
        if let Some(selected) = self_.version_selector.active_id() {
            if let Some(versions) = &*self_.engine_versions.borrow() {
                if let Some(version) = versions.get(selected.as_str()) {
                    if let Some(dm) = self_.download_manager.get() {
                        dm.download_engine_from_epic(&version.name);
                        self.show_confirmation(
                            "<b><big>Engine Install Initialized</big></b>
<i>See Header Bar for details</i>",
                        );
                    }
                }
            }
        }
    }

    pub fn open_eula_browser(&self) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let (sender, receiver) = async_channel::unbounded::<String>();

            glib::spawn_future_local(async move {
                while let Ok(response) = receiver.recv().await {
                    open_browser(
                        &response,
                        "https%3A%2F%2Fwww.unrealengine.com%2Feulacheck%2Funreal",
                    );
                }
            });

            thread::spawn(move || {
                if let Some(token) = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.game_token())
                {
                    sender.send_blocking(token.code).unwrap();
                }
            });
        }
    }

    pub fn open_download_browser(&self) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let (sender, receiver) = async_channel::unbounded::<String>();

            glib::spawn_future_local(async move {
                while let Ok(response) = receiver.recv().await {
                    open_browser(&response, "https%3A%2F%2Fwww.unrealengine.com%2Flinux");
                }
            });

            thread::spawn(move || {
                if let Some(token) = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.game_token())
                {
                    sender.send_blocking(token.code).unwrap();
                }
            });
        }
    }

    pub fn update(&self, msg: Msg) {
        let self_ = self.imp();
        match msg {
            Msg::EULAValid(validity) => {
                if validity {
                    self_.size_row.set_visible(true);
                    self_.versions_row.set_visible(true);
                    self_.eula_stack.set_visible_child_name("valid");
                    let (sender, receiver) = async_channel::unbounded();

                    glib::spawn_future_local(clone!(
                        #[weak(rename_to=ed)]
                        self,
                        #[upgrade_or_panic]
                        async move {
                            while let Ok(response) = receiver.recv().await {
                                let self_ = ed.imp();
                                let s = self_.sender.clone();
                                s.send_blocking(Msg::Versions(response)).unwrap();
                            }
                        }
                    ));
                    self.get_versions(sender);
                } else {
                    self_.eula_stack.set_visible_child_name("invalid");
                }
            }
            Msg::Versions(versions) => {
                self_.version_selector.remove_all();
                let mut result: HashMap<String, Blob> = HashMap::new();
                for version in versions {
                    let re =
                        Regex::new(r"Linux_Unreal_Engine_(\d\.\d+.\d+)_?(preview-\d+)?").unwrap();
                    if re.is_match(&version.name) {
                        for cap in re.captures_iter(&version.name) {
                            result.insert(
                                cap.get(2).map_or_else(
                                    || cap[1].to_string(),
                                    |suffix| {
                                        format!(
                                            "{} ({})",
                                            cap.get(1).map_or("", |v| v.as_str()),
                                            suffix.as_str()
                                        )
                                    },
                                ),
                                version.clone(),
                            );
                        }
                    }
                }
                self_.engine_versions.replace(Some(result.clone()));
                let mut version: Vec<&String> = result.keys().collect();
                version.sort_by(|a, b| {
                    version_compare::compare(b, a).map_or(
                        std::cmp::Ordering::Equal,
                        |cmp| match cmp {
                            Cmp::Eq | Cmp::Le | Cmp::Ge => std::cmp::Ordering::Equal,
                            Cmp::Ne | Cmp::Lt => std::cmp::Ordering::Less,
                            Cmp::Gt => std::cmp::Ordering::Greater,
                        },
                    )
                });

                for ver in version {
                    self_.version_selector.append(Some(ver), ver);
                    if self_.version_selector.active_id().is_none() {
                        self_.version_selector.set_active_id(Some(ver));
                    }
                }
                get_action!(self_.actions, @install).set_enabled(true);
            }
        }
    }

    fn validate_eula(&self) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            let sender = self_.sender.clone();
            let Some(id) = eg.user_details().account_id else {
                sender.send_blocking(Msg::EULAValid(false)).unwrap();
                return;
            };
            thread::spawn(move || {
                if let Some(token) = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.game_token())
                {
                    let mut web = EpicWeb::new();
                    web.start_session(token.code);
                    sender
                        .send_blocking(Msg::EULAValid(web.validate_eula(&id)))
                        .unwrap();
                };
            });
        }
    }

    pub fn get_versions(&self, sender: async_channel::Sender<Vec<Blob>>) {
        let self_ = self.imp();
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let mut eg = win_.model.borrow().epic_games.borrow().clone();
            thread::spawn(move || {
                if let Some(token) = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(eg.game_token())
                {
                    let mut web = EpicWeb::new();
                    web.start_session(token.code);
                    if let Ok(versions) = web.run_query::<VersionResponse>(
                        "https://www.unrealengine.com/api/blobs/linux".to_string(),
                    ) {
                        sender.send_blocking(versions.blobs).unwrap();
                    };
                }
            });
        }
    }
}

fn open_browser(code: &str, redirect: &str) {
    #[cfg(target_os = "linux")]
    if gio::AppInfo::launch_default_for_uri(
        &format!(
            "https://www.epicgames.com/id/exchange?exchangeCode={code}&redirectUrl={redirect}"
        ),
        None::<&gio::AppLaunchContext>,
    )
    .is_err()
    {
        error!("Please go to https://www.epicgames.com/id/exchange?exchangeCode={code}&redirectUrl={redirect}");
    }
    #[cfg(target_os = "windows")]
    open::that(format!(
        "https://www.epicgames.com/id/exchange?exchangeCode={}&redirectUrl={redirect}",
        code
    ));
}
