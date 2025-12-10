use serde::{Deserialize, Deserializer, Serialize};
use simdnbt::{ToNbtTag, owned::NbtTag};

/// An RGB color.
#[derive(Debug, Deserialize, Clone, Copy, Eq, Hash, PartialEq)]
#[allow(missing_docs)]
pub struct RGBColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[allow(missing_docs)]
impl RGBColor {
    #[must_use]
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        RGBColor { red, green, blue }
    }
}

#[allow(missing_docs)]
impl Serialize for RGBColor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!(
            "#{:02X}{:02X}{:02X}",
            self.red, self.green, self.blue
        ))
    }
}

/// An ARGB color.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Deserialize)]
#[allow(missing_docs)]
pub struct ARGBColor {
    alpha: u8,
    red: u8,
    green: u8,
    blue: u8,
}

#[allow(missing_docs)]
impl ARGBColor {
    #[must_use]
    pub fn new(alpha: u8, red: u8, green: u8, blue: u8) -> Self {
        ARGBColor {
            alpha,
            red,
            green,
            blue,
        }
    }
}

/// Converts the ARGB color to an `NbtTag::Int` of the ARGB hex code as decimal.
///
/// Formula: (Alpha << 24) + (Red << 16) + (Green << 8) + Blue
#[allow(missing_docs)]
impl ToNbtTag for ARGBColor {
    fn to_nbt_tag(self) -> NbtTag {
        let value: i32 = (i32::from(self.alpha) << 24)
            | (i32::from(self.red) << 16)
            | (i32::from(self.green) << 8)
            | i32::from(self.blue);
        NbtTag::Int(value)
    }
}

#[allow(missing_docs)]
impl Serialize for ARGBColor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes([self.alpha, self.red, self.green, self.blue].as_ref())
    }
}

/// Named Minecraft color
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum NamedColor {
    Black = 0,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
}

#[allow(clippy::to_string_trait_impl)]
#[allow(missing_docs)]
impl ToString for NamedColor {
    fn to_string(&self) -> String {
        match self {
            NamedColor::Black => "black".to_string(),
            NamedColor::DarkBlue => "dark_blue".to_string(),
            NamedColor::DarkGreen => "dark_green".to_string(),
            NamedColor::DarkAqua => "dark_aqua".to_string(),
            NamedColor::DarkRed => "dark_red".to_string(),
            NamedColor::DarkPurple => "dark_purple".to_string(),
            NamedColor::Gold => "gold".to_string(),
            NamedColor::Gray => "gray".to_string(),
            NamedColor::DarkGray => "dark_gray".to_string(),
            NamedColor::Blue => "blue".to_string(),
            NamedColor::Green => "green".to_string(),
            NamedColor::Aqua => "aqua".to_string(),
            NamedColor::Red => "red".to_string(),
            NamedColor::LightPurple => "light_purple".to_string(),
            NamedColor::Yellow => "yellow".to_string(),
            NamedColor::White => "white".to_string(),
        }
    }
}

#[allow(missing_docs)]
impl TryFrom<&str> for NamedColor {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "black" => Ok(NamedColor::Black),
            "dark_blue" => Ok(NamedColor::DarkBlue),
            "dark_green" => Ok(NamedColor::DarkGreen),
            "dark_aqua" => Ok(NamedColor::DarkAqua),
            "dark_red" => Ok(NamedColor::DarkRed),
            "dark_purple" => Ok(NamedColor::DarkPurple),
            "gold" => Ok(NamedColor::Gold),
            "gray" => Ok(NamedColor::Gray),
            "dark_gray" => Ok(NamedColor::DarkGray),
            "blue" => Ok(NamedColor::Blue),
            "green" => Ok(NamedColor::Green),
            "aqua" => Ok(NamedColor::Aqua),
            "red" => Ok(NamedColor::Red),
            "light_purple" => Ok(NamedColor::LightPurple),
            "yellow" => Ok(NamedColor::Yellow),
            "white" => Ok(NamedColor::White),
            _ => Err(()),
        }
    }
}

/// A color.
#[derive(Default, Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
#[allow(missing_docs)]
pub enum Color {
    /// The default color for the text will be used, which varies by context
    /// (in some cases, it's white; in others, it's black; in still others, it
    /// is a shade of gray that isn't normally used on text).
    #[default]
    Reset,
    /// RGB Color
    Rgb(RGBColor),
    /// One of the 16 named Minecraft colors
    Named(NamedColor),
}

#[allow(missing_docs)]
impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        if s == "reset" {
            Ok(Color::Reset)
        } else if let Some(hex) = s.strip_prefix('#') {
            if s.len() != 7 {
                return Err(serde::de::Error::custom(
                    "Hex color must be in the format '#RRGGBB'",
                ));
            }

            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| serde::de::Error::custom("Invalid red component in hex color"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| serde::de::Error::custom("Invalid green component in hex color"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| serde::de::Error::custom("Invalid blue component in hex color"))?;

            Ok(Color::Rgb(RGBColor::new(r, g, b)))
        } else {
            Ok(Color::Named(NamedColor::try_from(s.as_str()).map_err(
                |()| serde::de::Error::custom("Invalid named color"),
            )?))
        }
    }
}

impl From<NamedColor> for Color {
    fn from(value: NamedColor) -> Self {
        Self::Named(value)
    }
}
