pub trait Refresh {
    fn run_refresh(&self) {
        unimplemented!()
    }
    fn can_be_refreshed(&self) -> bool {
        unimplemented!()
    }
    fn refresh_state_changed(&self) {
        unimplemented!()
    }
}
