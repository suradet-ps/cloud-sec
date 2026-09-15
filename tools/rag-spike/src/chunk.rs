//! Clause-aware chunking of the standard's extracted pages.
//!
//! Chunk identity is (clause, party). A clause is a dotted number such as
//! `5.2.3.4`; a party is `csc`, `csp`, or `both` when a segment mixes
//! intro text or could not be attributed.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Segments longer than this are split at line boundaries.
pub const MAX_CHUNK_CHARS: usize = 1000;
/// Segments shorter than this are dropped.
const MIN_CHUNK_CHARS: usize = 40;

/// One PDF page with its raw extracted lines and quality ratio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub no: u32,
    pub thai_ratio: f32,
    pub lines: Vec<String>,
}

/// One retrievable unit: a clause segment for one party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub clause: String,
    pub heading: String,
    pub party: String,
    pub page: u32,
    pub text: String,
    pub embed_text: String,
    #[serde(default)]
    pub vector: Vec<f32>,
}

/// Parse the page-tagged extraction produced by scripts/extract_pdf.py.
///
/// # Errors
///
/// Returns an error when the marker line is malformed.
pub fn parse_pages(raw: &str) -> anyhow::Result<Vec<Page>> {
    let marker = Regex::new(r"^===== PAGE (\d+) \| thai=([0-9.]+) =====$")?;
    let mut pages = Vec::new();
    let mut current: Option<Page> = None;
    for line in raw.lines() {
        if let Some(caps) = marker.captures(line) {
            if let Some(page) = current.take() {
                pages.push(page);
            }
            current = Some(Page {
                no: caps[1].parse()?,
                thai_ratio: caps[2].parse()?,
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

pub fn thai_digits_to_ascii(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{0e50}'..='\u{0e59}' => {
                char::from_u32('0' as u32 + (c as u32 - '\u{0e50}' as u32)).unwrap_or(c)
            }
            _ => c,
        })
        .collect()
}

fn is_noise(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if Regex::new(r"^[-–]\s*[0-9๐-๙]+\s*[-–]$")
        .expect("static regex")
        .is_match(trimmed)
    {
        return true;
    }
    if trimmed.contains("ราชกิจจานุเบกษา") || trimmed.starts_with("หน้า ") && trimmed.contains("เล่ม")
    {
        return true;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    false
}

/// Strip Thai combining marks and private-use characters for matching.
///
/// The embedded font (THSarabunPSK) maps some combining marks into the PUA
/// block (for example U+F70A..U+F71A), so literal Thai comparison must ignore
/// marks on both sides. This is a matching-only normalization: the extracted
/// text itself is stored unchanged.
pub fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| {
            let v = *c as u32;
            let is_mark = matches!(v, 0x0e31 | 0x0e34..=0x0e3a | 0x0e47..=0x0e4e);
            let is_pua = (0xe000..=0xf8ff).contains(&v);
            !is_mark && !is_pua
        })
        .collect()
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

/// True when a bullet is the first item of a Thai ordinal list ("ก)").
fn starts_with_ko(line: &str) -> bool {
    let mut chars = line.trim().chars();
    chars.next() == Some('\u{0e01}') && chars.next() == Some(')')
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
fn case_header_party(line: &str) -> Option<&'static str> {
    let normalized = normalize_for_match(line);
    if !normalized.contains(&normalize_for_match("กรณีของ")) {
        return None;
    }
    bullet_party(line)
}

fn is_bullet(line: &str) -> bool {
    let mut chars = line.trim().chars();
    match (chars.next(), chars.next()) {
        (Some(c), Some(')')) => ('\u{0e01}'..='\u{0e2e}').contains(&c),
        _ => false,
    }
}

fn bullet_party(line: &str) -> Option<&'static str> {
    let head: String = line.trim().chars().take(60).collect();
    let head = normalize_for_match(&head);
    let csc = head.find(&normalize_for_match("ผู้ใช้บริการคลาวด์"));
    let csp = head.find(&normalize_for_match("ผู้ให้บริการคลาวด์"));
    match (csc, csp) {
        (Some(a), Some(b)) => Some(if a < b { "csc" } else { "csp" }),
        (Some(_), None) => Some("csc"),
        (None, Some(_)) => Some("csp"),
        (None, None) => None,
    }
}

fn party_label(party: &str) -> &'static str {
    match party {
        "csc" => "ผู้ใช้บริการคลาวด์ (CSC)",
        "csp" => "ผู้ให้บริการคลาวด์ (CSP)",
        _ => "ทั้งสองฝ่าย",
    }
}

