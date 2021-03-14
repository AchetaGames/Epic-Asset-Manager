use glib::object::IsA;
use glib::translate::*;
use gtk::{IconSize, Image};

pub trait ImageExtCust: 'static {
    fn set_from_stock(&self, icon_name: Option<&str>, size: IconSize);
}

impl<O: IsA<Image>> ImageExtCust for O {
    fn set_from_stock(&self, icon_name: Option<&str>, size: IconSize) {
        unsafe {
            gtk_sys::gtk_image_set_from_stock(
                self.as_ref().to_glib_none().0,
                icon_name.to_glib_none().0,
                size.to_glib(),
            );
        }
    }
}
