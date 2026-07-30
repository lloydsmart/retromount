use std::collections::BTreeSet;
use std::fmt;
use std::fs::File;
use std::path::Path;

use serde::Deserialize;

use crate::core::content::{DiscMedia, Platform};
use crate::output::capabilities::{CapabilityFeature, ContentType, Format};
use crate::output::presentation_spec::{
    ArtifactSpec, FileRuleSpec, LayoutSpec, NamingSpec, PresentationSpec, SelectSpec,
};

pub const PRESENTATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationDocument {
    pub name: String,
    pub spec: PresentationSpec,
}

#[derive(Debug)]
pub enum PresentationFileError {
    Read(std::io::Error),
    Parse(serde_yaml::Error),
    Invalid(String),
}

impl fmt::Display for PresentationFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "failed to read presentation: {error}"),
            Self::Parse(error) => write!(formatter, "failed to parse presentation YAML: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid presentation: {message}"),
        }
    }
}

impl std::error::Error for PresentationFileError {}

pub fn load_presentation_file(path: &Path) -> Result<PresentationDocument, PresentationFileError> {
    let file = File::open(path).map_err(PresentationFileError::Read)?;
    let document: SerializedPresentation =
        serde_yaml::from_reader(file).map_err(PresentationFileError::Parse)?;
    document.try_into()
}

