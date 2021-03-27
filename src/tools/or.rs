pub trait Or: Sized {
    fn or(self, other: Self) -> Self;
}

impl<'a> Or for &'a str {
    fn or(self, other: &'a str) -> &'a str {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

impl<'a> Or for &'a String {
    fn or(self, other: &'a String) -> &'a String {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}
