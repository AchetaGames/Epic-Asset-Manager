// use glib::subclass;
// use glib::subclass::prelude::*;
// use glib::translate::*;
// use gtk::prelude::*;
// use gtk::{gio, glib::clone, ResponseType};
// use serde::de::DeserializeOwned;
//
// mod imp {
//     use super::*;
//
//     use gio::subclass::prelude::ListModelImpl;
//     use glib::subclass;
//     use glib::subclass::prelude::*;
//     use std::cell::RefCell;
//
//     pub struct ObjectWrapper {
//         data: RefCell<Option<String>>,
//         thumbnail: RefCell<Option<String>>,
//     }
//
//     static PROPERTIES: [subclass::Property; 2] = [
//         subclass::Property("data", |name| {
//             glib::ParamSpec::string(
//                 name,
//                 "Data",
//                 "Data",
//                 None, // Default value
//                 glib::ParamFlags::READWRITE,
//             )
//         }),
//         subclass::Property("thumbnail", |name| {
//             glib::ParamSpec::string(
//                 name,
//                 "Thumbnail",
//                 "Thumbnail",
//                 None,
//                 glib::ParamFlags::READWRITE,
//             )
//         }),
//     ];
//
//     impl ObjectSubclass for ObjectWrapper {
//         const NAME: &'static str = "ObjectWrapper";
//         type Type = super::ObjectWrapper;
//         type ParentType = glib::Object;
//         type Interfaces = (gio::ListModel,);
//         type Instance = subclass::simple::InstanceStruct<Self>;
//         type Class = subclass::simple::ClassStruct<Self>;
//
//         glib_object_subclass!();
//
//         fn class_init(klass: &mut Self::Class) {
//             klass.install_properties(&PROPERTIES);
//         }
//
//         fn new() -> Self {
//             Self {
//                 data: RefCell::new(None),
//                 thumbnail: RefCell::new(None),
//             }
//         }
//     }
//
//     impl ObjectImpl for ObjectWrapper {
//         glib_object_impl!();
//
//         fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
//             let prop = &PROPERTIES[id];
//
//             match *prop {
//                 subclass::Property("data", ..) => {
//                     let data = value.get().unwrap();
//                     self.data.replace(data);
//                 }
//                 subclass::Property("thumbnail", ..) => {
//                     let thumbnail = value
//                         .get()
//                         .expect("type conformity checked by `Object::set_property`");
//                     self.thumbnail.replace(thumbnail);
//                 }
//                 _ => unimplemented!(),
//             }
//         }
//
//         fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
//             let prop = &PROPERTIES[id];
//
//             match *prop {
//                 subclass::Property("data", ..) => Ok(self.data.borrow().to_value()),
//                 subclass::Property("thumbnail", ..) => Ok(self.thumbnail.borrow().to_value()),
//                 _ => unimplemented!(),
//             }
//         }
//     }
//
//     impl ListModelImpl for ObjectWrapper {
//         fn get_item_type(&self, _list_model: &Self::Type) -> glib::Type {
//             RowData::static_type()
//         }
//         fn get_n_items(&self, _list_model: &Self::Type) -> u32 {
//             self.0.borrow().len() as u32
//         }
//         fn get_item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
//             self.0
//                 .borrow()
//                 .get(position as usize)
//                 .map(|o| o.clone().upcast::<glib::Object>())
//         }
//     }
// }
//
// glib_wrapper! {
//     pub struct ObjectWrapper(Object<subclass::simple::InstanceStruct<imp::ObjectWrapper>, subclass::simple::ClassStruct<imp::ObjectWrapper>, ObjectWrapperClass>);
//
//     match fn {
//         get_type => || imp::ObjectWrapper::get_type().to_glib(),
//     }
// }
//
// impl ObjectWrapper {
//     pub fn new<O>(object: O, image: String) -> ObjectWrapper
//     where
//         O: serde::ser::Serialize,
//     {
//         glib::Object::new(
//             Self::static_type(),
//             &[
//                 ("data", &serde_json::to_string(&object).unwrap()),
//                 ("thumbnail", &Some(image)),
//             ],
//         )
//         .unwrap()
//         .downcast()
//         .unwrap()
//     }
//
//     pub fn deserialize<O>(&self) -> O
//     where
//         O: DeserializeOwned,
//     {
//         let data = self.get_property("data").unwrap().get::<String>().unwrap();
//         serde_json::from_str(&data.unwrap()).unwrap()
//     }
//
//     pub fn image(&self) -> Vec<u8> {
//         match self
//             .get_property("thumbnail")
//             .unwrap()
//             .get::<String>()
//             .unwrap()
//         {
//             None => {
//                 vec![]
//             }
//             Some(img) => match hex::decode(img) {
//                 Ok(v) => v,
//                 Err(_) => {
//                     vec![]
//                 }
//             },
//         }
//     }
// }

use crate::models::row_data::RowData;
use egs_api::api::types::AssetInfo;
use gio::glib::subclass::types::ObjectSubclass;
use gio::ListModelExt;
use gtk::gio;

mod imp {
    use super::*;
    use gio::subclass::prelude::ListModelImpl;
    use glib::subclass::prelude::*;
    use glib::{subclass, Cast, StaticType};
    use std::cell::RefCell;

    #[derive(Debug)]
    pub struct Model(pub RefCell<Vec<RowData>>);

    // Basic declaration of our type for the GObject type system
    impl ObjectSubclass for Model {
        const NAME: &'static str = "Model";
        type Type = super::Model;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        // Called once at the very beginning of instantiation
        fn new() -> Self {
            Self(RefCell::new(Vec::new()))
        }
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
                .binary_search_by(|probe| {
                    let f: AssetInfo = probe.deserialize::<AssetInfo>();
                    let s: AssetInfo = obj.deserialize::<AssetInfo>();
                    f.title.to_lowercase().cmp(&s.title.to_lowercase())
                })
                .unwrap_or_else(|e| e);
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
