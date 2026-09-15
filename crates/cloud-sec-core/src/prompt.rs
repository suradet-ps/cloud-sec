//! Prompt assembly: the system rules and the retrieved-context user prompt.

use crate::model::Chunk;

/// System prompt for the chat model. Thai, per the document and the users.
///
/// The rules enforce the product promise: every requirement statement must
/// carry a clause number and a party, and the model must refuse rather than
/// improvise.
pub const SYSTEM_PROMPT: &str = "\
คุณเป็นผู้ช่วยวิเคราะห์มาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567
ตอบคำถามโดยใช้เฉพาะข้อมูลใน \"เอกสารอ้างอิง\" ที่ให้มาเท่านั้น
กฎ:
1. ทุกข้อความที่อ้างถึงข้อกำหนด ต้องระบุเลขข้อ (เช่น 5.2.3.4) และฝ่ายที่รับผิดชอบ (ผู้ใช้บริการคลาวด์ หรือ ผู้ให้บริการคลาวด์)
2. ถ้าเอกสารอ้างอิงไม่ครอบคลุมคำถาม ให้ตอบว่า \"ไม่พบในเอกสาร\" แล้วบอกว่าข้อใดใกล้เคียงที่สุด
3. ถ้าคำถามระบุระดับผลกระทบ (ต่ำ/กลาง/สูง) ให้ตอบเฉพาะข้อกำหนดของระดับนั้น ห้ามนำข้อกำหนดของระดับอื่นมาตอบ ถ้าข้อมูลที่ให้มาไม่พอ ให้บอกว่าไม่พบข้อมูลของระดับนั้น
4. ถ้าคำถามถามว่าต้องปฏิบัติตามข้อกำหนดใด ให้ระบุเลขข้อทั้งหมดที่ปรากฏในเอกสารอ้างอิงของระดับหรือฝ่ายนั้นให้ครบ
5. ตอบเป็นภาษาไทย กระชับ ตรงประเด็น สุภาพและเป็นธรรมชาติ ไม่แต่งเติมข้อกำหนดขึ้นเอง";

/// Assemble the user prompt from retrieved hits, capped at `max_chars` of
/// context. Returns the prompt and the number of chunks that fit.
pub fn build_user_prompt(
    question: &str,
    hits: &[(usize, f32)],
    chunks: &[Chunk],
    max_chars: usize,
) -> (String, usize) {
    let mut context = String::new();
    let mut used_chars = 0;
    let mut used = 0;
    for (rank, (idx, score)) in hits.iter().enumerate() {
        let chunk = &chunks[*idx];
        let block = format!(
            "[{}] ข้อ {} ({}) หน้า {} ฝ่าย {} (score {:.3})\n{}\n\n",
            rank + 1,
            chunk.clause,
            chunk.heading,
            chunk.page,
            chunk.party.label_th(),
            score,
            chunk.text,
        );
        let block_chars = block.chars().count();
        if used_chars + block_chars > max_chars && used > 0 {
            break;
        }
        context.push_str(&block);
        used_chars += block_chars;
        used += 1;
    }
    let prompt = format!("เอกสารอ้างอิง:\n\n{context}---\n\nคําถาม: {question}");
    (prompt, used)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Party;

    fn chunk(clause: &str, text: &str) -> Chunk {
        Chunk {
            id: clause.to_string(),
            clause: clause.to_string(),
            heading: "heading".to_string(),
            party: Party::Csc,
            page: 5,
            text: text.to_string(),
            embed_text: String::new(),
            vector: Vec::new(),
        }
    }

    #[test]
    fn budget_drops_chunks_instead_of_truncating() {
        let chunks = vec![chunk("1.1", &"ก".repeat(60)), chunk("1.2", &"ข".repeat(60))];
        let hits = vec![(0, 0.9), (1, 0.8)];
        let (prompt, used) = build_user_prompt("คําถาม", &hits, &chunks, 150);
        assert_eq!(used, 1);
        assert!(prompt.contains("ข้อ 1.1"));
        assert!(!prompt.contains("ข้อ 1.2"));
    }

    #[test]
    fn prompt_carries_party_and_citation_labels() {
        let chunks = vec![chunk("5.2.3.4", "ก) ผู้ใช้บริการคลาวด์ต้องใช้เทคนิคการยืนยันตัวตน")];
        let hits = vec![(0, 0.77)];
        let (prompt, used) = build_user_prompt("คําถาม", &hits, &chunks, 2800);
        assert_eq!(used, 1);
        assert!(prompt.contains("ข้อ 5.2.3.4"));
        assert!(prompt.contains("หน้า 5"));
        assert!(prompt.contains("ผู้ใช้บริการคลาวด์ (CSC)"));
    }
}
