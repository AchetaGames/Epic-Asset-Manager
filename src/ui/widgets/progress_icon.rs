use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

pub(crate) mod imp {
    use super::*;
    use gtk4::{graphene, gsk};
    use once_cell::sync::Lazy;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct ProgressIcon {
        pub fraction: RefCell<f64>,
        pub inverted: RefCell<bool>,
        pub clockwise: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgressIcon {
        const NAME: &'static str = "ProgressIcon";
        type Type = super::ProgressIcon;
        type ParentType = gtk4::Widget;

        fn new() -> Self {
            Self {
                fraction: RefCell::new(0.0),
                inverted: RefCell::new(false),
                clockwise: RefCell::new(true),
            }
        }
    }

    impl ObjectImpl for ProgressIcon {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_float(
                        "fraction",
                        "Progress",
                        "Progress of the icon",
                        0.0,
                        1.0,
                        0.0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpec::new_boolean(
                        "inverted",
                        "Inverted",
                        "Invert icon colors",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    glib::ParamSpec::new_boolean(
                        "clockwise",
                        "Clockwise",
                        "Direction of the icon",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "fraction" => obj.fraction().to_value(),
                "inverted" => obj.inverted().to_value(),
                "clockwise" => obj.clockwise().to_value(),
                _ => unreachable!(),
            }
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "fraction" => obj.set_fraction(value.get().unwrap()),
                "inverted" => obj.set_inverted(value.get().unwrap()),
                "clockwise" => obj.set_clockwise(value.get().unwrap()),
                _ => unreachable!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_valign(gtk4::Align::Center);
        }
    }

    impl WidgetImpl for ProgressIcon {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk4::Snapshot) {
            let size = widget.size() as f32;
            let radius = size / 2.0;
            let mut color = widget.style_context().color();
            let fraction = if widget.clockwise() {
                1.0 - widget.fraction()
            } else {
                widget.fraction()
            };

            let rect = graphene::Rect::new(0.0, 0.0, size, size);
            let circle = gsk::RoundedRect::from_rect(rect.clone(), radius);
            let center = graphene::Point::new(size / 2.0, size / 2.0);

            if widget.inverted() {
                color.alpha = 1.0;
            } else {
                color.alpha = 0.15;
            }
            let color_stop = gsk::ColorStop::new(fraction as f32, color);

            if widget.inverted() {
                color.alpha = 0.15;
            } else {
                color.alpha = 1.0;
            }
            let color_stop_end = gsk::ColorStop::new(fraction as f32, color);

            let rotation = 0.0;
            snapshot.push_rounded_clip(&circle);
            snapshot.append_conic_gradient(&rect, &center, rotation, &[color_stop, color_stop_end]);
            snapshot.pop();
        }

        fn measure(
            &self,
            widget: &Self::Type,
            _orientation: gtk4::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            (widget.size(), widget.size(), -1, -1)
        }
    }
}

glib::wrapper! {
    /// A widget to display the fraction of an operation.
    ///
    /// The [`NotificationExt::fraction()`] property of [`ProgressIcon`] is a float between 0.0 and 1.0,
    /// inclusive which denote that an operation has started or finished, respectively.
    ///
    /// **Implements**: [`ProgressIconExt`]
    pub struct ProgressIcon(ObjectSubclass<imp::ProgressIcon>)
        @extends gtk4::Widget;
}

impl Default for ProgressIcon {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl ProgressIcon {
    /// Creates a new [`ProgressIcon`].
    pub fn new() -> Self {
        Self::default()
    }

    fn size(&self) -> i32 {
        let width = self.width_request();
        let height = self.width_request();

        std::cmp::max(16, std::cmp::min(width, height))
    }
}

pub trait ProgressIconExt {
    /// Gets the child widget of `self`.
    ///
    /// Returns: the fraction of `self`
    fn fraction(&self) -> f64;

    /// Sets the fraction of `self`. `fraction` should be between 0.0 and 1.0, inclusive.
    fn set_fraction(&self, fraction: f64);

    /// Returns whether `self` is inverted.
    fn inverted(&self) -> bool;

    /// Sets whether `self` is inverted.
    fn set_inverted(&self, inverted: bool);

    /// Returns the completion direction of `self`.
    fn clockwise(&self) -> bool;

    /// Sets the fraction display direction of `self`.
    fn set_clockwise(&self, clockwise: bool);

    fn connect_fraction_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
    fn connect_inverted_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
    fn connect_clockwise_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
}

impl<W: IsA<ProgressIcon>> ProgressIconExt for W {
    fn fraction(&self) -> f64 {
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        *this.fraction.borrow()
    }
    fn set_fraction(&self, fraction: f64) {
        if (fraction - self.fraction()).abs() < f64::EPSILON {
            return;
        }
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        let clamped = fraction.clamp(0.0, 1.0);
        this.fraction.replace(clamped);
        self.as_ref().queue_draw();
        self.notify("fraction");
    }

    fn inverted(&self) -> bool {
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        *this.inverted.borrow()
    }
    fn set_inverted(&self, inverted: bool) {
        if inverted == self.inverted() {
            return;
        }
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        this.inverted.replace(inverted);
        self.as_ref().queue_draw();
        self.notify("inverted");
    }

    fn clockwise(&self) -> bool {
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        *this.clockwise.borrow()
    }
    fn set_clockwise(&self, clockwise: bool) {
        if clockwise == self.clockwise() {
            return;
        }
        let this = imp::ProgressIcon::from_instance(self.as_ref());
        this.clockwise.replace(clockwise);
        self.as_ref().queue_draw();
        self.notify("clockwise");
    }

    fn connect_fraction_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("fraction"), move |this, _| {
            f(this);
        })
    }
    fn connect_inverted_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("inverted"), move |this, _| {
            f(this);
        })
    }
    fn connect_clockwise_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("clockwise"), move |this, _| {
            f(this);
        })
    }
}