struct Builder {
    chunks: Vec<Chunk>,
    clause: String,
    heading: String,
    section2: String,
    section3: String,
    party: &'static str,
    page: u32,
    lines: Vec<String>,
    header_seen: bool,
    bullets_in_segment: usize,
    id_counters: std::collections::HashMap<String, u32>,
}

impl Builder {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            clause: String::new(),
            heading: String::new(),
            section2: String::new(),
            section3: String::new(),
            party: "both",
            page: 0,
            lines: Vec::new(),
            header_seen: false,
            bullets_in_segment: 0,
            id_counters: std::collections::HashMap::new(),
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
        let base = format!("{}#{}", self.clause, self.party);
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
                party = party_label(self.party),
                breadcrumb = breadcrumb,
                body = part,
            );
            self.chunks.push(Chunk {
                id: format!("{}#{}{}", self.clause, self.party, suffix),
                clause: self.clause.clone(),
                heading: self.heading.clone(),
                party: self.party.to_string(),
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
                        _ => {
                            b.clause = number.clone();
                        }
                    }
                    b.heading = heading;
                    b.party = "both";
                    b.header_seen = false;
                    b.page = page.no;
                    continue;
                }
            }
            if is_column_header(line) {
                if !b.lines.is_empty() {
                    b.flush();
                }
                b.party = "csc";
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
                    b.party = "csp";
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

/// Detect the most specific clause number in a question, in Thai or Arabic
/// digits, for example "5.2.3.4" or "๕.๒.๓.๔".
pub fn query_clause(question: &str) -> Option<String> {
    let normalized = thai_digits_to_ascii(question);
    let re = Regex::new(r"(\d+(?:\.\d+){1,3})").expect("static regex");
    re.captures_iter(&normalized)
        .map(|caps| caps[1].to_string())
        .max_by_key(String::len)
}

/// Detect an impact level named in the question.
pub fn query_level(question: &str) -> Option<&'static str> {
    let q = normalize_for_match(question);
    let low = q.contains(&normalize_for_match("ระดับต่ำ"));
    let medium = q.contains(&normalize_for_match("ระดับกลาง"));
    let high = q.contains(&normalize_for_match("ระดับสูง"));
    match (low, medium, high) {
        (true, false, false) => Some("low"),
        (false, true, false) => Some("medium"),
        (false, false, true) => Some("high"),
        _ => None,
    }
}

/// Detect the impact level a chunk describes, if any.
pub fn chunk_level(text: &str) -> Option<&'static str> {
    let t = normalize_for_match(text);
    if t.contains(&normalize_for_match("ผลกระทบระดับต่ำ")) {
        Some("low")
    } else if t.contains(&normalize_for_match("ผลกระทบระดับกลาง")) {
        Some("medium")
    } else if t.contains(&normalize_for_match("ผลกระทบระดับสูง")) {
        Some("high")
    } else {
        None
    }
}

