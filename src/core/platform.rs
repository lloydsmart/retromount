use serde::de::{self, Deserializer};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    PlayStation,
    PlayStation2,
    SuperNintendo,
    MegaDrive,
    Dreamcast,
    GameCube,
    Wii,
    PC,
}

impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let normalized = raw.to_ascii_lowercase();

        match normalized.as_str() {
            "ps1" | "playstation" => Ok(Platform::PlayStation),
            "ps2" | "playstation2" => Ok(Platform::PlayStation2),

            "snes" | "supernintendo" | "super_nintendo" => Ok(Platform::SuperNintendo),

            "megadrive" | "mega_drive" | "genesis" => Ok(Platform::MegaDrive),

            "dreamcast" | "dc" => Ok(Platform::Dreamcast),

            "gamecube" | "gc" => Ok(Platform::GameCube),

            "wii" => Ok(Platform::Wii),

            "pc" => Ok(Platform::PC),

            _ => Err(de::Error::custom(format!("unknown platform '{}'", raw))),
        }
    }
}
