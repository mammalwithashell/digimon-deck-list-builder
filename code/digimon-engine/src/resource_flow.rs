use crate::card_data::CardData;
use crate::enums::Keyword;

pub(crate) fn card_data_indicates_resource_flow(data: &CardData) -> bool {
    data.keywords.iter().any(keyword_indicates_resource_flow)
        || text_indicates_resource_flow(&data.text_for_search_all_faces())
}

pub(crate) fn keyword_indicates_resource_flow(keyword: &Keyword) -> bool {
    matches!(
        keyword,
        Keyword::DrawX(_) | Keyword::Save | Keyword::MaterialSave(_)
    )
}

pub(crate) fn text_indicates_resource_flow(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("draw")
        || text.contains("<save>")
        || text.contains("＜save＞")
        || text.contains("material save")
        || (text.contains("add") && text.contains("hand"))
        || text.contains("search")
}

pub(crate) fn identifier_indicates_resource_flow(identifier: &str) -> bool {
    text_indicates_resource_flow(identifier) || identifier.to_ascii_lowercase().contains("save")
}
