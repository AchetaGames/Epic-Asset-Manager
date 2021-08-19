use crate::tools::cache::Cache;
use crate::Win;
use egs_api::api::types::asset_info::AssetInfo;
use egs_api::api::types::epic_asset::EpicAsset;
use gtk4::traits::{ProgressBarExt, RevealerExt};
use relm::Channel;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::path::Path;
use std::thread;
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

pub(crate) trait EpicAssets {
    fn process_list(&self, _anm: HashMap<String, Vec<String>>, _am: HashMap<String, EpicAsset>) {
        unimplemented!()
    }
}

impl EpicAssets for Win {
    fn process_list(&self, anm: HashMap<String, Vec<String>>, am: HashMap<String, EpicAsset>) {
        // TODO: Cache EpicAsset

        if let Ok(mut asset_namespace_map) = crate::DATA.asset_namespace_map.write() {
            asset_namespace_map.clear();
            asset_namespace_map.extend(anm.clone())
        }
        if let Ok(mut asset_map) = crate::DATA.asset_map.write() {
            asset_map.clear();
            asset_map.extend(am.clone())
        }
        self.widgets.loading_progress.set_fraction(0.0);
        self.widgets
            .loading_progress
            .set_pulse_step(1.0 / am.len() as f64);
        self.widgets.progress_revealer.set_reveal_child(true);
        let stream = self.model.relm.stream().clone();
        let (_channel, sender) = Channel::new(move |ai| {
            stream.emit(crate::ui::messages::Msg::ProcessAssetInfo(ai));
        });

        let eg = self.model.epic_games.clone();
        let mut fa: Vec<EpicAsset> = Vec::from_iter(am.values().cloned());
        let user_data =
            Path::new(&self.model.configuration.path.clone().unwrap()).join("user.json");
        thread::spawn(move || {
            let start = std::time::Instant::now();
            let pool = ThreadPool::new(3);
            fa.sort_by(|a, b| a.app_name.cmp(&b.app_name));
            for ass in fa.clone() {
                if !user_data.exists() {
                    break;
                }
                let mut e = eg.clone();
                let s = sender.clone();
                let ud = user_data.clone();
                pool.execute(move || {
                    if ud.exists() {
                        let start = std::time::Instant::now();
                        match AssetInfo::load_from_cache(ass.catalog_item_id.clone(), None) {
                            None => {
                                if let Some(asset) =
                                    Runtime::new().unwrap().block_on(e.asset_info(ass))
                                {
                                    asset.save(None, None);
                                    if let Ok(mut asset_info) = crate::DATA.asset_info.write() {
                                        asset_info.insert(asset.id.clone(), asset.clone());
                                    }
                                    s.send(asset).unwrap();
                                };
                            }
                            Some(asset) => {
                                if let Ok(mut asset_info) = crate::DATA.asset_info.write() {
                                    asset_info.insert(asset.id.clone(), asset.clone());
                                }
                                s.send(asset).unwrap();
                            }
                        };
                        debug!(
                            "{:?} - Asset Info loading took {:?}",
                            thread::current().id(),
                            start.elapsed()
                        );
                    }
                });
            }
            debug!(
                "{:?} - AssetInfo processing took {:?}",
                thread::current().id(),
                start.elapsed()
            );
        });
    }
}
