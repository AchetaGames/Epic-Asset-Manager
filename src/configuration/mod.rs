use std::fs;

use clap::{App, Arg};
use config::Config;
use egs_api::api::UserData;
use env_logger::Env;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Configuration {
    pub egs: Config,
    pub user_data: Option<UserData>,
    pub directories: DirectoryConfiguration,
    pub env: Config,
    pub path: Option<String>,
    pub verbose: bool,
}

impl Configuration {
    pub(crate) fn new() -> Configuration {
        let matches = App::new("Epic Asset Manager")
            .about("A GUI tool to access the Epic Games Store Assets")
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("FILE")
                    .help("path to configuration file")
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("v")
                    .short("v")
                    .help("Sets the level of verbosity"),
            )
            .get_matches();

        let path = match matches.value_of("config") {
            None => match dirs::config_dir() {
                None => None,
                Some(mut dir) => {
                    dir.push("epic_asset_manager");
                    match dir.to_str() {
                        None => None,
                        Some(s) => Some(s.to_string()),
                    }
                }
            },
            Some(s) => Some(s.to_string()),
        };

        let mut conf = Configuration {
            egs: Default::default(),
            user_data: UserData::load(path.clone()),
            directories: DirectoryConfiguration::load(path.clone()).unwrap(),
            env: Default::default(),
            path,
            verbose: match matches.occurrences_of("v") {
                0 => false,
                1 | _ => true,
            },
        };

        env_logger::Builder::from_env(Env::default().default_filter_or(if conf.verbose {
            "epic_asset_manager:debug"
        } else {
            "epic_asset_manager:warn"
        }))
        .format(|buf, record| {
            writeln!(
                buf,
                "<{}> - [{}] - {}",
                record.target(),
                record.level(),
                record.args()
            )
        })
        .try_init();

        match conf.egs.merge(config::File::new(
            Path::new(&conf.path.clone().unwrap())
                .join("config.json")
                .to_str()
                .unwrap(),
            config::FileFormat::Json,
        )) {
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to load configuration:  {}", e);
            }
        }

        match conf.env.merge(config::Environment::with_prefix("EAM")) {
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to load properties from Environment: {}", e)
            }
        }

        conf
    }

    pub(crate) fn save(&self) {
        if let Some(path) = self.path.clone() {
            let config_path = Path::new(&path);
            fs::create_dir_all(config_path.clone()).unwrap();
            self.user_data.as_ref().unwrap().save(Some(path.clone()));
            self.directories.save(Some(path.clone()));
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectoryConfiguration {
    pub cache_directory: String,
    pub temporary_download_directory: String,
    pub unreal_vault_directory: String,
    pub unreal_engine_directories: Vec<String>,
    pub unreal_projects_directories: Vec<String>,
}

pub trait Save {
    fn remove(&self, _path: Option<String>) {
        todo!()
    }
    fn save(&self, _path: Option<String>) {
        todo!()
    }
    fn load(_path: Option<String>) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl Save for DirectoryConfiguration {
    fn save(&self, path: Option<String>) {
        if let Some(path) = path.clone() {
            let config_path = Path::new(&path);
            fs::create_dir_all(config_path.clone()).unwrap();
            match File::create(config_path.join("directories.json")) {
                Ok(mut directories_file) => {
                    directories_file
                        .write(serde_json::to_string(&self).unwrap().as_bytes().as_ref())
                        .unwrap();
                }
                Err(e) => {
                    error!("Unable to save directories configuration: {:?}", e)
                }
            }
        }
    }

    fn load(path: Option<String>) -> Option<Self> {
        match File::open(Path::new(&path.clone().unwrap()).join("directories.json")) {
            Ok(user_file) => {
                let reader = BufReader::new(user_file);
                match serde_json::from_reader(reader) {
                    Ok(ud) => {
                        return Some(ud);
                    }
                    Err(e) => {
                        error!("Unable to parse directories configuration: {:?}", e)
                    }
                };
            }
            Err(..) => {
                warn!("Unable to load directories configuration")
            }
        }
        // Config file does not exist, creating new one
        Some(DirectoryConfiguration {
            cache_directory: match dirs::cache_dir() {
                None => PathBuf::from("cache"),
                Some(mut dir) => {
                    dir.push("epic_asset_manager");
                    dir
                }
            }
            .as_path()
            .to_str()
            .unwrap()
            .into(),
            temporary_download_directory: match dirs::document_dir() {
                None => PathBuf::from("Vault"),
                Some(mut dir) => {
                    dir.push("EpicVault");
                    dir
                }
            }
            .as_path()
            .to_str()
            .unwrap()
            .into(),
            unreal_vault_directory: match dirs::document_dir() {
                None => PathBuf::from("Vault"),
                Some(mut dir) => {
                    dir.push("EpicVault");
                    dir
                }
            }
            .as_path()
            .to_str()
            .unwrap()
            .into(),
            unreal_engine_directories: vec![],
            unreal_projects_directories: match dirs::document_dir() {
                Some(mut dir) => {
                    dir.push("Unreal Projects");
                    vec![dir.as_path().to_str().unwrap().to_string()]
                }
                None => {
                    vec![]
                }
            },
        })
    }
}

impl Save for UserData {
    fn save(&self, path: Option<String>) {
        if let Some(path) = path.clone() {
            let config_path = Path::new(&path);
            fs::create_dir_all(config_path.clone()).unwrap();
            match File::create(config_path.join("user.json")) {
                Ok(mut directories_file) => {
                    directories_file
                        .write(serde_json::to_string(&self).unwrap().as_bytes().as_ref())
                        .unwrap();
                }
                Err(e) => {
                    error!("Unable to save User Data: {:?}", e)
                }
            }
        }
    }

    fn remove(&self, path: Option<String>) {
        match fs::remove_file(Path::new(&path.clone().unwrap()).join("user.json")) {
            Ok(_) => {
                info!("User Data Removed")
            }
            Err(e) => {
                warn!("Unable to remove User Data: {}", e)
            }
        }
    }

    fn load(path: Option<String>) -> Option<Self> {
        match File::open(Path::new(&path.clone().unwrap()).join("user.json")) {
            Ok(user_file) => {
                let reader = BufReader::new(user_file);
                match serde_json::from_reader(reader) {
                    Ok(ud) => Some(ud),
                    Err(..) => None,
                }
            }
            Err(..) => None,
        }
    }
}
