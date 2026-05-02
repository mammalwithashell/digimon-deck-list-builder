use crate::card_data::CardData;

pub fn matches_digixros_name_requirement(card: &CardData, required_name: &str) -> bool {
    name_matches(&card.card_name, required_name)
        || card
            .digixros_aliases
            .iter()
            .any(|alias| name_matches(alias, required_name))
}

pub fn matches_generic_name_requirement(card: &CardData, required_name: &str) -> bool {
    name_matches(&card.card_name, required_name)
}

fn name_matches(candidate: &str, required_name: &str) -> bool {
    candidate
        .to_lowercase()
        .contains(&required_name.to_lowercase())
}

pub fn matches_digixros_name_requirement_for_test(card: &CardData, required_name: &str) -> bool {
    matches_digixros_name_requirement(card, required_name)
}

pub fn matches_generic_name_requirement_for_test(card: &CardData, required_name: &str) -> bool {
    matches_generic_name_requirement(card, required_name)
}
