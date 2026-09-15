//! Question intent routing.
//!
//! The product promises grounded, cited answers from the document. That
//! promise only makes sense for document questions: a meta question such as
//! "ทำอะไรได้บ้าง" must never be forced through retrieval, where it would
//! pull a random clause and answer "ไม่พบในเอกสาร". Intent routing keeps
//! the three conversation modes apart, deterministically and testably.

use crate::text::normalize_for_match;

/// What kind of answer a question needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// A question about the standard: retrieval and citations apply.
    Document,
    /// A question about the assistant itself ("ทำอะไรได้บ้าง").
    Help,
    /// A greeting or thanks.
    Greeting,
}

/// Canned answer for help questions.
pub const HELP_ANSWER: &str = "\
สวัสดีครับ ผมเป็นผู้ช่วยค้นและวิเคราะห์มาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567 \
ทำงานในเครื่องนี้ทั้งหมด เอกสารไม่ถูกส่งออกไปไหน

ผมช่วยได้เช่น
- ค้นข้อกำหนดรายข้อ เช่น \"ข้อ 5.2.3.4 กำหนดอะไรกับผู้ใช้บริการคลาวด์\"
- แยกหน้าที่สองฝ่าย: ผู้ใช้บริการคลาวด์ (CSC) และผู้ให้บริการคลาวด์ (CSP)
- ดูตามระดับผลกระทบ: ระดับต่ำ กลาง สูง ต้องทำข้อใด และต้องได้รับการรับรองแบบใด
- รวบรวมรายการข้อกำหนดของระดับหรือฝ่ายที่ต้องการ เพื่อทำเช็กลิสต์

ทุกคำตอบอ้างอิงเลขข้อและหน้าจากเอกสารต้นฉบับ ถ้าคำถามไม่อยู่ในเอกสาร ผมจะบอกว่าไม่พบในเอกสารครับ";

/// Canned answer for greetings and thanks.
pub const GREETING_ANSWER: &str = "\
สวัสดีครับ มีอะไรให้ช่วยค้นมาตรฐานคลาวด์ไหมครับ \
ลองถามเช่น \"ผู้ให้บริการคลาวด์ต้องทำอะไรเรื่องข้อมูลสำรอง\" \
หรือ \"ระดับกลางต้องปฏิบัติตามข้อกำหนดใดบ้าง\" ก็ได้ครับ";

/// Classify a question into a conversation mode.
pub fn classify(question: &str) -> Intent {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return Intent::Document;
    }
    let lower = trimmed.to_lowercase();
    let normalized = normalize_for_match(&lower);

    // Patterns are written in natural Thai and normalized on both sides,
    // so mark handling lives in exactly one place.
    const HELP_PATTERNS: [&str; 8] = [
        "ทำอะไรได้",
        "ช่วยอะไรได้",
        "ช่วยอะไรบ้าง",
        "ใช้งานยังไง",
        "ใช้ยังไง",
        "วิธีใช้",
        "คุณเป็นใคร",
        "แนะนำตัว",
    ];
    if lower.contains("help")
        || HELP_PATTERNS
            .iter()
            .any(|pattern| normalized.contains(&normalize_for_match(pattern)))
    {
        return Intent::Help;
    }

    // Short greetings and thanks. ASCII greetings match only as a whole
    // word so they cannot fire inside another token.
    let ascii_simple = matches!(
        lower.as_str(),
        "hi" | "hello" | "hey" | "thanks" | "thank you" | "thankyou"
    );
    let short = trimmed.chars().count() <= 40;
    const GREETING_PATTERNS: [&str; 3] = ["สวัสดี", "หวัดดี", "ขอบคุณ"];
    if short
        && (ascii_simple
            || GREETING_PATTERNS
                .iter()
                .any(|pattern| normalized.contains(&normalize_for_match(pattern))))
    {
        return Intent::Greeting;
    }

    Intent::Document
}

/// The canned answer for a question that must never reach retrieval:
/// `Some(text)` for help and greeting intents, `None` for document
/// questions.
pub fn canned_answer(question: &str) -> Option<&'static str> {
    match classify(question) {
        Intent::Help => Some(HELP_ANSWER),
        Intent::Greeting => Some(GREETING_ANSWER),
        Intent::Document => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_questions_are_help() {
        assert_eq!(classify("ทำอะไรได้บ้าง"), Intent::Help);
        assert_eq!(classify("ช่วยอะไรได้บ้างครับ"), Intent::Help);
        assert_eq!(classify("คุณเป็นใคร"), Intent::Help);
        assert_eq!(classify("ใช้งานยังไง"), Intent::Help);
    }

    #[test]
    fn short_greetings_are_greetings() {
        assert_eq!(classify("สวัสดีครับ"), Intent::Greeting);
        assert_eq!(classify("หวัดดี"), Intent::Greeting);
        assert_eq!(classify("hello"), Intent::Greeting);
        assert_eq!(classify("ขอบคุณครับ"), Intent::Greeting);
    }

    #[test]
    fn canned_answers_bypass_retrieval() {
        assert!(canned_answer("ทำอะไรได้บ้าง").is_some());
        assert!(
            canned_answer("ทำอะไรได้บ้าง")
                .expect("help")
                .contains("อ้างอิงเลขข้อ")
        );
        assert!(canned_answer("สวัสดี").is_some());
        assert!(canned_answer("ข้อ 5.2.3.4 กำหนดอะไร").is_none());
    }

    #[test]
    fn document_questions_stay_document() {
        assert_eq!(classify("ข้อ 5.2.3.4 กำหนดอะไร"), Intent::Document);
        assert_eq!(classify("มีบทลงโทษไหม"), Intent::Document);
        assert_eq!(
            classify("ผู้ให้บริการคลาวด์ต้องทำอะไรเรื่องสำรองข้อมูล"),
            Intent::Document
        );
        assert_eq!(
            classify("สวัสดีครับ ข้อ 5.2.3.4 กำหนดอะไรกับผู้ใช้บริการคลาวด์ที่ต้องยืนยันตัวตนแบบหลายปัจจัย"),
            Intent::Document
        );
    }
}
