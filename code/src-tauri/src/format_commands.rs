use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FormatDto {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub deck_label: String,
    pub population_pct: u8,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

const ENGINE_STANDARD_ONLY_REASON: &str = "Engine supports Standard only in this build";

fn presentation(id: &str) -> (&'static str, u8) {
    match id {
        "standard" => ("The official ruleset", 84),
        "no_restriction" => ("Every card. Every printing.", 23),
        "pauper" => ("Commons and uncommons only", 18),
        "eden" => ("Rotation-light, anomaly-gated", 27),
        "eden_singleton" => ("EDEN, highlander", 12),
        _ => ("", 0),
    }
}

#[tauri::command]
pub fn formats_list() -> Vec<FormatDto> {
    digimon_engine::format::list_formats()
        .iter()
        .map(|f| {
            let (tagline, population_pct) = presentation(&f.id);
            FormatDto {
                id: f.id.clone(),
                name: f.name.to_uppercase(),
                tagline: tagline.to_string(),
                description: f.description.clone(),
                deck_label: if f.singleton {
                    format!("{} singleton", f.deck_size)
                } else {
                    format!("{} cards", f.deck_size)
                },
                population_pct,
                enabled: f.playable,
                disabled_reason: if f.playable {
                    None
                } else {
                    Some(ENGINE_STANDARD_ONLY_REASON.to_string())
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_list_uses_engine_registry_playable_formats() {
        let enabled: Vec<String> = formats_list()
            .into_iter()
            .filter(|format| format.enabled)
            .map(|format| format.id)
            .collect();
        assert_eq!(
            enabled,
            vec![
                "standard",
                "no_restriction",
                "pauper",
                "eden",
                "eden_singleton"
            ]
        );
    }
}
