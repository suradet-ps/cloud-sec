//! Clause-aware chunking of the standard's extracted pages.
//!
//! Chunk identity is (clause, party). A clause is a dotted number such as
//! `5.2.3.4`; a party is [`Party::Csc`], [`Party::Csp`], or [`Party::Both`]
//! when a segment mixes intro text or could not be attributed.

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{Chunk, Party};
use crate::text::{normalize_for_match, thai_digits_to_ascii};

/// Segments longer than this are split at line boundaries.
pub const MAX_CHUNK_CHARS: usize = 1000;
/// Segments shorter than this are dropped.
const MIN_CHUNK_CHARS: usize = 40;

/// One PDF page with its raw extracted lines and quality ratio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// PDF page number.
    pub no: u32,
    /// Fraction of Thai characters on the page (quality gate).
    pub thai_ratio: f32,
    /// Raw extracted lines.
    pub lines: Vec<String>,
}

/// Parse the page-tagged extraction produced by the ingest step.
///
/// # Errors
///
/// Returns an error when a page marker line is malformed.
pub fn parse_pages(raw: &str) -> Result<Vec<Page>, CoreError> {
    let marker = Regex::new(r"^===== PAGE (\d+) \| thai=([0-9.]+) =====$")
        .map_err(|e| CoreError::Regex(e.to_string()))?;
    let mut pages = Vec::new();
    let mut current: Option<Page> = None;
    for line in raw.lines() {
        if let Some(caps) = marker.captures(line) {
            if let Some(page) = current.take() {
                pages.push(page);
            }
            current = Some(Page {
                no: caps[1]
                    .parse()
                    .map_err(|e: std::num::ParseIntError| CoreError::Number(e.to_string()))?,
                thai_ratio: caps[2]
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| CoreError::Number(e.to_string()))?,
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(page) = current.as_mut() {
            page.lines.push(line.to_string());
        }
    }
    if let Some(page) = current {
        pages.push(page);
    }
    Ok(pages)
}

fn is_noise(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if Regex::new(r"^[-\u{2013}]\s*[0-9๐-๙]+\s*[-\u{2013}]$")
        .expect("static regex")
        .is_match(trimmed)
    {
        return true;
    }
    if trimmed.contains("ราชกิจจานุเบกษา") || trimmed.starts_with("หน้า ") && trimmed.contains("เล่ม")
    {
        return true;
    }
    trimmed.chars().all(|c| c.is_ascii_digit())
}

/// The two-column header line of a clause, in any spacing variant.
fn is_column_header(line: &str) -> bool {
    let collapsed: String = line.split_whitespace().collect();
    let expected = format!(
        "{}{}",
        normalize_for_match("ผู้ใช้บริการคลาวด์"),
        normalize_for_match("ผู้ให้บริการคลาวด์"),
    );
    normalize_for_match(&collapsed) == expected
}

fn is_bullet(line: &str) -> bool {
    let mut chars = line.trim().chars();
    match (chars.next(), chars.next()) {
        (Some(c), Some(')')) => ('\u{0e01}'..='\u{0e2e}').contains(&c),
        _ => false,
    }
}

/// True when a bullet is the first item of a Thai ordinal list ("ก)").
fn starts_with_ko(line: &str) -> bool {
    let mut chars = line.trim().chars();
    chars.next() == Some('\u{0e01}') && chars.next() == Some(')')
}

fn bullet_party(line: &str) -> Option<Party> {
    let head: String = line.trim().chars().take(60).collect();
    let head = normalize_for_match(&head);
    let csc = head.find(&normalize_for_match("ผู้ใช้บริการคลาวด์"));
    let csp = head.find(&normalize_for_match("ผู้ให้บริการคลาวด์"));
    match (csc, csp) {
        (Some(a), Some(b)) => Some(if a < b { Party::Csc } else { Party::Csp }),
        (Some(_), None) => Some(Party::Csc),
        (None, Some(_)) => Some(Party::Csp),
        (None, None) => None,
    }
}

/// The impact level table and the certification frequency list both open a
/// new row per level; each row must stay whole inside one chunk.
fn starts_level_row(line: &str) -> bool {
    let trimmed = line
        .trim()
        .trim_start_matches(['-', '\u{2013}', '\u{2022}'])
        .trim_start();
    normalize_for_match(trimmed).starts_with(&normalize_for_match("ผลกระทบระดับ"))
}

/// A "กรณีของ..." case line names the party the rows below it belong to.
fn case_header_party(line: &str) -> Option<Party> {
    let normalized = normalize_for_match(line);
    if !normalized.contains(&normalize_for_match("กรณีของ")) {
        return None;
    }
    bullet_party(line)
}

struct Builder {
    chunks: Vec<Chunk>,
    clause: String,
    heading: String,
    section2: String,
    section3: String,
    party: Party,
    page: u32,
    lines: Vec<String>,
    header_seen: bool,
    bullets_in_segment: usize,
    id_counters: HashMap<String, u32>,
}

impl Builder {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            clause: String::new(),
            heading: String::new(),
            section2: String::new(),
            section3: String::new(),
            party: Party::Both,
            page: 0,
            lines: Vec::new(),
            header_seen: false,
            bullets_in_segment: 0,
            id_counters: HashMap::new(),
        }
    }

    fn flush(&mut self) {
        self.bullets_in_segment = 0;
        let text = self.lines.join("\n").trim().to_string();
        let text = Regex::new(r"\n{3,}")
            .expect("static regex")
            .replace_all(&text, "\n\n")
            .to_string();
        if text.chars().count() < MIN_CHUNK_CHARS || self.clause.is_empty() {
            self.lines.clear();
            return;
        }
        let mut parts: Vec<String> = Vec::new();
        if text.chars().count() <= MAX_CHUNK_CHARS {
            parts.push(text);
        } else {
            let mut current = String::new();
            for line in text.lines() {
                if current.chars().count() + line.chars().count() + 1 > MAX_CHUNK_CHARS
                    && !current.is_empty()
                {
                    parts.push(current.trim().to_string());
                    current = String::new();
                }
                current.push_str(line);
                current.push('\n');
            }
            if !current.trim().is_empty() {
                parts.push(current.trim().to_string());
            }
        }
        let base = format!("{}#{}", self.clause, self.party.as_str());
        let sequence = self.id_counters.entry(base).or_insert(0);
        *sequence += 1;
        let sequence_suffix = if *sequence > 1 {
            format!("-{sequence}")
        } else {
            String::new()
        };
        for (i, part) in parts.iter().enumerate() {
            let suffix = if parts.len() > 1 {
                format!("{sequence_suffix}-p{}", i + 1)
            } else {
                sequence_suffix.clone()
            };
            let breadcrumb = match (self.section2.is_empty(), self.section3.is_empty()) {
                (true, _) => String::new(),
                (false, true) => format!("{}\n", self.section2),
                (false, false) => format!("{} > {}\n", self.section2, self.section3),
            };
            let embed_text = format!(
                "มาตรฐานความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567\nข้อ {clause}: {heading}\nฝ่าย: {party}\n{breadcrumb}{body}",
                clause = self.clause,
                heading = self.heading,
                party = self.party.label_th(),
                breadcrumb = breadcrumb,
                body = part,
            );
            self.chunks.push(Chunk {
                id: format!("{}#{}{}", self.clause, self.party.as_str(), suffix),
                clause: self.clause.clone(),
                heading: self.heading.clone(),
                party: self.party,
                page: self.page,
                text: part.clone(),
                embed_text,
                vector: Vec::new(),
            });
        }
        self.lines.clear();
    }
}

