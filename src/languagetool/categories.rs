use serde::{Deserialize, Serialize};

/// Copied from
/// <https://languagetool.org/development/api/org/languagetool/rules/Categories.html/>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum Categories {
    Casing,
    Colloquialisms,
    Compounding,
    ConfusedWords,
    FalseFriends,
    GenderNeutrality,
    Grammar,
    Misc,
    PlainEnglish,
    Punctuation,
    Redundancy,
    Regionalisms,
    Repetitions,
    RepetitionsStyle,
    Semantics,
    Style,
    Typography,
    Typos,
    Wikipedia,
    AmericanEnglishStyle,
    NonstandardPhrases,
    Collocations,
}

impl core::fmt::Display for Categories {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let json_str = serde_json::to_string(self).map_err(|_ser_error| core::fmt::Error)?;
        let screaming_case_str: String =
            serde_json::from_str(&json_str).map_err(|_deser_error| core::fmt::Error)?;

        write!(f, "{screaming_case_str}")
    }
}

impl core::str::FromStr for Categories {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{value}\""))
    }
}
