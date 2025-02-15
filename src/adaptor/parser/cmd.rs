use nom::{
    IResult,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{
        char,
        multispace0,
        space0,
    },
    combinator::map,
    multi::separated_list0,
    sequence::{
        delimited,
        preceded,
        separated_pair,
        terminated,
    },
};

use crate::{
    Error,
    prelude::*,
};

/// A parser for command-line style key-value pairs.
#[derive(Debug, Default)]
pub struct CmdParser;

impl CmdParser {
    /// Parses a key-value pair separated by an '=' character.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice that holds the input to be parsed.
    ///
    /// # Returns
    ///
    /// * `IResult` - A result containing the remaining input and a tuple of the
    ///   parsed key and value.
    fn parse_pair(input: &str) -> IResult<&str, (String, Value)> {
        separated_pair(Self::parse_key, char('='), Self::parse_value)(input)
    }

    /// Parses a key which can contain alphanumeric characters, dots, and
    /// underscores.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice that holds the input to be parsed.
    ///
    /// # Returns
    ///
    /// * `IResult` - A result containing the remaining input and the parsed key
    ///   as a string.
    fn parse_key(input: &str) -> IResult<&str, String> {
        map(
            take_while1(|c: char| c.is_alphanumeric() || c == '.' || c == '_'),
            |s: &str| s.trim().to_string(),
        )(input)
    }

    /// Parses a value which can be an array, a quoted string, or an unquoted
    /// string.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice that holds the input to be parsed.
    ///
    /// # Returns
    ///
    /// * `IResult` - A result containing the remaining input and the parsed
    ///   value.
    fn parse_value(input: &str) -> IResult<&str, Value> {
        alt((
            Self::parse_array,
            // CHECK: is this necessary?
            // for something like "He said, Hello, World!" it is needed
            map(
                delimited(char('"'), take_while1(|c| c != '"'), char('"')),
                |s: &str| Value::String(s.trim().to_string()),
            ),
            map(take_while1(|c| c != ',' && c != ']'), |s: &str| {
                Value::String(s.trim().to_string())
            }),
        ))(input)
    }

    /// Parses an array of values separated by semicolons and enclosed in square
    /// brackets.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice that holds the input to be parsed.
    ///
    /// # Returns
    ///
    /// * `IResult` - A result containing the remaining input and the parsed
    ///   array as a `Value::Array`.
    fn parse_array(input: &str) -> IResult<&str, Value> {
        let (input, _) = multispace0(input)?;
        delimited(
            char('['),
            map(
                separated_list0(
                    preceded(space0, char(';')),
                    alt((
                        Self::parse_array,
                        map(
                            take_while1(|c| c != ';' && c != ']'),
                            |s: &str| Value::String(s.trim().to_string()),
                        ),
                    )),
                ),
                Value::Array,
            ),
            char(']'),
        )(input)
    }

    /// Parses a command string into a map of keys and values.
    ///
    /// # Arguments
    ///
    /// * `input` - A string slice that holds the input to be parsed.
    ///
    /// # Returns
    ///
    /// * `IResult` - A result containing the remaining input and the parsed
    ///   map.
    fn parse_cmd(input: &str) -> IResult<&str, Map<String, Value>> {
        let (input, pairs) = separated_list0(
            terminated(char(','), multispace0),
            Self::parse_pair,
        )(input)?;

        let map =
            pairs.into_iter().fold(Map::new(), |mut acc, (key, value)| {
                let parts: Vec<&str> = key.split('.').collect();
                Self::insert_nested(&mut acc, &parts, value);
                acc
            });

        Ok((input, map))
    }

    fn insert_nested(
        map: &mut Map<String, Value>,
        parts: &[&str],
        value: Value,
    ) {
        match parts {
            [head] => {
                map.insert((*head).to_string(), value);
            }
            [head, tail @ ..] => {
                let entry = map
                    .entry((*head).to_string())
                    .or_insert_with(|| Value::Table(Map::new()));
                if let Value::Table(ref mut nested_map) = entry {
                    Self::insert_nested(nested_map, tail, value);
                }
            }
            [] => {}
        }
    }
}

impl<T: AsRef<str>> Parser<T> for CmdParser {
    type Item = Value;
    type Error = Error;

