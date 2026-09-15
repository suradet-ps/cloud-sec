//! Domain types shared by every layer.

use serde::{Deserialize, Serialize};

/// The two responsible parties in the standard's two-column requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Party {
    /// Both columns: intro text or a segment that could not be attributed.
    #[default]
    Both,
    /// ผู้ใช้บริการคลาวด์ (Cloud Service Customer).
    Csc,
    /// ผู้ให้บริการคลาวด์ (Cloud Service Provider).
    Csp,
}

impl Party {
    /// Wire and display key, matching the serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            Party::Both => "both",
            Party::Csc => "csc",
            Party::Csp => "csp",
        }
    }

    /// Thai label for citations and the UI.
    pub fn label_th(self) -> &'static str {
        match self {
            Party::Both => "ทั้งสองฝ่าย",
            Party::Csc => "ผู้ใช้บริการคลาวด์ (CSC)",
            Party::Csp => "ผู้ให้บริการคลาวด์ (CSP)",
        }
    }
}

/// Impact levels from annex section 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    /// ผลกระทบระดับต่ำ.
    Low,
    /// ผลกระทบระดับกลาง.
    Medium,
    /// ผลกระทบระดับสูง.
    High,
}

impl ImpactLevel {
    /// Thai label for citations and the UI.
    pub fn label_th(self) -> &'static str {
        match self {
            ImpactLevel::Low => "ต่ำ",
            ImpactLevel::Medium => "กลาง",
            ImpactLevel::High => "สูง",
        }
    }
}

/// One retrievable unit: a clause segment for one party.
///
/// Requirement identity is always the pair (clause, party).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Deterministic id, for example `5.2.3.4#csp`.
    pub id: String,
    /// Canonical dotted clause number, for example `5.2.3.4`.
    pub clause: String,
    /// Clause heading, Thai with the English gloss when present.
    pub heading: String,
    /// Which column the requirement binds.
    pub party: Party,
    /// PDF page number (1 to 24).
    pub page: u32,
    /// Display text.
    pub text: String,
    /// Text used for embedding (breadcrumb + heading + party + body).
    pub embed_text: String,
    /// Embedding vector; empty until the index build fills it.
    #[serde(default)]
    pub vector: Vec<f32>,
}
