use crate::ui::widgets::logged_in::engine::EpicEngine;
use git2::{Cred, Direction, Oid, RemoteCallbacks, Repository};
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;

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

pub(crate) mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/engines.ui")]
    pub struct EpicEnginesBox {
        pub window: OnceCell<crate::window::EpicAssetManagerWindow>,
        pub download_manager: OnceCell<crate::ui::widgets::download_manager::EpicDownloadManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicEnginesBox {
        const NAME: &'static str = "EpicEnginesBox";
        type Type = super::EpicEnginesBox;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                window: OnceCell::new(),
                download_manager: OnceCell::new(),
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

    impl ObjectImpl for EpicEnginesBox {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.load_engines();
        }
    }

    impl WidgetImpl for EpicEnginesBox {}
    impl BoxImpl for EpicEnginesBox {}
}

glib::wrapper! {
    pub struct EpicEnginesBox(ObjectSubclass<imp::EpicEnginesBox>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicEnginesBox {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicEnginesBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_window(&self, window: &crate::window::EpicAssetManagerWindow) {
        let self_: &imp::EpicEnginesBox = imp::EpicEnginesBox::from_instance(self);
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
        let self_: &imp::EpicEnginesBox = imp::EpicEnginesBox::from_instance(self);
        // Do not run this twice
        if self_.download_manager.get().is_some() {
            return;
        }
        self_.download_manager.set(dm.clone()).unwrap();
    }

    pub fn load_engines(&self) {
        for (guid, path) in Self::read_engines_ini() {
            let version = Self::read_engine_version(path.to_string());
            let engine = EpicEngine::new();
            engine.set_property("path", path.clone());
            engine.set_property(
                "version",
                format!(
                    "{}.{}.{}",
                    version.major_version, version.minor_version, version.patch_version
                ),
            );
            self.append(&engine);
        }
    }

    pub fn read_engines_ini() -> HashMap<String, String> {
        let ini = gtk4::glib::KeyFile::new();
        let mut dir = gtk4::glib::home_dir();
        // TODO: This is not platform independent, Linux only
        dir.push(".config");
        dir.push("Epic");
        dir.push("UnrealEngine");
        dir.push("Install.ini");
        let mut result: HashMap<String, String> = HashMap::new();
        if let Err(e) = ini.load_from_file(dir, gtk4::glib::KeyFileFlags::NONE) {
            warn!("Unable to load engine Install.ini: {}", e);
            return result;
        };

        if let Ok(keys) = ini.keys("Installations") {
            for item in keys.0 {
                if let Ok(path) = ini.value("Installations", &item) {
                    debug!("Got engine install: {} in {}", item, path);
                    result.insert(item.to_string(), path.to_string());
                }
            }
        }
        result
    }

    fn read_engine_version(path: String) -> UnrealVersion {
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

    fn needs_repo_update(path: String) -> bool {
        if let Ok(repo) = Repository::open(&path) {
            let mut commit = Oid::zero();
            let mut branch = String::new();
            if let Ok(head) = repo.head() {
                if head.is_branch() {
                    commit = head.target().unwrap().into();
                    branch = head.name().unwrap().to_string();
                }
            }
            let mut time = git2::Time::new(0, 0);
            if let Ok(c) = repo.find_commit(commit) {
                time = c.time();
            }
            if let Ok(remotes) = repo.remotes() {
                for remote in remotes.iter() {
                    if let Some(rem) = remote {
                        if let Ok(mut r) = repo.find_remote(rem) {
                            let cb = EpicEnginesBox::git_callbacks();
                            if let Err(e) = r.connect_auth(Direction::Fetch, Some(cb), None) {
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
                                            return true;
                                        }
                                    }
                                }
                            }
                        };
                    }
                }
            }
        };
        false
    }

    fn git_callbacks() -> RemoteCallbacks<'static> {
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
                    .unwrap_or("git".to_string());
                if key.exists() {
                    Cred::ssh_key(&user, None, key.as_path(), None)
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
