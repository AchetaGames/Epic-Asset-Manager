use glib::clone;
use gtk4::cairo::glib::Sender;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::thread;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UnrealVersion {
    #[serde(default)]
    pub major_version: i64,
    #[serde(default)]
    pub minor_version: i64,
    #[serde(default)]
    pub patch_version: i64,
    #[serde(default)]
    pub changelist: i64,
    #[serde(default)]
    pub compatible_changelist: i64,
    #[serde(default)]
    pub is_licensee_version: i64,
    #[serde(default)]
    pub is_promoted_build: i64,
    #[serde(default)]
    pub branch_name: String,
}

#[derive(Debug, Clone)]
pub enum EngineMsg {
    Update(bool),
    Branch(String),
}

pub(crate) mod imp {
    use super::*;
    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engine.ui")]
    pub struct EpicEngine {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
        version: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        guid: RefCell<Option<String>>,
        branch: RefCell<Option<String>>,
        updatable: RefCell<bool>,
        has_branch: RefCell<bool>,
        pub sender: gtk4::glib::Sender<super::EngineMsg>,
        pub receiver: RefCell<Option<gtk4::glib::Receiver<super::EngineMsg>>>,
        pub ueversion: RefCell<Option<super::UnrealVersion>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEngine {
        const NAME: &'static str = "EpicEngine";
        type Type = super::EpicEngine;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            let (sender, receiver) = gtk4::glib::MainContext::channel(gtk4::glib::PRIORITY_DEFAULT);
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
                version: RefCell::new(None),
                path: RefCell::new(None),
                guid: RefCell::new(None),
                branch: RefCell::new(None),
                updatable: RefCell::new(false),
                has_branch: RefCell::new(false),
                sender,
                receiver: RefCell::new(Some(receiver)),
                ueversion: RefCell::new(None),
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

    impl ObjectImpl for EpicEngine {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_messaging();
            obj.init();
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "needs-update",
                        "needs update",
                        "Check if engine needs update",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "version",
                        "Version",
                        "Version",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "path",
                        "Path",
                        "Path",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "branch",
                        "Branch",
                        "Branch",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_boolean(
                        "has-branch",
                        "Has Branch",
                        "Has Branch",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "guid",
                        "GUID",
                        "GUID",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "needs-update" => {
                    let updatable = value.get().unwrap();
                    self.updatable.replace(updatable);
                }
                "version" => {
                    let version = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<span size=\"xx-large\"><b><u>{}</u></b></span>", l));
                    self.version.replace(version);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                    obj.init();
                }
                "branch" => {
                    let branch = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`")
                        .map(|l| format!("<i><b>Branch:</b> {}</i>", l));
                    self.branch.replace(branch);
                }
                "has-branch" => {
                    let has_branch = value.get().unwrap();
                    self.has_branch.replace(has_branch);
                }
                "guid" => {
                    let guid = value.get().unwrap();
                    self.guid.replace(guid);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "needs-update" => self.updatable.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "branch" => self.branch.borrow().to_value(),
                "has-branch" => self.has_branch.borrow().to_value(),
                "guid" => self.guid.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicEngine {}
    impl BoxImpl for EpicEngine {}
}

glib::wrapper! {
    pub struct EpicEngine(ObjectSubclass<imp::EpicEngine>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEngine {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn init(&self) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        if let Some(path) = self.path() {
            let version = Self::read_engine_version(&path);
            self_.ueversion.replace(Some(version.clone()));
            self.set_property(
                "version",
                format!(
                    "{}.{}.{}",
                    version.major_version, version.minor_version, version.patch_version
                ),
            )
            .unwrap();
            let p = path.clone();
            let sender = self_.sender.clone();
            thread::spawn(move || {
                Self::needs_repo_update(p, Some(sender));
            });
        }
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as engine => @default-panic, move |msg| {
                engine.update(msg);
                glib::Continue(true)
            }),
        );
    }
    pub fn update(&self, msg: EngineMsg) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        match msg {
            EngineMsg::Update(waiting) => {
                self.set_property("needs-update", waiting).unwrap();
            }
            EngineMsg::Branch(branch) => {
                self.set_property("has-branch", !branch.is_empty()).unwrap();
                self.set_property("branch", branch).unwrap();
            }
        }
    }

    pub fn path(&self) -> Option<String> {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
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
        let self_: &imp::EpicEngine = imp::EpicEngine::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    fn needs_repo_update(path: String, sender: Option<gtk4::glib::Sender<EngineMsg>>) -> bool {
        if let Ok(repo) = git2::Repository::open(&path) {
            let mut commit = git2::Oid::zero();
            let mut branch = String::new();
            if let Ok(head) = repo.head() {
                if head.is_branch() {
                    commit = head.target().unwrap();
                    branch = head.name().unwrap().to_string();
                    if let Some(s) = sender.clone() {
                        s.send(EngineMsg::Branch(
                            head.shorthand().unwrap_or_default().to_string(),
                        ));
                    }
                }
            }
            let mut time = git2::Time::new(0, 0);
            if let Ok(c) = repo.find_commit(commit) {
                time = c.time();
            }
            if let Ok(remotes) = repo.remotes() {
                for remote in remotes.iter().flatten() {
                    if let Ok(mut r) = repo.find_remote(remote) {
                        let cb = Self::git_callbacks();
                        if let Err(e) = r.connect_auth(git2::Direction::Fetch, Some(cb), None) {
                            warn!("Unable to connect: {}", e)
                        }
                        // let mut fo = git2::FetchOptions::new();
                        // let cb = EpicEnginesBox::git_callbacks();
                        // fo.remote_callbacks(cb);
                        // r.fetch(&[&branch], Some(&mut fo), None);
                        if let Ok(list) = r.list() {
                            for head in list {
                                if branch.eq(&head.name()) {
                                    if head.oid().eq(&commit) {
                                        debug!("{} Up to date", path);
                                        if let Some(s) = sender.clone() {
                                            s.send(EngineMsg::Update(false));
                                        }
                                        return false;
                                    } else {
                                        info!("{} needs updating", path);
                                        debug!(
                                            "{} Local commit {}({}), remote commit {}",
                                            path,
                                            commit,
                                            time.seconds(),
                                            head.oid()
                                        );
                                        if let Some(s) = sender.clone() {
                                            s.send(EngineMsg::Update(true));
                                        }
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        false
    }

    fn git_callbacks() -> git2::RemoteCallbacks<'static> {
        let git_config = git2::Config::open_default().unwrap();
        let mut cb = git2::RemoteCallbacks::new();
        cb.credentials(move |url, username, allowed| {
            let mut cred_helper = git2::CredentialHelper::new(url);
            cred_helper.config(&git_config);
            let creds = if allowed.is_ssh_key() {
                // TODO: Add configuration to specify the ssh key and password(if needed)
                let mut key = gtk4::glib::home_dir();
                key.push(".ssh");
                key.push("id_rsa");

                let user = username
                    .map(|s| s.to_string())
                    .or_else(|| cred_helper.username.clone())
                    .unwrap_or_else(|| "git".to_string());
                if key.exists() {
                    git2::Cred::ssh_key(&user, None, key.as_path(), None)
                } else {
                    git2::Cred::ssh_key_from_agent(&user)
                }
            } else if allowed.is_user_pass_plaintext() {
                git2::Cred::credential_helper(&git_config, url, username)
            } else if allowed.is_default() {
                git2::Cred::default()
            } else {
                Err(git2::Error::from_str("no authentication available"))
            };
            creds
        });
        cb
    }

    pub fn read_engine_version(path: &str) -> UnrealVersion {
        let mut p = std::path::PathBuf::from(path);
        p.push("Engine");
        p.push("Build");
        p.push("Build.version");
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return serde_json::from_str(&contents).unwrap_or_default();
            }
        }
        UnrealVersion::default()
    }
}
