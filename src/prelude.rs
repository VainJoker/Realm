//! Prelude for the crate
pub use crate::{
    Map,
    Realme,
    RealmeBuilder,
    Value,
    adaptor::{
        Adaptor,
        parser::{
            Parser,
            cmd::CmdParser,
            env::EnvParser,
            ini::IniParser,
            json::JsonParser,
            json5::Json5Parser,
            ron::RonParser,
            ser::SerParser,
            toml::TomlParser,
            yaml::YamlParser,
        },
        source::{
            Source,
            cmd::CmdSource,
            env::EnvSource,
            file::FileSource,
            ser::SerSource,
            string::StringSource,
        },
    },
    utils::W,
};
