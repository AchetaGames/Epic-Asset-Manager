use gtk4::prelude::ObjectExt;
use gtk4::{glib, prelude::*, subclass::prelude::*};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Read;
use std::thread;

#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
        if self.valid() {
            format!(
                "{}.{}.{}",
                self.major_version, self.minor_version, self.patch_version
            )
        } else {
            self.branch_name.to_string()
        }
    }

    pub const fn valid(&self) -> bool {
        !(self.major_version == -1
            && self.minor_version == -1
            && self.patch_version == -1
            && self.changelist == -1
            && self.compatible_changelist == -1
            && self.is_licensee_version == -1
            && self.is_promoted_build == -1)
    }

    pub fn compare(&self, other: &UnrealVersion) -> Ordering {
        if !self.valid() {
            return Ordering::Greater;
        }
        if !other.valid() {
            return Ordering::Less;
        }
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
pub enum Msg {
    Update(bool),
    Branch(String),
}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use gtk4::glib::{ParamSpec, ParamSpecBoolean, ParamSpecString};
    use gtk4::prelude::ToValue;
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
        pub ueversion: RefCell<Option<UnrealVersion>>,
        pub sender: async_channel::Sender<Msg>,
        pub receiver: RefCell<Option<async_channel::Receiver<Msg>>>,
        pub model: OnceCell<gtk4::gio::ListStore>,
        pub position: OnceCell<u32>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for EngineData {
        const NAME: &'static str = "EngineData";
        type Type = super::EngineData;

        fn new() -> Self {
            let (sender, receiver) = async_channel::unbounded();
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
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_messaging();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![glib::subclass::Signal::builder("finished")
                        .flags(glib::SignalFlags::ACTION)
                        .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("guid").build(),
                    ParamSpecString::builder("path").build(),
                    ParamSpecString::builder("version").build(),
                    ParamSpecBoolean::builder("needs-update").build(),
                    ParamSpecString::builder("branch").build(),
                    ParamSpecBoolean::builder("has-branch").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
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
        let data: Self = glib::Object::new::<Self>();
        let self_ = data.imp();
        self_.position.set(model.n_items()).unwrap();
        self_.model.set(model.clone()).unwrap();
        data.set_property("path", path);
        data.set_property("guid", guid);
        self_.ueversion.replace(Some(version.clone()));
        data.set_property("version", version.format());
        if let Some(path) = data.path() {
            let sender = self_.sender.clone();
            thread::spawn(move || {
                Self::needs_repo_update(&path, &Some(sender));
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
                return Some(serde_json::from_str(&contents).unwrap_or_default());
            }
        }
        None
    }

    pub fn setup_messaging(&self) {
        let self_ = self.imp();
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        let engine = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(response) = receiver.recv().await {
                engine.update(response);
            }
        });
    }

    pub fn update(&self, msg: Msg) {
        match msg {
            Msg::Update(waiting) => {
                self.set_property("needs-update", waiting);
            }
            Msg::Branch(branch) => {
                self.set_property("has-branch", !branch.is_empty());
                self.set_property("branch", branch);
            }
        };
        self.emit_by_name::<()>("finished", &[]);
    }

    #[allow(clippy::missing_const_for_fn)]
    fn needs_repo_update(_path: &str, _sender: &Option<async_channel::Sender<Msg>>) -> bool {
        // #[cfg(target_os = "linux")]
        // This is disabled due to issues with git2 crate and constant need to rebuild if git lib gets updated
        // {
        //     if let Ok(repo) = git2::Repository::open(&path) {
        //         let mut commit = git2::Oid::zero();
        //         let mut branch = String::new();
        //         if let Ok(head) = repo.head() {
        //             if head.is_branch() {
        //                 commit = head.target().unwrap();
        //                 branch = head.name().unwrap().to_string();
        //                 if let Some(s) = sender.clone() {
        //                     s.send(Msg::Branch(
        //                         head.shorthand().unwrap_or_default().to_string(),
        //                     ))
        //                     .unwrap();
        //                 }
        //             }
        //         }
        //         let mut time = git2::Time::new(0, 0);
        //         if let Ok(c) = repo.find_commit(commit) {
        //             time = c.time();
        //         }
        //         if let Ok(remotes) = repo.remotes() {
        //             let num_remotes = remotes.len();
        //             for remote in remotes.iter().flatten() {
        //                 if let Ok(mut r) = repo.find_remote(remote) {
        //                     if num_remotes > 1 {
        //                         if let Some(url) = r.url() {
        //                             if !url.contains("EpicGames/UnrealEngine.git") {
        //                                 continue;
        //                             }
        //                         }
        //                     }
        //                     let cb = Self::git_callbacks();
        //                     if let Err(e) = r.connect_auth(git2::Direction::Fetch, Some(cb), None) {
        //                         warn!("Unable to connect: {}", e);
        //                     }
        //                     if let Ok(list) = r.list() {
        //                         for head in list {
        //                             if branch.eq(&head.name()) {
        //                                 if head.oid().eq(&commit) {
        //                                     debug!("{} Up to date", path);
        //                                     if let Some(s) = sender {
        //                                         s.send(Msg::Update(false)).unwrap();
        //                                     }
        //                                     return false;
        //                                 }
        //                                 info!("{} needs updating", path);
        //                                 debug!(
        //                                     "{} Local commit {}({}), remote commit {}",
        //                                     path,
        //                                     commit,
        //                                     time.seconds(),
        //                                     head.oid()
        //                                 );
        //                                 if let Some(s) = sender {
        //                                     s.send(Msg::Update(true)).unwrap();
        //                                 }
        //                                 return true;
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     };
        // }
        false
    }

    pub fn guid(&self) -> Option<String> {
        self.property("guid")
    }

    pub fn path(&self) -> Option<String> {
        self.property("path")
    }

    pub fn branch(&self) -> Option<String> {
        self.property("branch")
    }

    pub fn ueversion(&self) -> Option<UnrealVersion> {
        let self_ = self.imp();
        self_.ueversion.borrow().clone()
    }

    pub fn valid(&self) -> bool {
        self.ueversion().map_or(false, |v| v.valid())
    }

    pub fn version(&self) -> Option<String> {
        self.property("version")
    }

    pub fn has_branch(&self) -> bool {
        self.property("has-branch")
    }

    pub fn needs_update(&self) -> bool {
        self.property("needs-update")
    }

    // #[cfg(target_os = "linux")]
    // fn git_callbacks() -> git2::RemoteCallbacks<'static> {
    //     let git_config = git2::Config::open_default().unwrap();
    //     let mut cb = git2::RemoteCallbacks::new();
    //     cb.credentials(move |url, username, allowed| {
    //         let mut cred_helper = git2::CredentialHelper::new(url);
    //         cred_helper.config(&git_config);
    //         if allowed.is_ssh_key() {
    //             // TODO: Add configuration to specify the ssh key and password(if needed)
    //             let mut key = glib::home_dir();
    //             key.push(".ssh");
    //             key.push("id_rsa");
    //
    //             let user = username
    //                 .map(std::string::ToString::to_string)
    //                 .or_else(|| cred_helper.username.clone())
    //                 .unwrap_or_else(|| "git".to_string());
    //             if key.exists() {
    //                 git2::Cred::ssh_key(&user, None, key.as_path(), None)
    //             } else {
    //                 git2::Cred::ssh_key_from_agent(&user)
    //             }
    //         } else if allowed.is_user_pass_plaintext() {
    //             git2::Cred::credential_helper(&git_config, url, username)
    //         } else if allowed.is_default() {
    //             git2::Cred::default()
    //         } else {
    //             Err(git2::Error::from_str("no authentication available"))
    //         }
    //     });
    //     cb
    // }
}
