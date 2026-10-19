//! Runtime authored rule metadata. Official observations remain printed-data authority.
use serde::{Deserialize, Serialize};
#[derive(Debug,Clone,PartialEq,Eq,Serialize,Deserialize,schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeckCopyRule {
    pub max_copies: u8,
    pub evidence_observation_id: String,
}
impl DeckCopyRule {
    pub fn validate(&self)->Result<(),String>{
        if !(1..=50).contains(&self.max_copies){return Err("max_copies must be between 1 and 50".into());}
        if self.evidence_observation_id.len()!=64 || !self.evidence_observation_id.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)){return Err("evidence_observation_id must be a lowercase 64-digit hex official observation ID".into());}
        Ok(())
    }
    /// Fail closed on a missing/mismatched authority record or unsupported wording.
    pub fn validate_observation(&self,card_id:&str,bytes:&[u8])->Result<(),String>{
        self.validate()?;
        let v:serde_json::Value=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
        if v["source"]!="official" || v["observation_id"]!=self.evidence_observation_id || v["extracted"]["card_id"]!=card_id{return Err("official observation ID/card/source mismatch".into());}
        let expected=format!("You can include up to {} copies of cards with this card's card number in your deck.",self.max_copies);
        let matches=v["extracted"]["text_sections"].as_array().into_iter().flatten().any(|section|{
            section["label"]=="Effect" && section["text"].as_str().is_some_and(|text|text.lines().any(|line|line.trim()==expected))
        });
        if !matches{return Err("official Effect text does not contain the authored printed deck-copy allowance".into());}
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rule()->DeckCopyRule { DeckCopyRule { max_copies:50, evidence_observation_id:"a".repeat(64) } }
    fn observation()->serde_json::Value {serde_json::json!({"source":"official","observation_id":"a".repeat(64),"extracted":{"card_id":"PROBE-DECK","text_sections":[{"label":"Effect","text":"You can include up to 50 copies of cards with this card's card number in your deck.\nOther effect."}]}})}
    #[test] fn official_allowance_is_card_and_value_bound(){let r=rule();let v=observation();assert!(r.validate_observation("PROBE-DECK",&serde_json::to_vec(&v).unwrap()).is_ok());for key in ["source","id","card","count","label"]{let mut v=observation();match key{"source"=>v["source"]="secondary".into(),"id"=>v["observation_id"]="b".repeat(64).into(),"card"=>v["extracted"]["card_id"]="OTHER".into(),"count"=>v["extracted"]["text_sections"][0]["text"]="You can include up to 5 copies of cards with this card's card number in your deck.".into(),_=>v["extracted"]["text_sections"][0]["label"]="Inherited Effect".into()}assert!(r.validate_observation("PROBE-DECK",&serde_json::to_vec(&v).unwrap()).is_err(),"{key}");}}
    #[test] fn missing_and_malformed_evidence_fail_closed(){let r=rule();for bytes in [b"".as_slice(),b"{}",b"null"]{assert!(r.validate_observation("PROBE-DECK",bytes).is_err());}}
}
