/// A parser for JSON data.
///
/// This struct implements the `Parser` trait for parsing JSON strings into
/// `serde_json::Value`.
use crate::{
    Error,
    prelude::*,
};

#[derive(Debug)]
pub struct JsonParser;

impl<T: AsRef<str>> Parser<T> for JsonParser {
    type Item = serde_json::Value;
    type Error = Error;

    /// Parses a JSON string into a `serde_json::Value`.
    ///
    /// # Arguments
    ///
    /// * `args` - A string-like type that can be converted to a string slice.
    ///
    /// # Returns
    ///
    /// * `Result<Self::Item, Self::Error>` - A Result containing either the
    ///   parsed JSON value or a `Error` if parsing fails.
    /// # Examples
    /// ```rust
    /// use realme::prelude::*;
    /// let json_str = r#"{"name": "John", "age": 30}"#;
    /// let result = JsonParser::parse(json_str);
    /// assert!(result.is_ok());
    /// ```
    fn parse(args: T) -> Result<Self::Item, Self::Error> {
        let args = args.as_ref().trim();
        serde_json::from_str(args).map_err(|e| {
            Error::new_parse_error(args.to_string(), e.to_string())
        })
    }
}
