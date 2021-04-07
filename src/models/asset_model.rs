use crate::glib::subclass::types::ObjectSubclassExt;
use crate::models::row_data::RowData;
use gio::ListModelExt;
use gtk::gio;

mod imp {
    use super::*;
    use gio::subclass::prelude::ListModelImpl;
    use glib::subclass::prelude::*;
    use glib::{Cast, StaticType};
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct Model(pub RefCell<Vec<RowData>>);

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for Model {
        const NAME: &'static str = "Model";
        type Type = super::Model;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for Model {}

    impl ListModelImpl for Model {
        fn get_item_type(&self, _list_model: &Self::Type) -> glib::Type {
            RowData::static_type()
        }
        fn get_n_items(&self, _list_model: &Self::Type) -> u32 {
            self.0.borrow().len() as u32
        }
        fn get_item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
            self.0
                .borrow()
                .get(position as usize)
                .map(|o| o.clone().upcast::<glib::Object>())
        }
    }
}

// Public part of the Model type.
glib::wrapper! {
    pub struct Model(ObjectSubclass<imp::Model>) @implements gio::ListModel;
}

// Constructor for new instances. This simply calls glib::Object::new()
impl Model {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Model {
        glib::Object::new(&[]).expect("Failed to create Model")
    }

    pub fn append(&self, obj: &RowData) {
        let self_ = imp::Model::from_instance(self);
        let index = {
            // Borrow the data only once and ensure the borrow guard is dropped
            // before we emit the items_changed signal because the view
            // could call get_item / get_n_item from the signal handler to update its state
            let mut data = self_.0.borrow_mut();
            let pos = data
                .binary_search_by(|probe| probe.id().to_lowercase().cmp(&obj.id().to_lowercase()))
                .unwrap_or_else(|e| e);
            if let Some(d) = data.get(pos) {
                if d.id() == obj.id() {
                    return;
                }
            };
            data.insert(pos, obj.clone());
            pos
        };
        // Emits a signal that 1 item was added, 0 removed at the position index
        self.items_changed(index as u32, 0, 1);
    }

    pub fn remove(&self, index: u32) {
        let self_ = imp::Model::from_instance(self);
        self_.0.borrow_mut().remove(index as usize);
        // Emits a signal that 1 item was removed, 0 added at the position index
        self.items_changed(index, 1, 0);
    }
}
