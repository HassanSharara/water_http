/// containing all request implementations
#[macro_use]
pub mod request;
/// containing all response implementations
mod response;

/// defining the most used status codes

pub mod status_code;
/// exporting all import response implementations
pub use response::*;

