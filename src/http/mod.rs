pub mod client;
pub mod curl;

pub use client::HttpClient;
pub use curl::{copy_to_clipboard, from_curl, to_curl};
