use glib::clone;
use glib::ObjectExt;
use gtk4::{glib, prelude::*, subclass::prelude::*};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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

impl UnrealVersion {
    pub fn format(&self) -> String {
        format!(
            "{}.{}.{}",
            self.major_version, self.minor_version, self.patch_version
        )
    }
}

impl UnrealVersion {
    pub fn compare(&self, other: &UnrealVersion) -> Ordering {
        match self.major_version.cmp(&other.major_version) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => match self.minor_version.cmp(&other.minor_version) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => match self.patch_version.cmp(&other.patch_version) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => Ordering::Equal,
                    Ordering::Greater => Ordering::Greater,
                },
                Ordering::Greater => Ordering::Greater,
            },
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EngineMsg {
    Update(bool),
    Branch(String),
}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::glib::ParamSpec;
    use once_cell::sync::OnceCell;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct EngineData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        version: RefCell<Option<String>>,
        branch: RefCell<Option<String>>,
        updatable: RefCell<bool>,
        has_branch: RefCell<bool>,
        pub ueversion: RefCell<Option<super::UnrealVersion>>,
        pub sender: gtk4::glib::Sender<super::EngineMsg>,
        pub receiver: RefCell<Option<gtk4::glib::Receiver<super::EngineMsg>>>,
        pub model: OnceCell<gtk4::gio::ListStore>,
        pub position: OnceCell<u32>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for EngineData {
        const NAME: &'static str = "EngineData";
        type Type = super::EngineData;
        type ParentType = glib::Object;

        fn new() -> Self {
            let (sender, receiver) = gtk4::glib::MainContext::channel(gtk4::glib::PRIORITY_DEFAULT);
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                version: RefCell::new(None),
                branch: RefCell::new(None),
                updatable: RefCell::new(false),
                has_branch: RefCell::new(false),
                ueversion: RefCell::new(None),
                sender,
                receiver: RefCell::new(Some(receiver)),
                model: OnceCell::new(),
                position: OnceCell::new(),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for EngineData {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_messaging();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "finished",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "guid",
                        "GUID",
                        "GUID",
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
                        "version",
                        "Version",
                        "Version",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_boolean(
                        "needs-update",
                        "needs update",
                        "Check if engine needs update",
                        false,
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
                "guid" => {
                    let guid = value.get().unwrap();
                    self.guid.replace(guid);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "version" => {
                    let version = value.get().unwrap();
                    self.version.replace(version);
                }
                "needs-update" => {
                    let updatable = value.get().unwrap();
                    self.updatable.replace(updatable);
                }
                "branch" => {
                    let branch = value
                        .get::<Option<String>>()
                        .expect("type conformity checked by `Object::set_property`");
                    self.branch.replace(branch);
                }
                "has-branch" => {
                    let has_branch = value.get().unwrap();
                    self.has_branch.replace(has_branch);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "version" => self.version.borrow().to_value(),
                "needs-update" => self.updatable.borrow().to_value(),
                "branch" => self.branch.borrow().to_value(),
                "has-branch" => self.has_branch.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the EngineData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct EngineData(ObjectSubclass<imp::EngineData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl EngineData {
    pub fn new(
        path: &str,
        guid: &str,
        version: &UnrealVersion,
        model: &gtk4::gio::ListStore,
    ) -> EngineData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create EngineData");
        let self_: &imp::EngineData = imp::EngineData::from_instance(&data);
        self_.position.set(model.n_items()).unwrap();
        self_.model.set(model.clone()).unwrap();
        data.set_property("path", &path).unwrap();
        data.set_property("guid", &guid).unwrap();
        self_.ueversion.replace(Some(version.clone()));
        data.set_property("version", version.format()).unwrap();
        if let Some(path) = data.path() {
            let sender = self_.sender.clone();
            thread::spawn(move || {
                Self::needs_repo_update(&path, Some(sender));
            });
        }
        data
    }

    pub fn read_engine_version(path: &str) -> Option<UnrealVersion> {
        let mut p = std::path::PathBuf::from(path);
        p.push("Engine");
        p.push("Build");
        p.push("Build.version");
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return Some(json5::from_str(&contents).unwrap_or_default());
            }
        }
        None
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::EngineData = imp::EngineData::from_instance(self);
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
        match msg {
            EngineMsg::Update(waiting) => {
                self.set_property("needs-update", waiting).unwrap();
            }
            EngineMsg::Branch(branch) => {
                self.set_property("has-branch", !branch.is_empty()).unwrap();
                self.set_property("branch", branch).unwrap();
            }
        };
        self.emit_by_name("finished", &[]).unwrap();
    }

    fn needs_repo_update(path: &str, sender: Option<gtk4::glib::Sender<EngineMsg>>) -> bool {
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
                        ))
                        .unwrap();
                    }
                }
            }
            let mut time = git2::Time::new(0, 0);
            if let Ok(c) = repo.find_commit(commit) {
                time = c.time();
            }
            if let Ok(remotes) = repo.remotes() {
                let num_remotes = remotes.len();
                for remote in remotes.iter().flatten() {
                    if let Ok(mut r) = repo.find_remote(remote) {
                        if num_remotes > 1 {
                            if let Some(url) = r.url() {
                                if !url.contains("EpicGames/UnrealEngine.git") {
                                    continue;
                                }
                            }
                        }
                        let cb = Self::git_callbacks();
                        if let Err(e) = r.connect_auth(git2::Direction::Fetch, Some(cb), None) {
                            warn!("Unable to connect: {}", e);
                        }
                        if let Ok(list) = r.list() {
                            for head in list {
                                if branch.eq(&head.name()) {
                                    if head.oid().eq(&commit) {
                                        debug!("{} Up to date", path);
                                        if let Some(s) = sender {
                                            s.send(EngineMsg::Update(false)).unwrap();
                                        }
                                        return false;
                                    }
                                    info!("{} needs updating", path);
                                    debug!(
                                        "{} Local commit {}({}), remote commit {}",
                                        path,
                                        commit,
                                        time.seconds(),
                                        head.oid()
                                    );
                                    if let Some(s) = sender {
                                        s.send(EngineMsg::Update(true)).unwrap();
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        };
        false
    }

    pub fn guid(&self) -> Option<String> {
        if let Ok(value) = self.property("guid") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn path(&self) -> Option<String> {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn branch(&self) -> Option<String> {
        if let Ok(value) = self.property("branch") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn ueversion(&self) -> Option<UnrealVersion> {
        let self_: &imp::EngineData = imp::EngineData::from_instance(self);
        self_.ueversion.borrow().clone()
    }

    pub fn version(&self) -> Option<String> {
        if let Ok(value) = self.property("version") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn has_branch(&self) -> Option<bool> {
        if let Ok(value) = self.property("has-branch") {
            if let Ok(id_opt) = value.get::<bool>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn needs_update(&self) -> Option<bool> {
        if let Ok(value) = self.property("needs-update") {
            if let Ok(id_opt) = value.get::<bool>() {
                return Some(id_opt);
            }
        };
        None
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
                    .map(std::string::ToString::to_string)
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
}
