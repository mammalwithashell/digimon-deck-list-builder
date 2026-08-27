//! Resolves which cards DCGO actually implements — the exam can only examine
//! clauses that exist on both sides.
//!
//! DCGO's card scripts live at
//! `<dcgo_root>/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`, with
//! **underscored** filenames (`BT17-001` → `BT17_001.cs`). The colour
//! subdirectory is not derivable from a card id, so the lookup globs across the
//! colours of the card's set.
//!
//! The check is deliberately **per card, not per set**. DCGO spans AD1,
//! BT1–BT26, EX1–EX12, ST1–ST24, LM, P and RB1, and a set directory routinely
//! exists while an individual card in it has no script (vanilla cards, and
//! cards whose effects DCGO has not scripted). "Newer than DCGO" is therefore
//! the wrong test.

use std::path::{Path, PathBuf};

/// The path, relative to a DCGO checkout root, under which card scripts live.
const CARD_EFFECT_SUBDIR: &str = "Assets/Scripts/CardEffect";

/// Does DCGO have a card-effect script for `card_id`?
///
/// # Panics
///
/// Panics if `<dcgo_root>/Assets/Scripts/CardEffect` does not exist. This is a
/// loud failure on purpose: in a git worktree the local `./DCGO` is an
/// intentionally-empty placeholder (CLAUDE.md rule 29), and a checker pointed
/// there would report *every* card unavailable — quietly turning the whole exam
/// into a no-op that reads as "nothing to verify". Resolve the base-repo
/// checkout instead:
/// `$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO`.
pub fn has_dcgo_script(dcgo_root: &Path, card_id: &str) -> bool {
    let card_effect = dcgo_root.join(CARD_EFFECT_SUBDIR);
    if !card_effect.is_dir() {
        panic!(
            "DCGO card-effect directory not found at {}. In a worktree the local ./DCGO is an \
             empty placeholder — resolve the base-repo checkout with \
             `$(dirname \"$(git rev-parse --path-format=absolute --git-common-dir)\")/DCGO` \
             (CLAUDE.md rule 29). Refusing to report every card as unavailable.",
            card_effect.display()
        );
    }

    let Some(set_dir) = set_dir_for(&card_effect, card_id) else {
        return false;
    };
    // A missing set directory is an ordinary absence, not an error: DCGO simply
    // has no scripts for that set yet.
    let Ok(colours) = std::fs::read_dir(&set_dir) else {
        return false;
    };

    let file_name = format!("{}.cs", card_id.replace('-', "_"));
    for colour in colours.flatten() {
        if !colour.path().is_dir() {
            continue;
        }
        if colour.path().join(&file_name).is_file() {
            return true;
        }
    }
    false
}

/// `EX12-035` → `<card_effect>/EX12`. Returns `None` for an id with no set
/// prefix, which cannot name a DCGO script.
fn set_dir_for(card_effect: &Path, card_id: &str) -> Option<PathBuf> {
    let (set, rest) = card_id.split_once('-')?;
    if set.is_empty() || rest.is_empty() {
        return None;
    }
    Some(card_effect.join(set))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The plan's tests take a bare `tempdir()`; `tempfile`'s returns a
    /// `Result`. The `TempDir` must be bound by the caller so the directory
    /// outlives the assertions.
    fn tempdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    fn fake_checkout(tmp: &std::path::Path) {
        // Mirrors the real layout: CardEffect/<SET>/<COLOR>/<CARD_ID>.cs, with
        // UNDERSCORED filenames (BT17-001 -> BT17_001.cs).
        let dir = tmp.join("Assets/Scripts/CardEffect/EX12/Red");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("EX12_035.cs"), "// stub").unwrap();
        fs::create_dir_all(tmp.join("Assets/Scripts/CardEffect/BT26/Blue")).unwrap();
    }

    #[test]
    fn finds_a_card_dcgo_implements() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(has_dcgo_script(tmp.path(), "EX12-035"));
    }

    #[test]
    fn a_card_in_an_existing_set_but_with_no_script_is_absent() {
        // The per-card, not per-set, rule: a set directory can exist while an
        // individual card has none. "Newer than DCGO" is the WRONG test --
        // DCGO spans BT1-BT26, EX1-EX12, ST1-ST24, AD1, LM, P, RB1.
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(!has_dcgo_script(tmp.path(), "BT26-001"));
    }

    #[test]
    fn a_card_from_a_set_dcgo_does_not_have_is_absent() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        assert!(!has_dcgo_script(tmp.path(), "BT27-001"));
    }

    #[test]
    fn the_underscore_naming_convention_is_honored() {
        let tmp = tempdir();
        fake_checkout(tmp.path());
        // The hyphenated filename must NOT match -- DCGO uses underscores.
        assert!(!tmp
            .path()
            .join("Assets/Scripts/CardEffect/EX12/Red/EX12-035.cs")
            .exists());
        assert!(has_dcgo_script(tmp.path(), "EX12-035"));
    }

    #[test]
    #[should_panic(expected = "DCGO card-effect directory not found")]
    fn an_empty_placeholder_checkout_fails_loudly_instead_of_reporting_unavailable() {
        // A worktree's ./DCGO placeholder has no Assets/ tree at all. Returning
        // false here would mark every card unavailable and silently reduce the
        // exam to a no-op, so this must panic rather than answer.
        let tmp = tempdir();
        let _ = has_dcgo_script(tmp.path(), "EX12-035");
    }
}
