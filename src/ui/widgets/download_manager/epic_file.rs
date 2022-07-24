use crate::ui::widgets::download_manager::{download_item, Msg, ThreadMessages};
use crate::ui::widgets::logged_in::engines::epic_download::Blob;
use glib::clone;
use gtk4::glib;
use gtk4::glib::Sender;
use gtk4::glib::{MainContext, ObjectExt, PRIORITY_DEFAULT};
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use regex::Regex;
use reqwest::Url;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

pub(crate) trait EpicFile {
    fn perform_file_download(&self, _url: &str, _size: i64, _version: &str) {
        unimplemented!()
    }

    fn download_engine_from_epic(&self, _version: &str) {
        unimplemented!()
    }

    fn file_finished(&self, _item: &download_item::EpicDownloadItem) {
        unimplemented!()
    }

    fn start_version_file_download(&self, _version: &str) {
        unimplemented!()
    }

    fn engine_target_directory(&self) -> Option<PathBuf> {
        unimplemented!()
    }

    fn epic_download_progress(&self, _version: &str, _progress: u64) {
        unimplemented!()
    }
}

impl EpicFile for crate::ui::widgets::download_manager::EpicDownloadManager {
    fn perform_file_download(&self, url: &str, size: i64, version: &str) {
        let self_ = self.imp();
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.set_property("status", "waiting for download slot".to_string());
        item.set_total_size(size as u128);
        item.set_total_files(1);
        let (send, recv) = std::sync::mpsc::channel::<super::ThreadMessages>();
        self.add_thread_sender(version.to_string(), send);
        let sender = self_.sender.clone();
        let link = Url::parse(url).expect("Valid URL");
        let ver = version.to_string();
        let mut p = self
            .engine_target_directory()
            .expect("Invalid Target directory");
        p.push(version);
        self_.download_pool.execute(move || {
            if let Ok(w) = crate::RUNNING.read() {
                if !*w {
                    return;
                }
            };
            if let Ok(m) = recv.try_recv() {
                process_epic_thread_message(ver, &sender, &m);
                return;
            }
            debug!(
                "Downloading engine {} from {} to {:?}",
                ver,
                link.to_string(),
                p
            );
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            let mut client = match reqwest::blocking::get(link.clone()) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to start Engine download: {}", e);
                    return;
                }
            };
            let mut buffer: [u8; 1024] = [0; 1024];
            let mut downloaded: u64 = 0;
            let mut file = File::create(&p).unwrap();
            loop {
                if let Ok(m) = recv.try_recv() {
                    process_epic_thread_message(ver, &sender, &m);
                    return;
                }
                match client.read(&mut buffer) {
                    Ok(size) => {
                        if let Ok(m) = recv.try_recv() {
                            process_epic_thread_message(ver, &sender, &m);
                            return;
                        }
                        if size > 0 {
                            downloaded += size as u64;
                            file.write_all(&buffer[0..size]).unwrap();
                            sender
                                .send(super::Msg::EpicDownloadProgress(ver.clone(), size as u64))
                                .unwrap();
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Download error: {:?}", e);
                        break;
                    }
                }
            }
        });
    }

    fn engine_target_directory(&self) -> Option<PathBuf> {
        let self_ = self.imp();

        let mut target = match self_.settings.strv("unreal-engine-directories").get(0) {
            None => {
                if let Some(w) = self_.window.get() {
                    w.add_notification(
                        "missing engine config",
                        "Unable to download engine missing Unreal Engine Directories configuration",
                        gtk4::MessageType::Error,
                    );
                }
                return None;
            }
            Some(p) => PathBuf::from(p),
        };
        target.push("epic");
        Some(target)
    }

    fn download_engine_from_epic(&self, version: &str) {
        debug!("Initializing epic engine download of {}", version);
        let self_ = self.imp();
        let re = Regex::new(r"(\d\.\d+.\d+)").unwrap();
        let mut items = self_.download_items.borrow_mut();
        let item = match items.get_mut(version) {
            None => {
                let item =
                    crate::ui::widgets::download_manager::download_item::EpicDownloadItem::new();
                debug!("Adding item to the list under: {}", version);
                items.insert(version.to_string(), item.clone());
                item
            }
            Some(_) => {
                return;
            }
        };
        if let Some(w) = self_.window.get() {
            item.set_window(w);
        }
        item.set_download_manager(self);
        item.set_property("version", version);
        item.set_property("item-type", download_item::ItemType::Epic);
        for cap in re.captures_iter(version) {
            item.set_property("label", cap[1].to_string());
        }
        item.set_property("status", "initializing...".to_string());

        item.connect_local(
            "finished",
            false,
            clone!(@weak self as edm, @weak item => @default-return None, move |_| {
                edm.file_finished(&item);
                None
            }),
        );

        item.set_property("thumbnail", Some(gtk4::gdk::Texture::from_resource(
            "/io/github/achetagames/epic_asset_manager/icons/scalable/emblems/ue-logo-symbolic.svg",
        )));

        self_.downloads.append(&item);

        self.set_property("has-items", self_.downloads.first_child().is_some());
        self.start_version_file_download(version);
    }

    fn start_version_file_download(&self, version: &str) {
        let self_ = self.imp();
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

        let vers = version.to_string();
        receiver.attach(
            None,
            clone!(@weak self as dm => @default-panic, move |v| {
                let self_ = dm.imp();
                let s = self_.sender.clone();
                if let Some(ver) = filter_versions(v, &vers) {
                    s.send(Msg::EpicDownloadStart(ver.name, ver.url, ver.size)).unwrap();
                }
                glib::Continue(false)
            }),
        );
        if let Some(window) = self_.window.get() {
            let win_ = window.imp();
            let l = win_.logged_in_stack.imp();
            let e = l.engines.imp();
            let s = e.side.imp();
            let i = s.install.imp();
            let f = i.epic.clone();
            f.get_versions(sender);
        }
    }

    fn file_finished(&self, item: &download_item::EpicDownloadItem) {
        self.finish(item);
    }

    fn epic_download_progress(&self, version: &str, progress: u64) {
        let item = match self.get_item(version) {
            None => {
                return;
            }
            Some(i) => i,
        };
        item.add_downloaded_size(progress as u128);

        self.emit_by_name::<()>("tick", &[]);
    }
}

fn filter_versions(
    versions: Vec<crate::ui::widgets::logged_in::engines::epic_download::Blob>,
    version: &str,
) -> Option<Blob> {
    for ver in versions {
        if ver.name.eq(version) {
            return Some(ver);
        }
    }
    None
}

fn process_epic_thread_message(version: String, sender: &Sender<Msg>, m: &ThreadMessages) {
    match m {
        ThreadMessages::Cancel => {
            sender.send(Msg::EpicCanceled(version)).unwrap();
        }
        ThreadMessages::Pause => {
            sender.send(Msg::EpicPaused(version)).unwrap();
        }
    }
}
