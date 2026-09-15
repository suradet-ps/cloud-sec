//! Thai text handling: digit conversion, mark-insensitive matching, and
//! query-side detection of clause numbers, parties, and impact levels.
//!
//! The PDF font (THSarabunPSK) maps some combining marks into the private
//! use area (for example U+F70A..U+F71A), so any literal Thai comparison
//! must ignore marks on both sides. This is matching-only normalization:
//! stored text is never rewritten.

use regex::Regex;

use crate::model::{ImpactLevel, Party};

/// Convert Thai digits (U+0E50..U+0E59) to ASCII digits.
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

/// Strip Thai combining marks and private-use characters for matching.
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

/// The most specific clause number in a question, Thai or Arabic digits,
/// for example "5.2.3.4" or "๕.๒.๓.๔".
pub fn query_clause(question: &str) -> Option<String> {
    let normalized = thai_digits_to_ascii(question);
    let re = Regex::new(r"(\d+(?:\.\d+){1,3})").expect("static regex");
    re.captures_iter(&normalized)
        .map(|caps| caps[1].to_string())
        .max_by_key(String::len)
}

/// Which party a question asks about.
pub fn query_party(question: &str) -> Option<Party> {
    let head: String = question.chars().take(120).collect();
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

/// An impact level named in the question.
pub fn query_level(question: &str) -> Option<ImpactLevel> {
    let q = normalize_for_match(question);
    let low = q.contains(&normalize_for_match("ระดับต่ำ"));
    let medium = q.contains(&normalize_for_match("ระดับกลาง"));
    let high = q.contains(&normalize_for_match("ระดับสูง"));
    match (low, medium, high) {
        (true, false, false) => Some(ImpactLevel::Low),
        (false, true, false) => Some(ImpactLevel::Medium),
        (false, false, true) => Some(ImpactLevel::High),
        _ => None,
    }
}

/// The impact level a chunk's text describes, if any.
pub fn chunk_level(text: &str) -> Option<ImpactLevel> {
    let t = normalize_for_match(text);
    if t.contains(&normalize_for_match("ผลกระทบระดับต่ำ")) {
        Some(ImpactLevel::Low)
    } else if t.contains(&normalize_for_match("ผลกระทบระดับกลาง")) {
        Some(ImpactLevel::Medium)
    } else if t.contains(&normalize_for_match("ผลกระทบระดับสูง")) {
        Some(ImpactLevel::High)
    } else {
        None
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
    fn query_clause_reads_thai_digits() {
        assert_eq!(
            query_clause("ข้อ ๕.๒.๓.๔ กำหนดให้ผู้ใช้บริการคลาวด์ต้องทำอะไร"),
            Some("5.2.3.4".to_string())
        );
        assert_eq!(query_clause("ระดับกลางต้องทำอะไร"), None);
    }

    #[test]
    fn query_party_detects_named_side() {
        assert_eq!(
            query_party("ข้อ 5.2.3.4 กำหนดให้ผู้ใช้บริการคลาวด์ต้องทำอะไร"),
            Some(Party::Csc)
        );
        assert_eq!(query_party("ผู้ให้บริการคลาวด์ต้องแจ้งอะไร"), Some(Party::Csp));
        assert_eq!(query_party("ระดับกลางต้องทำอะไร"), None);
    }

    #[test]
    fn level_detection_works_both_directions() {
        assert_eq!(
            query_level("ผลกระทบระดับกลางต้องทำอะไร"),
            Some(ImpactLevel::Medium)
        );
        assert_eq!(query_level("ระบบระดับต่ำต้องทำอะไร"), Some(ImpactLevel::Low));
        assert_eq!(query_level("ต้องทำอะไรบ้าง"), None);
        assert_eq!(
            chunk_level("- ผลกระทบระดับสูง : ได้รับการรับรอง"),
            Some(ImpactLevel::High)
        );
        assert_eq!(chunk_level("ก) ต้องใช้ศูนย์ข้อมูลหลักในประเทศไทย"), None);
    }
}
