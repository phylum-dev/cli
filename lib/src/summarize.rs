
use crate::types::*;
use crate::render::Renderable;


pub trait Summarize: Renderable {
    fn summarize(&self) -> String {
      self.render()
    }
}

impl Summarize for RequestStatusResponse<PackageStatus> {
  fn summarize(&self) -> String {
    "TODO".to_string()
  }
}

impl Summarize for RequestStatusResponse<PackageStatusExtended> {
  fn summarize(&self) -> String {
    "TODO".to_string()
  }
}

impl Summarize for PackageStatus {
    fn summarize(&self) -> String {
        "TODO".to_string()
    }
}

impl Summarize for PackageStatusExtended {
    fn summarize(&self) -> String {
        "TODO".to_string()
    }
}

impl<T> Summarize for Vec<T>
where
    T: Renderable {}


impl Summarize for String {}
impl Summarize for CancelRequestResponse {}