pub fn parse_presentation_yaml(yaml: &str) -> Result<PresentationDocument, PresentationFileError> {
    let document: SerializedPresentation =
        serde_yaml::from_str(yaml).map_err(PresentationFileError::Parse)?;
    document.try_into()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializedPresentation {
    version: u32,
    name: String,
    layout: SerializedLayout,
    files: Vec<SerializedFileRule>,
}

impl TryFrom<SerializedPresentation> for PresentationDocument {
    type Error = PresentationFileError;

    fn try_from(document: SerializedPresentation) -> Result<Self, Self::Error> {
        if document.version != PRESENTATION_SCHEMA_VERSION {
            return Err(PresentationFileError::Invalid(format!(
                "unsupported schema version {}; expected {}",
                document.version, PRESENTATION_SCHEMA_VERSION
            )));
        }

        let name = document.name.trim();
        if name.is_empty() {
            return Err(PresentationFileError::Invalid(
                "name must not be empty".to_string(),
            ));
        }
        if document.files.is_empty() {
            return Err(PresentationFileError::Invalid(
                "files must contain at least one rule".to_string(),
            ));
        }

        let layout = document.layout.try_into()?;
        let files = document
            .files
            .into_iter()
            .enumerate()
            .map(|(index, rule)| {
                rule.into_rule()
                    .map_err(|message| invalid_file_rule(index, message))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PresentationDocument {
            name: name.to_string(),
            spec: PresentationSpec::new(layout, files),
        })
    }
}

fn invalid_file_rule(index: usize, message: String) -> PresentationFileError {
    PresentationFileError::Invalid(format!("files[{index}]: {message}"))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SerializedLayout {
    Flat,
    GroupedByPlatformAndGame,
    LiteralRoot { path: String },
}

impl TryFrom<SerializedLayout> for LayoutSpec {
    type Error = PresentationFileError;

    fn try_from(layout: SerializedLayout) -> Result<Self, Self::Error> {
        match layout {
            SerializedLayout::Flat => Ok(Self::Flat),
            SerializedLayout::GroupedByPlatformAndGame => Ok(Self::GroupedByPlatformAndGame),
            SerializedLayout::LiteralRoot { path } if path.trim().is_empty() => Err(
                PresentationFileError::Invalid("layout.path must not be empty".to_string()),
            ),
            SerializedLayout::LiteralRoot { path } => Ok(Self::LiteralRoot(path)),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializedFileRule {
    #[serde(default)]
    directory: Option<String>,
    select: SerializedSelect,
    naming: SerializedNaming,
    artifact: SerializedArtifact,
}

impl SerializedFileRule {
    fn into_rule(self) -> Result<FileRuleSpec, String> {
        let directory = self
            .directory
            .map(|path| parse_virtual_directory(&path))
            .transpose()?
            .unwrap_or_default();

        Ok(FileRuleSpec::new(
            self.select.into_select(),
            self.naming.into_naming()?,
            self.artifact.into_artifact()?,
        )
        .in_directory(directory))
    }
}

fn parse_virtual_directory(path: &str) -> Result<Vec<String>, String> {
    if path.trim().is_empty() {
        return Err("directory must not be empty".to_string());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("directory must be a relative virtual path".to_string());
    }

    let normalized = path.replace('\\', "/");
    let segments = normalized
        .split('/')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();

    if segments
        .iter()
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(
            "directory must not contain empty, current, or parent path segments".to_string(),
        );
    }
    if segments
        .first()
        .is_some_and(|segment| segment.ends_with(':'))
    {
        return Err("directory must not contain a drive prefix".to_string());
    }

    Ok(segments)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SerializedSelect {
    Games,
    GamesWithoutParts,
    SingleDiscGames,
    SingleDiscGamesByPlatform {
        platform: SerializedPlatform,
    },
    SingleDiscGamesByPlatformAndMedia {
        platform: SerializedPlatform,
        media: SerializedDiscMedia,
    },
    MultiDiscGames,
    MultiDiscGamesByPlatform {
        platform: SerializedPlatform,
    },
    SingleRomGames,
    Bytes,
    Text,
}

impl SerializedSelect {
    fn into_select(self) -> SelectSpec {
        match self {
            Self::Games => SelectSpec::Games,
            Self::GamesWithoutParts => SelectSpec::GamesWithoutParts,
            Self::SingleDiscGames => SelectSpec::SingleDiscGames,
            Self::SingleDiscGamesByPlatform { platform } => SelectSpec::SingleDiscGamesByPlatform {
                platform: platform.into(),
            },
            Self::SingleDiscGamesByPlatformAndMedia { platform, media } => {
                SelectSpec::SingleDiscGamesByPlatformAndMedia {
                    platform: platform.into(),
                    media: media.into(),
                }
            }
            Self::MultiDiscGames => SelectSpec::MultiDiscGames,
            Self::MultiDiscGamesByPlatform { platform } => SelectSpec::MultiDiscGamesByPlatform {
                platform: platform.into(),
            },
            Self::SingleRomGames => SelectSpec::SingleRomGames,
            Self::Bytes => SelectSpec::Bytes,
            Self::Text => SelectSpec::Text,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SerializedNaming {
    GameTitle,
    GameName,
    PartName,
    PlaylistName,
    SourceName,
    Literal { value: String },
}

impl SerializedNaming {
    fn into_naming(self) -> Result<NamingSpec, String> {
        match self {
            Self::GameTitle => Ok(NamingSpec::GameTitle),
            Self::GameName => Ok(NamingSpec::GameName),
            Self::PartName => Ok(NamingSpec::PartName),
            Self::PlaylistName => Ok(NamingSpec::PlaylistName),
            Self::SourceName => Ok(NamingSpec::SourceName),
            Self::Literal { value } if value.trim().is_empty() => {
                Err("naming.value must not be empty".to_string())
            }
            Self::Literal { value } => Ok(NamingSpec::Literal(value)),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializedArtifact {
    content_type: SerializedContentType,
    #[serde(default)]
    format: Option<SerializedFormat>,
    #[serde(default)]
    required_features: BTreeSet<SerializedCapabilityFeature>,
    #[serde(default)]
    preferred_features: BTreeSet<SerializedCapabilityFeature>,
    #[serde(default)]
    forbidden_features: BTreeSet<SerializedCapabilityFeature>,
}

impl SerializedArtifact {
    fn into_artifact(self) -> Result<ArtifactSpec, String> {
        let required_features = convert_features(self.required_features);
        let preferred_features = convert_features(self.preferred_features);
        let forbidden_features = convert_features(self.forbidden_features);

        if let Some(feature) = required_features.intersection(&forbidden_features).next() {
            return Err(format!(
                "feature {feature:?} cannot be both required and forbidden"
            ));
        }
        if let Some(feature) = preferred_features.intersection(&forbidden_features).next() {
            return Err(format!(
                "feature {feature:?} cannot be both preferred and forbidden"
            ));
        }

        Ok(ArtifactSpec {
            content_type: self.content_type.into(),
            format: self.format.map(Into::into),
            required_features,
            preferred_features,
            forbidden_features,
        })
    }
}

fn convert_features(
    features: BTreeSet<SerializedCapabilityFeature>,
) -> BTreeSet<CapabilityFeature> {
    features.into_iter().map(Into::into).collect()
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum SerializedCapabilityFeature {
    MultiSource,
    Streaming,
    Lossless,
    RandomAccess,
    SupportsPartial,
    MultiFile,
}

impl From<SerializedCapabilityFeature> for CapabilityFeature {
    fn from(feature: SerializedCapabilityFeature) -> Self {
        match feature {
            SerializedCapabilityFeature::MultiSource => Self::MultiSource,
            SerializedCapabilityFeature::Streaming => Self::Streaming,
            SerializedCapabilityFeature::Lossless => Self::Lossless,
            SerializedCapabilityFeature::RandomAccess => Self::RandomAccess,
            SerializedCapabilityFeature::SupportsPartial => Self::SupportsPartial,
            SerializedCapabilityFeature::MultiFile => Self::MultiFile,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SerializedContentType {
    Rom,
    Disc,
    Playlist,
    Archive,
    Directory,
    Bytes,
    Text,
    Game,
}

impl From<SerializedContentType> for ContentType {
    fn from(content_type: SerializedContentType) -> Self {
        match content_type {
            SerializedContentType::Rom => Self::Rom,
            SerializedContentType::Disc => Self::Disc,
            SerializedContentType::Playlist => Self::Playlist,
            SerializedContentType::Archive => Self::Archive,
            SerializedContentType::Directory => Self::Directory,
            SerializedContentType::Bytes => Self::Bytes,
            SerializedContentType::Text => Self::Text,
            SerializedContentType::Game => Self::Game,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SerializedFormat {
    Iso,
    Chd,
    Zip,
    M3u,
    Directory,
    Bin,
    CueBin,
    Text,
}

impl From<SerializedFormat> for Format {
    fn from(format: SerializedFormat) -> Self {
        match format {
            SerializedFormat::Iso => Self::Iso,
            SerializedFormat::Chd => Self::Chd,
            SerializedFormat::Zip => Self::Zip,
            SerializedFormat::M3u => Self::M3u,
            SerializedFormat::Directory => Self::Directory,
            SerializedFormat::Bin => Self::Bin,
            SerializedFormat::CueBin => Self::CueBin,
            SerializedFormat::Text => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SerializedPlatform {
    Snes,
    Ps1,
    Ps2,
    Nes,
    Megadrive,
    Unknown,
}

impl From<SerializedPlatform> for Platform {
    fn from(platform: SerializedPlatform) -> Self {
        match platform {
            SerializedPlatform::Snes => Self::Snes,
            SerializedPlatform::Ps1 => Self::Ps1,
            SerializedPlatform::Ps2 => Self::Ps2,
            SerializedPlatform::Nes => Self::Nes,
            SerializedPlatform::Megadrive => Self::Megadrive,
            SerializedPlatform::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SerializedDiscMedia {
    Cd,
    Dvd,
}

impl From<SerializedDiscMedia> for DiscMedia {
    fn from(media: SerializedDiscMedia) -> Self {
        match media {
            SerializedDiscMedia::Cd => Self::Cd,
            SerializedDiscMedia::Dvd => Self::Dvd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
version: 1
name: opl
layout:
  type: literal_root
  path: DVD
files:
  - directory: DVD
    select:
      type: single_disc_games_by_platform_and_media
      platform: ps2
      media: dvd
    naming:
      type: game_title
    artifact:
      content_type: disc
      format: iso
      required_features:
        - random_access
        - lossless
"#;

    #[test]
    fn parses_a_versioned_opl_presentation() {
        let document = parse_presentation_yaml(VALID).unwrap();

        assert_eq!(document.name, "opl");
        assert_eq!(
            document.spec.layout,
            LayoutSpec::LiteralRoot("DVD".to_string())
        );
        assert_eq!(document.spec.files.len(), 1);
        assert_eq!(document.spec.files[0].directory, ["DVD"]);
        assert_eq!(
            document.spec.files[0].select,
            SelectSpec::SingleDiscGamesByPlatformAndMedia {
                platform: Platform::Ps2,
                media: DiscMedia::Dvd,
            }
        );
    }

    #[test]
    fn rejects_unknown_schema_versions_and_fields() {
        let version_error =
            parse_presentation_yaml(&VALID.replace("version: 1", "version: 2")).unwrap_err();
        assert!(version_error
            .to_string()
            .contains("unsupported schema version 2"));

        let field_error =
            parse_presentation_yaml(&VALID.replace("name: opl", "name: opl\nsurprise: true"))
                .unwrap_err();
        assert!(field_error.to_string().contains("unknown field `surprise`"));
    }

    #[test]
    fn rejects_empty_rules_and_conflicting_features() {
        let empty = r#"
version: 1
name: empty
layout:
  type: flat
files: []
"#;
        assert!(parse_presentation_yaml(empty)
            .unwrap_err()
            .to_string()
            .contains("at least one rule"));

        let conflicting = VALID.replace(
            "        - lossless",
            "        - lossless\n      forbidden_features:\n        - lossless",
        );
        assert!(parse_presentation_yaml(&conflicting)
            .unwrap_err()
            .to_string()
            .contains("both required and forbidden"));
    }

    #[test]
    fn rejects_unsafe_rule_destination_directories() {
        for directory in ["", "/DVD", "../DVD", "games//DVD", "C:\\DVD"] {
            let yaml = VALID.replace("directory: DVD", &format!("directory: {directory:?}"));
            let error = parse_presentation_yaml(&yaml).unwrap_err().to_string();

            assert!(
                error.contains("directory"),
                "expected directory error for {directory:?}, got {error}"
            );
        }
    }
}
