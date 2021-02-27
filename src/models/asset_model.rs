use gio::prelude::*;
use glib::prelude::*;
use crate::models::ObjectWrapper;
use egs_api::api::types::AssetInfo;
use gio::prelude::ListStoreExtManual;
use glib::{Object, child_watch_add};
use std::cmp::Ordering;
use gtk::TreeModelFilter;
use gtk::ListStore;

#[derive(Clone)]
pub struct AssetModel {
    pub model: gio::ListStore,
}

impl AssetModel {
    pub fn new() -> Self {
        let model = gio::ListStore::new(ObjectWrapper::static_type());
        Self {
            model,
        }
    }

    fn sort_by_title(a: &Object, b: &Object) -> Ordering {
        match a.downcast_ref::<ObjectWrapper>() {
            None => { std::cmp::Ordering::Less }
            Some(first) => {
                match b.downcast_ref::<ObjectWrapper>() {
                    None => { std::cmp::Ordering::Greater }
                    Some(second) => {
                        let f: AssetInfo = first.deserialize();
                        let s: AssetInfo = second.deserialize();
                        f.title.to_lowercase().cmp(&s.title.to_lowercase())
                    }
                }
            }
        }
    }

    pub fn sort(&mut self) {
        self.model.sort(AssetModel::sort_by_title);
    }

    pub fn add_asset(&mut self, asset: AssetInfo, image: Vec<u8>) {
        if !self.index(&asset).is_some() {
            let object = ObjectWrapper::new(asset.clone(), hex::encode(image));
            self.model.insert_sorted(&object, AssetModel::sort_by_title);
        }
    }

    pub fn remove_asset(&mut self, asset: &AssetInfo) -> std::io::Result<()> {
        self.index(asset).map(|index| self.model.remove(index));
        Ok(())
    }

    fn index(&self, asset: &AssetInfo) -> Option<u32> {
        for i in 0..self.model.get_n_items() {
            let s = self.get_asset(i);

            if s.id == asset.id {
                return Some(i);
            }
        }
        None
    }

    pub fn get_asset(&self, index: u32) -> AssetInfo {
        let gobject = self.model.get_object(index).unwrap();
        let asset_object = gobject.downcast_ref::<ObjectWrapper>().expect("ObjectWrapper is of wrong type");
        asset_object.deserialize()
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.model.remove_all();
        Ok(())
    }
}