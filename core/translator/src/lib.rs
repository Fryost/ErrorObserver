// DO NOT YET USE!!!
// NOT IMPLEMENTED

pub trait Translator {
    fn translate(&self, request: &str) -> Option<String>;
}

pub struct NotYetAvailable;

impl Translator for NotYetAvailable {
    fn translate(&self, _request: &str) -> Option<String> {
        None
    }
}