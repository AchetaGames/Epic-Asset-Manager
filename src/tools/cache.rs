use egs_api::api::types::asset_info::{AssetInfo, KeyImage};
use egs_api::api::types::download_manifest::DownloadManifest;
use egs_api::api::types::epic_asset::EpicAsset;
use log::error;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;

pub(crate) trait Cache {
    fn save(&self, data: Option<Vec<u8>>, asset_id: Option<String>);
    fn prepare(item: String) -> PathBuf {
        let mut path = match dirs::cache_dir() {
            None => PathBuf::from("cache"),
            Some(mut dir) => {
                dir.push("epic_asset_manager");
                dir
            }
        };
        path.push(&item);

        let cache = Path::new(&path);
        fs::create_dir_all(cache.clone()).unwrap();
        cache.to_path_buf()
    }

    fn load_from_cache(_item: String, _name: Option<String>) -> Option<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn load(&self) -> Option<Vec<u8>> {
        unimplemented!()
    }
}

impl Cache for EpicAsset {
    fn save(&self, _: Option<Vec<u8>>, _: Option<String>) {
        let mut cache = <Self as Cache>::prepare(self.catalog_item_id.clone());
        cache.push("epic_asset.json");
        if let Ok(mut asset_file) = File::create(cache.as_path()) {
            asset_file
                .write(serde_json::to_string(&self).unwrap().as_bytes().as_ref())
                .unwrap();
        }
    }
}

impl Cache for AssetInfo {
    fn save(&self, _: Option<Vec<u8>>, _: Option<String>) {
        let mut cache = <Self as Cache>::prepare(self.id.clone());
        cache.push("asset_info.json");
        if let Ok(mut asset_file) = File::create(cache.as_path()) {
            asset_file
                .write(serde_json::to_string(&self).unwrap().as_bytes().as_ref())
                .unwrap();
        }
    }

    fn load_from_cache(id: String, _: Option<String>) -> Option<Self> {
        let mut cache = <Self as Cache>::prepare(id.clone());
        cache.push("asset_info.json");
        match File::open(cache.as_path()) {
            Ok(mut f) => {
                let metadata = fs::metadata(&cache.as_path()).expect("unable to read metadata");
                let mut buffer = vec![0; metadata.len() as usize];
                f.read(&mut buffer).expect("buffer overflow");
                return serde_json::from_slice(buffer.as_ref()).unwrap_or(None);
            }
            Err(_) => {}
        };
        None
    }
}

impl Cache for DownloadManifest {
    fn save(&self, _: Option<Vec<u8>>, id: Option<String>) {
        let mut cache = <Self as Cache>::prepare(match id {
            None => return,
            Some(n) => n,
        });
        cache.push(format!("{}.json", self.app_name_string));
        if let Ok(mut asset_file) = File::create(cache.as_path()) {
            asset_file
                .write(serde_json::to_string(&self).unwrap().as_bytes().as_ref())
                .unwrap();
        }
    }

    fn load_from_cache(id: String, name: Option<String>) -> Option<Self> {
        let mut cache = <Self as Cache>::prepare(id.clone());
        cache.push(format!(
            "{}.json",
            match name {
                None => {
                    return None;
                }
                Some(n) => n,
            }
        ));
        match File::open(cache.as_path()) {
            Ok(mut f) => {
                let metadata = fs::metadata(&cache.as_path()).expect("unable to read metadata");
                let mut buffer = vec![0; metadata.len() as usize];
                f.read(&mut buffer).expect("buffer overflow");
                return serde_json::from_slice(buffer.as_ref()).unwrap_or(None);
            }
            Err(_) => {}
        };
        None
    }
}

impl Cache for KeyImage {
    fn save(&self, data: Option<Vec<u8>>, _: Option<String>) {
        let mut cache = <Self as Cache>::prepare("images".into());
        match data {
            None => {}
            Some(d) => {
                let name = Path::new(self.url.path())
                    .extension()
                    .and_then(OsStr::to_str);
                cache.push(format!("{}.{}", self.md5, name.unwrap_or(&".png")));
                match File::create(cache.as_path()) {
                    Ok(mut asset_file) => {
                        asset_file.write(&d).unwrap();
                    }
                    Err(e) => {
                        error!("{:?}", e);
                    }
                }
            }
        }
    }

    fn load(&self) -> Option<Vec<u8>> {
        let mut cache = <Self as Cache>::prepare("images".into());
        let name = Path::new(self.url.path())
            .extension()
            .and_then(OsStr::to_str);
        cache.push(format!("{}.{}", self.md5, name.unwrap_or(&".png")));
        match File::open(cache.as_path()) {
            Ok(mut f) => {
                let metadata = fs::metadata(&cache.as_path()).expect("unable to read metadata");
                let mut buffer = vec![0; metadata.len() as usize];
                f.read(&mut buffer).expect("buffer overflow");
                return Some(buffer);
            }
            Err(_) => {}
        };
        None
    }
}