    /// Parses the input string into a `Value` item.
    ///
    /// # Arguments
    ///
    /// * `args` - A generic type that can be converted to a string slice.
    ///
    /// # Returns
    ///
    /// * `Result` - A result containing the parsed `Value` or a `Error`.
    ///
    /// # Examples
    /// ```rust
    /// use realme::prelude::*;
    /// let cmd_str = "age=30, name.first=John, name.last=Doe";
    /// let result = CmdParser::parse(cmd_str);
    /// assert!(result.is_ok());
    /// ```
    fn parse(args: T) -> Result<Self::Item, Self::Error> {
        let args = args.as_ref().trim();
        if args.is_empty() {
            return Ok(Value::Table(Map::new()));
        }
        match Self::parse_cmd(args) {
            Ok((_, map)) => Ok(Value::Table(map)),
            Err(_) => Err(Error::new_parse_error(
                args.to_string(),
                "Failed to parse from cmd".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const FINAL_CMD: &str = r#"

       age=30,

       name.first=John,

       name.last=Doe,

       empty= ,

       address.city=New York,

       address.zip=10001,

       created_at=2024-02-20T12:00:00Z,
       extra=and.and,

       email=john.doe@example.com,

       website=https://example.com,
       quote="Life is beautiful",

       escaped_value="He said, Hello, World!",

       skills=[Go; Rust; Python; Bash Scripting],
    "#;

    #[test]

    fn test_parse() -> anyhow::Result<()> {
        let result = CmdParser::parse(FINAL_CMD.to_string())?;

        let expected = Value::Table(Map::from_iter([
            ("age".to_string(), Value::String("30".to_string())),
            (
                "name".to_string(),
                Value::Table(Map::from_iter([
                    ("first".to_string(), Value::String("John".to_string())),
                    ("last".to_string(), Value::String("Doe".to_string())),
                ])),
            ),
            ("empty".to_string(), Value::String(String::new())),
            (
                "address".to_string(),
                Value::Table(Map::from_iter([
                    ("city".to_string(), Value::String("New York".to_string())),
                    ("zip".to_string(), Value::String("10001".to_string())),
                ])),
            ),
            (
                "created_at".to_string(),
                Value::String("2024-02-20T12:00:00Z".to_string()),
            ),
            ("extra".to_string(), Value::String("and.and".to_string())),
            (
                "email".to_string(),
                Value::String("john.doe@example.com".to_string()),
            ),
            (
                "website".to_string(),
                Value::String("https://example.com".to_string()),
            ),
            (
                "quote".to_string(),
                Value::String("Life is beautiful".to_string()),
            ),
            (
                "escaped_value".to_string(),
                Value::String("He said, Hello, World!".to_string()),
            ),
            (
                "skills".to_string(),
                Value::Array(vec![
                    Value::String("Go".to_string()),
                    Value::String("Rust".to_string()),
                    Value::String("Python".to_string()),
                    Value::String("Bash Scripting".to_string()),
                ]),
            ),
        ]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]

    fn test_parse_simple() -> anyhow::Result<()> {
        let result = CmdParser::parse("age=30".to_string())?;
        let expected = Value::Table(Map::from_iter([(
            "age".to_string(),
            Value::String("30".to_string()),
        )]));

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_array() -> anyhow::Result<()> {
        let result = CmdParser::parse(
            "skills=[Go; Rust; Python; Bash Scripting]".to_string(),
        )?;
        let expected = Value::Table(Map::from_iter([(
            "skills".to_string(),
            Value::Array(vec![
                Value::String("Go".to_string()),
                Value::String("Rust".to_string()),
                Value::String("Python".to_string()),
                Value::String("Bash Scripting".to_string()),
            ]),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]

    fn test_parse_empty_array() -> anyhow::Result<()> {
        let result = CmdParser::parse("skills=[]".to_string())?;

        let expected = Value::Table(Map::from_iter([(
            "skills".to_string(),
            Value::Array(vec![]),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_array_of_array() -> anyhow::Result<()> {
        let result = CmdParser::parse(
            "nested_array=[[12]; [3; four; [5; 6]]]".to_string(),
        )?;
        let expected = Value::Table(Map::from_iter([(
            "nested_array".to_string(),
            Value::Array(vec![
                Value::Array(vec![Value::String("12".to_string())]),
                Value::Array(vec![
                    Value::String("3".to_string()),
                    Value::String("four".to_string()),
                    Value::Array(vec![
                        Value::String("5".to_string()),
                        Value::String("6".to_string()),
                    ]),
                ]),
            ]),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_table() -> anyhow::Result<()> {
        let result =
            CmdParser::parse("name.first=John, name.last=Doe".to_string())?;

        let expected = Value::Table(Map::from_iter([(
            "name".to_string(),
            Value::Table(Map::from_iter([
                ("first".to_string(), Value::String("John".to_string())),
                ("last".to_string(), Value::String("Doe".to_string())),
            ])),
        )]));

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_table_and_array() -> anyhow::Result<()> {
        let result = CmdParser::parse(
            "skills=[Go;Rust;Python;Bash Scripting], name.first=John, \
             name.last=Doe"
                .to_string(),
        )?;

        let expected = Value::Table(Map::from_iter([
            (
                "skills".to_string(),
                Value::Array(vec![
                    Value::String("Go".to_string()),
                    Value::String("Rust".to_string()),
                    Value::String("Python".to_string()),
                    Value::String("Bash Scripting".to_string()),
                ]),
            ),
            (
                "name".to_string(),
                Value::Table(Map::from_iter([
                    ("first".to_string(), Value::String("John".to_string())),
                    ("last".to_string(), Value::String("Doe".to_string())),
                ])),
            ),
        ]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_empty_value() -> anyhow::Result<()> {
        let result = CmdParser::parse("empty=\" \"".to_string())?;
        let expected = Value::Table(Map::from_iter([(
            "empty".to_string(),
            Value::String(String::new()),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_value_with_space() -> anyhow::Result<()> {
        let result = CmdParser::parse("address.city=New York".to_string())?;

        let expected = Value::Table(Map::from_iter([(
            "address".to_string(),
            Value::Table(Map::from_iter([(
                "city".to_string(),
                Value::String("New York".to_string()),
            )])),
        )]));

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_key_with_underscore() -> anyhow::Result<()> {
        let result = CmdParser::parse("nest_value=a_b_c".to_string())?;

        let expected = Value::Table(Map::from_iter([(
            "nest_value".to_string(),
            Value::String("a_b_c".to_string()),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_value_with_quote() -> anyhow::Result<()> {
        let result =
            CmdParser::parse("quote=\"Life is beautiful\"".to_string())?;
        let expected = Value::Table(Map::from_iter([(
            "quote".to_string(),
            Value::String("Life is beautiful".to_string()),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_parse_escaped_value() -> anyhow::Result<()> {
        let result = CmdParser::parse(
            "escaped_value=\"He said, Hello, World!\"".to_string(),
        )?;
        let expected = Value::Table(Map::from_iter([(
            "escaped_value".to_string(),
            Value::String("He said, Hello, World!".to_string()),
        )]));
        assert_eq!(result, expected);
        Ok(())
    }
}
