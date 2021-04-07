use crate::tools::cache::Cache;
use crate::Win;
use egs_api::api::types::asset_info::KeyImage;
use relm::Channel;
use std::path::Path;
use std::thread;

pub(crate) trait Images {
    fn download_image(&self, _asset_id: Option<String>, _image: KeyImage) {
        unimplemented!()
    }
}

impl Images for Win {
    fn download_image(&self, asset_id: Option<String>, image: KeyImage) {
        let stream = self.model.relm.stream().clone();
        let (_channel, sender) = Channel::new(move |(id, b)| {
            stream.emit(crate::ui::messages::Msg::ProcessImage(id, b));
        });

        let user_data =
            Path::new(&self.model.configuration.path.clone().unwrap()).join("user.json");
        match asset_id {
            None => &self.model.image_pool,
            Some(_) => &self.model.thumbnail_pool,
        }
        .execute(move || {
            if user_data.exists() {
                let start = std::time::Instant::now();
                match image.load() {
                    None => {
                        if let Ok(response) = reqwest::blocking::get(image.url.clone()) {
                            if let Ok(b) = response.bytes() {
                                image.save(Some(Vec::from(b.as_ref())), None);
                                match asset_id {
                                    None => {
                                        sender.send((None, Vec::from(b.as_ref()))).unwrap();
                                    }
                                    Some(i) => {
                                        let mut _data = crate::MUTEX.lock().unwrap();
                                        sender.send((Some(i), Vec::from(b.as_ref()))).unwrap();
                                    }
                                }
                            }
                        };
                    }
                    Some(b) => match asset_id {
                        None => {
                            sender.send((None, b)).unwrap();
                        }
                        Some(i) => {
                            let mut _data = crate::MUTEX.lock().unwrap();
                            sender.send((Some(i), b)).unwrap();
                        }
                    },
                }
                debug!(
                    "{:?} - Image loading took {:?}",
                    thread::current().id(),
                    start.elapsed()
                );
            }
        });
    }
}
