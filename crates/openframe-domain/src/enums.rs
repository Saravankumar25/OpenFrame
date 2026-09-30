//! Canonical enumerations shared across modules.
//!
//! Enums are persisted as their stable `as_str()` value (TEXT columns), never
//! as ordinals, so reordering variants can never corrupt stored data.

/// Declare a string-backed enum with serde/ts-rs support and stable text mapping.
///
/// The invoking crate must depend on `serde` and `ts-rs`.
#[macro_export]
macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[ts(export)]
        pub enum $name {
            $(
                #[serde(rename = $text)]
                $variant,
            )+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }

            pub fn parse(value: &str) -> Option<$name> {
                match value { $($text => Some($name::$variant),)+ _ => None }
            }

            /// Parse or fail with a validation error naming the field.
            pub fn parse_field(value: &str, field: &str) -> $crate::AppResult<$name> {
                Self::parse(value).ok_or_else(|| {
                    $crate::AppError::validation(field, format!("'{value}' is not a valid {field}."))
                })
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

string_enum!(
    /// FSD §4.1 project types.
    ProjectType {
        FeatureFilm = "Feature Film",
        ShortFilm = "Short Film",
        Episodic = "Episodic",
        Series = "Series",
    }
);

impl ProjectType {
    pub fn is_episodic(self) -> bool {
        matches!(self, ProjectType::Episodic | ProjectType::Series)
    }
}

string_enum!(
    /// FSD §4.3 manual project status labels. "Archived" is represented by the
    /// separate archive flag (FSD §4.5); see ADR-0012.
    ProjectStatus {
        Idea = "Idea",
        Development = "Development",
        Writing = "Writing",
        Rewrite = "Rewrite",
        ShootingDraft = "Shooting Draft",
        PreProduction = "Pre-Production",
        ShootPreparation = "Shoot Preparation",
        Shooting = "Shooting",
    }
);

string_enum!(
    /// Idea Vault item types (FSD §5.2, Domain §4).
    VaultItemType {
        Note = "note",
        Image = "image",
        Url = "url",
        Pdf = "pdf",
        Document = "document",
        Audio = "audio",
        Voice = "voice",
        Video = "video",
        Sketch = "sketch",
        Quote = "quote",
        Screenshot = "screenshot",
        File = "file",
    }
);

string_enum!(
    /// Screenplay element types (FSD §15.5, Domain §4 Screenplay Element).
    ElementType {
        SceneHeading = "scene_heading",
        Action = "action",
        Character = "character",
        Dialogue = "dialogue",
        Parenthetical = "parenthetical",
        Transition = "transition",
        Shot = "shot",
        Note = "note",
    }
);

string_enum!(
    /// Screenplay draft lifecycle (Domain §7).
    DraftStatus {
        Draft = "Draft",
        Review = "Review",
        Locked = "Locked",
        Revision = "Revision",
    }
);

string_enum!(
    /// Breakdown categories, exactly in FSD §26.3 order. Catalog views map these
    /// 1:1 onto the §28.2 catalog labels (ADR-0012).
    BreakdownCategory {
        Cast = "Cast",
        Extras = "Extras / Background",
        Location = "Location / Set",
        Props = "Props",
        Wardrobe = "Wardrobe",
        Vehicles = "Vehicles",
        HairMakeup = "Hair / Makeup",
        SpecialEffects = "Special Effects",
        Vfx = "VFX",
        Sound = "Sound",
        Animals = "Animals",
    }
);

string_enum!(
    /// Breakdown element confirmation (Domain §4 Breakdown Element).
    ConfirmationState {
        Suggested = "Suggested",
        Confirmed = "Confirmed",
        Rejected = "Rejected",
        Manual = "Manual",
    }
);

string_enum!(
    /// Change Set review state (Domain §4 Change Set).
    ChangeSetState {
        Pending = "Pending",
        Accepted = "Accepted",
        Rejected = "Rejected",
        Applied = "Applied",
        Stale = "Stale",
        Conflict = "Conflict",
        Failed = "Failed",
    }
);

string_enum!(
    /// Origin of a Change Set.
    ChangeSetOrigin {
        User = "User",
        Import = "Import",
        Review = "Review",
        Ai = "AI",
        Collaboration = "Collaboration",
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_round_trip_through_text() {
        for t in ProjectType::ALL {
            assert_eq!(ProjectType::parse(t.as_str()), Some(*t));
        }
        for c in BreakdownCategory::ALL {
            assert_eq!(BreakdownCategory::parse(c.as_str()), Some(*c));
        }
        assert_eq!(BreakdownCategory::ALL.len(), 11);
        assert_eq!(BreakdownCategory::ALL[0], BreakdownCategory::Cast);
    }

    #[test]
    fn serde_uses_labels() {
        let json = serde_json::to_string(&ProjectType::ShortFilm).unwrap();
        assert_eq!(json, "\"Short Film\"");
        assert!(ProjectStatus::parse_field("Nope", "status").is_err());
    }
}
