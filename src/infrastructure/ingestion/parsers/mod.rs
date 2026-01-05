//! Document parser implementations

mod html;
mod json;
mod markdown;
mod plain_text;

pub use html::HtmlParser;
pub use json::JsonParser;
pub use markdown::MarkdownParser;
pub use plain_text::PlainTextParser;