/// Turn page lines into clause/party chunks.
pub fn chunk_pages(pages: &[Page]) -> Vec<Chunk> {
    // Dotted clause numbers ("5.2.3.4") and top-level sections ("4. ...").
    // A bare number like a date ("22 ธันวาคม") must not match.
    let heading_dotted = Regex::new(r"^(\d+(?:\.\d+){1,3})\s+(.+)$").expect("static regex");
    let heading_section = Regex::new(r"^(\d+)\.\s+(.+)$").expect("static regex");
    let mut b = Builder::new();
    for page in pages {
        for raw in &page.lines {
            if is_noise(raw) {
                continue;
            }
            let line = raw.trim();
            if line.is_empty() {
                b.lines.push(String::new());
                continue;
            }
            let normalized = thai_digits_to_ascii(line);
            let heading_caps = heading_dotted
                .captures(&normalized)
                .or_else(|| heading_section.captures(&normalized));
            if let Some(caps) = heading_caps {
                let number = caps[1].to_string();
                let title = caps[2].trim().to_string();
                let depth = number.matches('.').count();
                if title.chars().count() >= 4 && depth <= 3 {
                    b.flush();
                    let heading = format!("{} {}", number, title);
                    match depth {
                        0 => b.clause = number.clone(),
                        1 => {
                            b.clause = number.clone();
                            b.section2 = heading.clone();
                            b.section3.clear();
                        }
                        2 => {
                            b.clause = number.clone();
                            b.section3 = heading.clone();
                        }
                        _ => b.clause = number.clone(),
                    }
                    b.heading = heading;
                    b.party = Party::Both;
                    b.header_seen = false;
                    b.page = page.no;
                    continue;
                }
            }
            if is_column_header(line) {
                if !b.lines.is_empty() {
                    b.flush();
                }
                b.party = Party::Csc;
                b.header_seen = true;
                b.bullets_in_segment = 0;
                b.page = page.no;
                continue;
            }
            if starts_level_row(line) && !b.lines.is_empty() {
                b.flush();
                b.page = page.no;
            }
            if let Some(party) = case_header_party(line) {
                if !b.lines.is_empty() && party != b.party {
                    b.flush();
                }
                b.party = party;
                b.page = page.no;
            }
            if is_bullet(line) {
                let party_kw = bullet_party(line);
                let restart = starts_with_ko(line)
                    && b.header_seen
                    && b.bullets_in_segment > 0
                    && party_kw.is_none();
                if restart {
                    b.flush();
                    b.party = Party::Csp;
                    b.page = page.no;
                } else if let Some(party) = party_kw {
                    if !b.lines.is_empty() && party != b.party {
                        b.flush();
                    }
                    b.party = party;
                    b.page = page.no;
                } else if b.lines.is_empty() {
                    b.page = page.no;
                }
                b.bullets_in_segment += 1;
            } else if b.lines.is_empty() {
                b.page = page.no;
            }
            b.lines.push(line.to_string());
        }
        b.lines.push(String::new());
    }
    b.flush();
    b.chunks
}