/// Detect which party a question asks about, using the same mark-insensitive
/// matching as the chunker.
pub fn query_party(question: &str) -> Option<&'static str> {
    let head: String = question.chars().take(120).collect();
    let head = normalize_for_match(&head);
    let csc = head.find(&normalize_for_match("ผู้ใช้บริการคลาวด์"));
    let csp = head.find(&normalize_for_match("ผู้ให้บริการคลาวด์"));
    match (csc, csp) {
        (Some(a), Some(b)) => Some(if a < b { "csc" } else { "csp" }),
        (Some(_), None) => Some("csc"),
        (None, Some(_)) => Some("csp"),
        (None, None) => None,
    }
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

    #[test]
    fn normalize_ignores_marks_and_pua() {
        assert_eq!(
            normalize_for_match(&dirty("ผู้ให้บริการคลาวด์")),
            normalize_for_match("ผู้ให้บริการคลาวด์")
        );
    }

    #[test]
    fn header_detection_survives_pua_marks() {
        assert!(is_column_header(&dirty(
            "ผู้ใช้บริการคลาวด์  ผู้ให้บริการคลาวด์"
        )));
        assert!(!is_column_header("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"));
    }

    #[test]
    fn query_clause_reads_thai_digits() {
        assert_eq!(
            query_clause("ข้อ ๕.๒.๓.๔ กำหนดให้ผู้ใช้บริการคลาวด์ต้องทำอะไร"),
            Some("5.2.3.4".to_string())
        );
        assert_eq!(query_clause("ระดับกลางต้องทำอะไร"), None);
    }

    #[test]
    fn dates_are_not_clause_headings() {
        let filler = "ก) ผู้ให้บริการคลาวด์ต้องจัดให้มีขั้นตอนการเข้าสู่ระบบอย่างปลอดภัยสำหรับบัญชีใด ๆ ที่ร้องขอ";
        let page = Page {
            no: 4,
            thai_ratio: 0.8,
            lines: vec![
                "๒๒ ธันวาคม ๒๕๖๖".to_string(),
                "๕.๑.๑ นโยบายด้านความมั่นคงปลอดภัยสารสนเทศ (Information Security Policies)".to_string(),
                filler.to_string(),
            ],
        };
        let chunks = chunk_pages(&[page]);
        let clauses: Vec<&str> = chunks.iter().map(|c| c.clause.as_str()).collect();
        assert!(clauses.contains(&"5.1.1"), "clauses: {clauses:?}");
        assert!(!clauses.contains(&"22"), "clauses: {clauses:?}");
    }

    #[test]
    fn level_detection_works_both_directions() {
        assert_eq!(query_level("ผลกระทบระดับกลางต้องทำอะไร"), Some("medium"));
        assert_eq!(query_level("ระบบระดับต่ำต้องทำอะไร"), Some("low"));
        assert_eq!(query_level("ต้องทำอะไรบ้าง"), None);
        assert_eq!(chunk_level("- ผลกระทบระดับสูง : ได้รับการรับรอง"), Some("high"));
        assert_eq!(chunk_level("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"), None);
    }

    #[test]
    fn query_party_detects_named_side() {
        assert_eq!(query_party("ข้อ 5.2.3.4 กำหนดให้ผู้ใช้บริการคลาวด์ต้องทำอะไร"), Some("csc"));
        assert_eq!(query_party("ผู้ให้บริการคลาวด์ต้องแจ้งอะไร"), Some("csp"));
        assert_eq!(query_party("ระดับกลางต้องทำอะไร"), None);
    }

    #[test]
    fn level_rows_and_case_headers_are_recognized() {
        assert!(starts_level_row("- ผลกระทบระดับกลาง : ได้รับการรับรอง"));
        assert!(starts_level_row(&dirty("ผลกระทบระดับต่ำ ข้อกำหนด")));
        assert!(!starts_level_row("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"));
        assert_eq!(
            case_header_party(&dirty("- กรณีของผู้ให้บริการคลาวด์")),
            Some("csp")
        );
        assert_eq!(
            case_header_party(&dirty("- กรณีของผู้ใช้บริการคลาวด์")),
            Some("csc")
        );
        assert_eq!(case_header_party("- ผลกระทบระดับกลาง"), None);
    }

    #[test]
    fn bullet_party_detects_both_sides() {
        assert_eq!(
            bullet_party(&dirty("ก) ผู้ให้บริการคลาวด์ต้องให้ข้อมูล")),
            Some("csp")
        );
        assert_eq!(
            bullet_party(&dirty("ก) ผู้ใช้บริการคลาวด์ต้องร้องขอ")),
            Some("csc")
        );
        assert_eq!(bullet_party("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"), None);
    }
}
