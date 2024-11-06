use gtk4::prelude::IsA;
use gtk4::{self, prelude::*, Button};

pub trait ButtonEpic: 'static {
    fn with_icon_and_label(icon: &str, label: &str) -> Button;
}

impl<O: IsA<Button>> ButtonEpic for O {
    fn with_icon_and_label(icon: &str, label: &str) -> Button {
        let button = gtk4::Button::new();
        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        button_box.append(&gtk4::Image::from_icon_name(icon));
        button_box.append(&gtk4::Label::new(Some(label)));
        button.set_child(Some(&button_box));
        button
    }
}
