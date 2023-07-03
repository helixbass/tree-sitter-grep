// derived from https://github.com/BurntSushi/ripgrep/blob/master/crates/printer/src/color.rs

use std::{error, fmt, str::FromStr};

use termcolor::{Color, ColorSpec, ParseColorError};

#[allow(dead_code)]
pub fn default_color_specs() -> Vec<UserColorSpec> {
    vec![
        #[cfg(unix)]
        "path:fg:magenta".parse().unwrap(),
        #[cfg(windows)]
        "path:fg:cyan".parse().unwrap(),
        "line:fg:green".parse().unwrap(),
        "match:fg:red".parse().unwrap(),
        "match:style:bold".parse().unwrap(),
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColorError {
    UnrecognizedOutType(String),
    UnrecognizedSpecType(String),
    UnrecognizedColor(String, String),
    UnrecognizedStyle(String),
    InvalidFormat(String),
}

impl error::Error for ColorError {
    fn description(&self) -> &str {
        match *self {
            ColorError::UnrecognizedOutType(_) => "unrecognized output type",
            ColorError::UnrecognizedSpecType(_) => "unrecognized spec type",
            ColorError::UnrecognizedColor(_, _) => "unrecognized color name",
            ColorError::UnrecognizedStyle(_) => "unrecognized style attribute",
            ColorError::InvalidFormat(_) => "invalid color spec",
        }
    }
}

impl ColorError {
    fn from_parse_error(err: ParseColorError) -> ColorError {
        ColorError::UnrecognizedColor(err.invalid().to_string(), err.to_string())
    }
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ColorError::UnrecognizedOutType(ref name) => write!(
                f,
                "unrecognized output type '{}'. Choose from: \
                     path, line, column, match.",
                name,
            ),
            ColorError::UnrecognizedSpecType(ref name) => write!(
                f,
                "unrecognized spec type '{}'. Choose from: \
                     fg, bg, style, none.",
                name,
            ),
            ColorError::UnrecognizedColor(_, ref msg) => write!(f, "{}", msg),
            ColorError::UnrecognizedStyle(ref name) => write!(
                f,
                "unrecognized style attribute '{}'. Choose from: \
                     nobold, bold, nointense, intense, nounderline, \
                     underline.",
                name,
            ),
            ColorError::InvalidFormat(ref original) => write!(
                f,
                "invalid color spec format: '{}'. Valid format \
                     is '(path|line|column|match):(fg|bg|style):(value)'.",
                original,
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ColorSpecs {
    path: ColorSpec,
    line: ColorSpec,
    column: ColorSpec,
    matched: ColorSpec,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserColorSpec {
    ty: OutType,
    value: SpecValue,
}

impl UserColorSpec {
    #[allow(dead_code)]
    pub fn to_color_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::default();
        self.value.merge_into(&mut spec);
        spec
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SpecValue {
    None,
    Fg(Color),
    Bg(Color),
    Style(Style),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OutType {
    Path,
    Line,
    Column,
    Match,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SpecType {
    Fg,
    Bg,
    Style,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Style {
    Bold,
    NoBold,
    Intense,
    NoIntense,
    Underline,
    NoUnderline,
}

impl ColorSpecs {
    pub fn new(specs: &[UserColorSpec]) -> ColorSpecs {
        let mut merged = ColorSpecs::default();
        for spec in specs {
            match spec.ty {
                OutType::Path => spec.merge_into(&mut merged.path),
                OutType::Line => spec.merge_into(&mut merged.line),
                OutType::Column => spec.merge_into(&mut merged.column),
                OutType::Match => spec.merge_into(&mut merged.matched),
            }
        }
        merged
    }

    #[allow(dead_code)]
    pub fn default_with_color() -> ColorSpecs {
        ColorSpecs::new(&default_color_specs())
    }

    pub fn path(&self) -> &ColorSpec {
        &self.path
    }

    pub fn line(&self) -> &ColorSpec {
        &self.line
    }

    pub fn column(&self) -> &ColorSpec {
        &self.column
    }

    pub fn matched(&self) -> &ColorSpec {
        &self.matched
    }
}

impl UserColorSpec {
    fn merge_into(&self, cspec: &mut ColorSpec) {
        self.value.merge_into(cspec);
    }
}

impl SpecValue {
    fn merge_into(&self, cspec: &mut ColorSpec) {
        match *self {
            SpecValue::None => cspec.clear(),
            SpecValue::Fg(ref color) => {
                cspec.set_fg(Some(color.clone()));
            }
            SpecValue::Bg(ref color) => {
                cspec.set_bg(Some(color.clone()));
            }
            SpecValue::Style(ref style) => match *style {
                Style::Bold => {
                    cspec.set_bold(true);
                }
                Style::NoBold => {
                    cspec.set_bold(false);
                }
                Style::Intense => {
                    cspec.set_intense(true);
                }
                Style::NoIntense => {
                    cspec.set_intense(false);
                }
                Style::Underline => {
                    cspec.set_underline(true);
                }
                Style::NoUnderline => {
                    cspec.set_underline(false);
                }
            },
        }
    }
}

impl FromStr for UserColorSpec {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<UserColorSpec, ColorError> {
        let pieces: Vec<&str> = s.split(':').collect();
        if pieces.len() <= 1 || pieces.len() > 3 {
            return Err(ColorError::InvalidFormat(s.to_string()));
        }
        let otype: OutType = pieces[0].parse()?;
        match pieces[1].parse()? {
            SpecType::None => Ok(UserColorSpec {
                ty: otype,
                value: SpecValue::None,
            }),
            SpecType::Style => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let style: Style = pieces[2].parse()?;
                Ok(UserColorSpec {
                    ty: otype,
                    value: SpecValue::Style(style),
                })
            }
            SpecType::Fg => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let color: Color = pieces[2].parse().map_err(ColorError::from_parse_error)?;
                Ok(UserColorSpec {
                    ty: otype,
                    value: SpecValue::Fg(color),
                })
            }
            SpecType::Bg => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let color: Color = pieces[2].parse().map_err(ColorError::from_parse_error)?;
                Ok(UserColorSpec {
                    ty: otype,
                    value: SpecValue::Bg(color),
                })
            }
        }
    }
}

impl FromStr for OutType {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<OutType, ColorError> {
        match &*s.to_lowercase() {
            "path" => Ok(OutType::Path),
            "line" => Ok(OutType::Line),
            "column" => Ok(OutType::Column),
            "match" => Ok(OutType::Match),
            _ => Err(ColorError::UnrecognizedOutType(s.to_string())),
        }
    }
}

impl FromStr for SpecType {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<SpecType, ColorError> {
        match &*s.to_lowercase() {
            "fg" => Ok(SpecType::Fg),
            "bg" => Ok(SpecType::Bg),
            "style" => Ok(SpecType::Style),
            "none" => Ok(SpecType::None),
            _ => Err(ColorError::UnrecognizedSpecType(s.to_string())),
        }
    }
}

impl FromStr for Style {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<Style, ColorError> {
        match &*s.to_lowercase() {
            "bold" => Ok(Style::Bold),
            "nobold" => Ok(Style::NoBold),
            "intense" => Ok(Style::Intense),
            "nointense" => Ok(Style::NoIntense),
            "underline" => Ok(Style::Underline),
            "nounderline" => Ok(Style::NoUnderline),
            _ => Err(ColorError::UnrecognizedStyle(s.to_string())),
        }
    }
}
