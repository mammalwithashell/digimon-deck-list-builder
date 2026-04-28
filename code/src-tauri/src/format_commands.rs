use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FormatDto {
    pub id: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub deck_label: &'static str,
    pub population_pct: u8,
    pub enabled: bool,
    pub disabled_reason: Option<&'static str>,
}

const ENGINE_STANDARD_ONLY_REASON: &str = "Engine supports Standard only in this build";

#[tauri::command]
pub fn formats_list() -> Vec<FormatDto> {
    vec![
        FormatDto {
            id: "standard",
            name: "STANDARD",
            tagline: "The official ruleset",
            description: "50-card decks, current banlist, mirrored memory gauge.",
            deck_label: "50 cards",
            population_pct: 84,
            enabled: true,
            disabled_reason: None,
        },
        FormatDto {
            id: "titan",
            name: "TITAN",
            tagline: "Bigger gauges. Bigger threats.",
            description: "75-card deck concept from the mock; disabled until Rules support lands.",
            deck_label: "75 cards",
            population_pct: 42,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "edh",
            name: "EDH",
            tagline: "One herald, one of each, four players",
            description: "100-card singleton concept from the mock; disabled until multiplayer Rules support lands.",
            deck_label: "100 singleton",
            population_pct: 67,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "nobanlist",
            name: "NO BANLIST",
            tagline: "Every card. Every printing.",
            description: "Standard shape without restrictions; disabled until validator support lands.",
            deck_label: "50 cards",
            population_pct: 23,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "draft",
            name: "DRAFT",
            tagline: "Build from a pod",
            description: "Limited mode concept from the mock; disabled until draft pool support lands.",
            deck_label: "40 cards",
            population_pct: 12,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
        FormatDto {
            id: "tutorial",
            name: "TUTORIAL",
            tagline: "Practice the board",
            description: "Guided game concept from the mock; disabled until scripted tutorial support lands.",
            deck_label: "Starter",
            population_pct: 9,
            enabled: false,
            disabled_reason: Some(ENGINE_STANDARD_ONLY_REASON),
        },
    ]
}