/// Hand-transcribed rows of the annex section 4 table (PDF pages 8 and 9).
///
/// The table extracts as column fragments that the model cannot attribute to
/// a level or a party, so the rows are stored explicitly. Every value is
/// transcribed from the source table; re-verify against the PDF before any
/// change.
pub fn curated_level_chunks() -> Vec<Chunk> {
    let rows = [
        (
            "low",
            "ผลกระทบระดับต่ำ: ข้อกำหนดขั้นต่ำ ส่วนที่ 1 (การกำกับดูแล) เฉพาะข้อ 5.1.1, 5.1.2; \
ส่วนที่ 2 (การปฏิบัติการ) เฉพาะข้อ 5.2.1, 5.2.2, 5.2.3, 5.2.4, 5.2.8, 5.2.9. \
ผู้ใช้บริการคลาวด์ (CSC): ประเมินตนเอง (Self-assessment) พร้อมแนบหลักฐานและขออนุมัติไปยังผู้บริหารสูงสุดของหน่วยงาน โดยเก็บรักษาไว้ที่หน่วยงานและส่งให้สำนักงาน และทบทวนอย่างน้อยปีละ 1 ครั้ง. \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3) และได้รับการรับรองตามมาตรฐาน ISO/IEC 27001 Certification และ CSA STAR Level 1/CCM Lite เป็นอย่างน้อย",
        ),
        (
            "medium",
            "ผลกระทบระดับกลาง: ข้อกำหนดขั้นต่ำ ส่วนที่ 1 (การกำกับดูแล) ทุกข้อ; \
ส่วนที่ 2 (การปฏิบัติการ) เฉพาะข้อ 5.2.1, 5.2.2, 5.2.3, 5.2.4, 5.2.7, 5.2.8, 5.2.9, 5.2.10. \
ผู้ใช้บริการคลาวด์ (CSC): ได้รับการรับรองโดยหน่วยงานควบคุมหรือกำกับดูแล (Attestation) หรือได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3). \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี และได้รับการรับรองตามมาตรฐาน CSA STAR Level 2/CCM และ ISO/IEC 27701 Certification เป็นอย่างน้อย",
        ),
        (
            "high",
            "ผลกระทบระดับสูง: ข้อกำหนดขั้นต่ำ ทุกข้อทั้งส่วนที่ 1 และส่วนที่ 2. \
ผู้ใช้บริการคลาวด์ (CSC): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3). \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี และได้รับการรับรองตามมาตรฐาน ISO/IEC 27017 Certification หรือ CSA STAR Level 2/CCM และ ISO/IEC 27018 Certification และ ISO/IEC 27701 Certification เป็นอย่างน้อย",
        ),
    ];
    rows.iter()
        .map(|(level, text)| Chunk {
            id: format!("4#curated-{level}"),
            clause: "4".to_string(),
            heading: "ข้อกำหนดขั้นต่ำและการตรวจรับรอง (ตาราง แปลงเป็นข้อความ)".to_string(),
            party: Party::Both,
            page: 8,
            text: (*text).to_string(),
            embed_text: format!(
                "มาตรฐานความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567\nข้อ 4: ข้อกำหนดขั้นต่ำและการตรวจรับรอง\n{text}"
            ),
            vector: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulate the font PUA substitution for the tone mark mai tho.
    fn dirty(s: &str) -> String {
        s.chars()
            .map(|c| if c == '\u{0e49}' { '\u{f70b}' } else { c })
            .collect()
    }

    fn filler() -> String {
        "ก) ผู้ให้บริการคลาวด์ต้องจัดให้มีขั้นตอนการเข้าสู่ระบบอย่างปลอดภัยสำหรับบัญชีใด ๆ ที่ร้องขอ".to_string()
    }

    #[test]
    fn header_detection_survives_pua_marks() {
        assert!(is_column_header(&dirty("ผู้ใช้บริการคลาวด์  ผู้ให้บริการคลาวด์")));
        assert!(!is_column_header("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"));
    }

    #[test]
    fn bullet_party_detects_both_sides() {
        assert_eq!(
            bullet_party(&dirty("ก) ผู้ให้บริการคลาวด์ต้องให้ข้อมูล")),
            Some(Party::Csp)
        );
        assert_eq!(
            bullet_party(&dirty("ก) ผู้ใช้บริการคลาวด์ต้องร้องขอ")),
            Some(Party::Csc)
        );
        assert_eq!(bullet_party("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"), None);
    }

    #[test]
    fn level_rows_and_case_headers_are_recognized() {
        assert!(starts_level_row("- ผลกระทบระดับกลาง : ได้รับการรับรอง"));
        assert!(starts_level_row(&dirty("ผลกระทบระดับต่ำ ข้อกำหนด")));
        assert!(!starts_level_row("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"));
        assert_eq!(
            case_header_party(&dirty("- กรณีของผู้ให้บริการคลาวด์")),
            Some(Party::Csp)
        );
        assert_eq!(
            case_header_party(&dirty("- กรณีของผู้ใช้บริการคลาวด์")),
            Some(Party::Csc)
        );
        assert_eq!(case_header_party("- ผลกระทบระดับกลาง"), None);
    }

    #[test]
    fn two_column_clause_splits_into_parties() {
        let page = Page {
            no: 17,
            thai_ratio: 0.8,
            lines: vec![
                "๕.๒.๕.๑ ตําแหน่งของศูนย์ข้อมูล (Data Center Location)".to_string(),
                "ผู้ใช้บริการคลาวด์  ผู้ให้บริการคลาวด์".to_string(),
                "ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย (Data Localization)".to_string(),
                "ก) ต้องจัดตั้งศูนย์ข้อมูลหลักในประเทศไทย (Data Localization)".to_string(),
                "ข) ต้องจัดตั้งศูนย์ข้อมูลสำรองในประเทศไทย หรืออยู่ในภูมิภาคเอเชียตะวันออกเฉียงใต้".to_string(),
            ],
        };
        let chunks = chunk_pages(&[page]);
        let parties: Vec<Party> = chunks.iter().map(|c| c.party).collect();
        assert!(parties.contains(&Party::Csc), "parties: {parties:?}");
        assert!(parties.contains(&Party::Csp), "parties: {parties:?}");
        let csp = chunks
            .iter()
            .find(|c| c.party == Party::Csp)
            .expect("csp chunk");
        assert!(csp.text.contains("ต้องจัดตั้งศูนย์ข้อมูลสำรอง"));
    }

    #[test]
    fn dates_are_not_clause_headings() {
        let page = Page {
            no: 4,
            thai_ratio: 0.8,
            lines: vec![
                "๒๒ ธันวาคม ๒๕๖๖".to_string(),
                "๕.๑.๑ นโยบายด้านความมั่นคงปลอดภัยสารสนเทศ (Information Security Policies)".to_string(),
                filler(),
            ],
        };
        let chunks = chunk_pages(&[page]);
        let clauses: Vec<&str> = chunks.iter().map(|c| c.clause.as_str()).collect();
        assert!(clauses.contains(&"5.1.1"), "clauses: {clauses:?}");
        assert!(!clauses.contains(&"22"), "clauses: {clauses:?}");
    }
}
