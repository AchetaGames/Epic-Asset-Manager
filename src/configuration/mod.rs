use std::fs;

use clap::{App, Arg};
use config::Config;
use egs_api::api::UserData;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct Configuration {
    pub egs: Config,
    pub user_data: Option<UserData>,
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

        let mut conf = Configuration {
            egs: Default::default(),
            user_data: None,
            env: Default::default(),
            path: match matches.value_of("config") {
                None => None,
                Some(s) => Some(s.to_string()),
            },
            verbose: match matches.occurrences_of("v") {
                0 => false,
                1 | _ => true,
            },
        };

        match conf.path {
            None => match dirs::config_dir() {
                None => {}
                Some(mut dir) => {
                    dir.push("epic_asset_manager");
                    conf.path = match dir.to_str() {
                        None => None,
                        Some(s) => Some(s.to_string()),
                    }
                }
            },
            Some(_) => {}
        }

        match File::open(Path::new(&conf.path.clone().unwrap()).join("user.json")) {
            Ok(user_file) => {
                let reader = BufReader::new(user_file);
                match serde_json::from_reader(reader) {
                    Ok(ud) => conf.user_data = Some(ud),
                    Err(_) => {}
                };
            }
            Err(_) => {}
        }

        match conf.egs.merge(config::File::new(
            &conf.path.clone().unwrap(),
            config::FileFormat::Json,
        )) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to load bot configuration:  {}", e);
            }
        }

        match conf.env.merge(config::Environment::with_prefix("EAM")) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to load properties from Environment: {}", e)
            }
        }

        conf
    }

    pub(crate) fn save(&self) {
        if let Some(path) = self.path.clone() {
            let config_path = Path::new(&path);
            fs::create_dir_all(config_path.clone()).unwrap();
            match File::create(config_path.join("user.json")) {
                Ok(mut user_file) => {
                    user_file
                        .write(
                            serde_json::to_string(&self.user_data)
                                .unwrap()
                                .as_bytes()
                                .as_ref(),
                        )
                        .unwrap();
                }
                Err(_) => {}
            }
        }
    }
}
