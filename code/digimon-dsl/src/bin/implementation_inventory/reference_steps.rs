// Explicit typed patterns: all fields and variants, no rest/wildcard patterns.
use super::{Collector, Walk};
impl Walk for digimon_dsl::step::StepSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::StepSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::StepSpec::GainMemory(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GainMemory"),
                );
                f0.walk(c, &format!("{}@GainMemory.0", path), role);
            }
            digimon_dsl::step::StepSpec::LoseMemory(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("LoseMemory"),
                );
                f0.walk(c, &format!("{}@LoseMemory.0", path), role);
            }
            digimon_dsl::step::StepSpec::SetMemory(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SetMemory"),
                );
                f0.walk(c, &format!("{}@SetMemory.0", path), role);
            }
            digimon_dsl::step::StepSpec::Draw(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Draw"),
                );
                f0.walk(c, &format!("{}@Draw.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashFromTop(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashFromTop"),
                );
                f0.walk(c, &format!("{}@TrashFromTop.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddToHandFromDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddToHandFromDeck"),
                );
                f0.walk(c, &format!("{}@AddToHandFromDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddToHandFromTrash(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddToHandFromTrash"),
                );
                f0.walk(c, &format!("{}@AddToHandFromTrash.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddToHandFromSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddToHandFromSecurity"),
                );
                f0.walk(c, &format!("{}@AddToHandFromSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaySecurityCard(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaySecurityCard"),
                );
                f0.walk(c, &format!("{}@PlaySecurityCard.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashSelectedSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashSelectedSecurity"),
                );
                f0.walk(c, &format!("{}@TrashSelectedSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnSelectedSecurityToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnSelectedSecurityToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnSelectedSecurityToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnTopSecurityToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnTopSecurityToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnTopSecurityToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddTopSecurityToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddTopSecurityToHand"),
                );
                f0.walk(c, &format!("{}@AddTopSecurityToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::MayAddTopSecurityToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MayAddTopSecurityToHand"),
                );
                f0.walk(c, &format!("{}@MayAddTopSecurityToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddToHandFromReveal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddToHandFromReveal"),
                );
                f0.walk(c, &format!("{}@AddToHandFromReveal.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddThisOptionToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddThisOptionToHand"),
                );
                f0.walk(c, &format!("{}@AddThisOptionToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashFromHandByIndex(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashFromHandByIndex"),
                );
                f0.walk(c, &format!("{}@TrashFromHandByIndex.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashFromReveal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashFromReveal"),
                );
                f0.walk(c, &format!("{}@TrashFromReveal.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnToDeckFromReveal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnToDeckFromReveal"),
                );
                f0.walk(c, &format!("{}@ReturnToDeckFromReveal.0", path), role);
            }
            digimon_dsl::step::StepSpec::ShuffleDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ShuffleDeck"),
                );
                f0.walk(c, &format!("{}@ShuffleDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::ShuffleSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ShuffleSecurity"),
                );
                f0.walk(c, &format!("{}@ShuffleSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::RevealSecurity(f0) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("RevealSecurity"));
                f0.walk(c, &format!("{}@RevealSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::RevealTopDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RevealTopDeck"),
                );
                f0.walk(c, &format!("{}@RevealTopDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceRemainderOnDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceRemainderOnDeck"),
                );
                f0.walk(c, &format!("{}@PlaceRemainderOnDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::RevealSearch(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RevealSearch"),
                );
                f0.walk(c, &format!("{}@RevealSearch.0", path), role);
            }
            digimon_dsl::step::StepSpec::ChooseFromReveal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ChooseFromReveal"),
                );
                f0.walk(c, &format!("{}@ChooseFromReveal.0", path), role);
            }
            digimon_dsl::step::StepSpec::OrderRemainder(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("OrderRemainder"),
                );
                f0.walk(c, &format!("{}@OrderRemainder.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeletePermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeletePermanent"),
                );
                f0.walk(c, &format!("{}@DeletePermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeleteForCostReduction(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeleteForCostReduction"),
                );
                f0.walk(c, &format!("{}@DeleteForCostReduction.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeleteBoundPermanents(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeleteBoundPermanents"),
                );
                f0.walk(c, &format!("{}@DeleteBoundPermanents.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeletePermanents(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeletePermanents"),
                );
                f0.walk(c, &format!("{}@DeletePermanents.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeleteAllPermanents(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeleteAllPermanents"),
                );
                f0.walk(c, &format!("{}@DeleteAllPermanents.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeleteOnePerOpponentColor(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeleteOnePerOpponentColor"),
                );
                f0.walk(c, &format!("{}@DeleteOnePerOpponentColor.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashBreedingPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashBreedingPermanent"),
                );
                f0.walk(c, &format!("{}@TrashBreedingPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnToHand"),
                );
                f0.walk(c, &format!("{}@ReturnToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::Suspend(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Suspend"),
                );
                f0.walk(c, &format!("{}@Suspend.0", path), role);
            }
            digimon_dsl::step::StepSpec::Unsuspend(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Unsuspend"),
                );
                f0.walk(c, &format!("{}@Unsuspend.0", path), role);
            }
            digimon_dsl::step::StepSpec::DeDigivolve(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DeDigivolve"),
                );
                f0.walk(c, &format!("{}@DeDigivolve.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceOnSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceOnSecurity"),
                );
                f0.walk(c, &format!("{}@PlaceOnSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayToken(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayToken"),
                );
                f0.walk(c, &format!("{}@PlayToken.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceAsBottomSource(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceAsBottomSource"),
                );
                f0.walk(c, &format!("{}@PlaceAsBottomSource.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceAsTopSource(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceAsTopSource"),
                );
                f0.walk(c, &format!("{}@PlaceAsTopSource.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceTopSourceAsBottom(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceTopSourceAsBottom"),
                );
                f0.walk(c, &format!("{}@PlaceTopSourceAsBottom.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashTopSource(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashTopSource"),
                );
                f0.walk(c, &format!("{}@TrashTopSource.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashBottomSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashBottomSources"),
                );
                f0.walk(c, &format!("{}@TrashBottomSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashAllSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashAllSources"),
                );
                f0.walk(c, &format!("{}@TrashAllSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashSelectedSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashSelectedSources"),
                );
                f0.walk(c, &format!("{}@TrashSelectedSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashSelectedMaterials(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashSelectedMaterials"),
                );
                f0.walk(c, &format!("{}@TrashSelectedMaterials.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceSelectedCardUnderTamer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceSelectedCardUnderTamer"),
                );
                f0.walk(c, &format!("{}@PlaceSelectedCardUnderTamer.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceSelectedSourcesUnderTamer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceSelectedSourcesUnderTamer"),
                );
                f0.walk(
                    c,
                    &format!("{}@PlaceSelectedSourcesUnderTamer.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::MoveMatchingSourcesUnderTamer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MoveMatchingSourcesUnderTamer"),
                );
                f0.walk(
                    c,
                    &format!("{}@MoveMatchingSourcesUnderTamer.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::TrashTopStackedSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashTopStackedSources"),
                );
                f0.walk(c, &format!("{}@TrashTopStackedSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnSelectedSourcesToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnSelectedSourcesToHand"),
                );
                f0.walk(c, &format!("{}@ReturnSelectedSourcesToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnSelectedSourcesToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnSelectedSourcesToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnSelectedSourcesToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnAllSourcesToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnAllSourcesToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnAllSourcesToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashBottomFaceDownSourceUnderTamer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashBottomFaceDownSourceUnderTamer"),
                );
                f0.walk(
                    c,
                    &format!("{}@TrashBottomFaceDownSourceUnderTamer.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::TrashBottomFaceDownSourcesUnderTamers(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashBottomFaceDownSourcesUnderTamers"),
                );
                f0.walk(
                    c,
                    &format!("{}@TrashBottomFaceDownSourcesUnderTamers.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::TrashLinkCardOfOwnDigimon(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashLinkCardOfOwnDigimon"),
                );
                f0.walk(c, &format!("{}@TrashLinkCardOfOwnDigimon.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashOptionFromOwnStacks(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashOptionFromOwnStacks"),
                );
                f0.walk(c, &format!("{}@TrashOptionFromOwnStacks.0", path), role);
            }
            digimon_dsl::step::StepSpec::BindPermanentProperty(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("BindPermanentProperty"),
                );
                f0.walk(c, &format!("{}@BindPermanentProperty.0", path), role);
            }
            digimon_dsl::step::StepSpec::Hatch(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Hatch"),
                );
                f0.walk(c, &format!("{}@Hatch.0", path), role);
            }
            digimon_dsl::step::StepSpec::MoveFromBreeding(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MoveFromBreeding"),
                );
                f0.walk(c, &format!("{}@MoveFromBreeding.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromHand"),
                );
                f0.walk(c, &format!("{}@PlayFromHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromHandFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromHandFree"),
                );
                f0.walk(c, &format!("{}@PlayFromHandFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::UseOptionFromHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("UseOptionFromHand"),
                );
                f0.walk(c, &format!("{}@UseOptionFromHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::UseOptionFromTrash(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("UseOptionFromTrash"),
                );
                f0.walk(c, &format!("{}@UseOptionFromTrash.0", path), role);
            }
            digimon_dsl::step::StepSpec::UseOptionFromRevealed(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("UseOptionFromRevealed"),
                );
                f0.walk(c, &format!("{}@UseOptionFromRevealed.0", path), role);
            }
            digimon_dsl::step::StepSpec::UseOptionFromSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("UseOptionFromSources"),
                );
                f0.walk(c, &format!("{}@UseOptionFromSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::UseOptionBound(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("UseOptionBound"),
                );
                f0.walk(c, &format!("{}@UseOptionBound.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayOrUseFromHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayOrUseFromHand"),
                );
                f0.walk(c, &format!("{}@PlayOrUseFromHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayOrUseFromSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayOrUseFromSources"),
                );
                f0.walk(c, &format!("{}@PlayOrUseFromSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromRevealedFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromRevealedFree"),
                );
                f0.walk(c, &format!("{}@PlayFromRevealedFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromTrash(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromTrash"),
                );
                f0.walk(c, &format!("{}@PlayFromTrash.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromTrashFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromTrashFree"),
                );
                f0.walk(c, &format!("{}@PlayFromTrashFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayUnionBoundFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayUnionBoundFree"),
                );
                f0.walk(c, &format!("{}@PlayUnionBoundFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashUnionBound(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashUnionBound"),
                );
                f0.walk(c, &format!("{}@TrashUnionBound.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnUnionBoundToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnUnionBoundToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnUnionBoundToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromSecurity"),
                );
                f0.walk(c, &format!("{}@PlayFromSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayFromMaterials(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayFromMaterials"),
                );
                f0.walk(c, &format!("{}@PlayFromMaterials.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaySelectedSourcesFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaySelectedSourcesFree"),
                );
                f0.walk(c, &format!("{}@PlaySelectedSourcesFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayUnderTamerSource(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayUnderTamerSource"),
                );
                f0.walk(c, &format!("{}@PlayUnderTamerSource.0", path), role);
            }
            digimon_dsl::step::StepSpec::EffectInitiatedDigivolve(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("EffectInitiatedDigivolve"),
                );
                f0.walk(c, &format!("{}@EffectInitiatedDigivolve.0", path), role);
            }
            digimon_dsl::step::StepSpec::EffectInitiatedDnaDigivolve(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("EffectInitiatedDnaDigivolve"),
                );
                f0.walk(c, &format!("{}@EffectInitiatedDnaDigivolve.0", path), role);
            }
            digimon_dsl::step::StepSpec::EffectInitiatedDnaDigivolveHandPartner(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("EffectInitiatedDnaDigivolveHandPartner"),
                );
                f0.walk(
                    c,
                    &format!("{}@EffectInitiatedDnaDigivolveHandPartner.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::EffectInitiatedDnaDigivolveTrashPartner(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("EffectInitiatedDnaDigivolveTrashPartner"),
                );
                f0.walk(
                    c,
                    &format!("{}@EffectInitiatedDnaDigivolveTrashPartner.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::TrashTopSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashTopSecurity"),
                );
                f0.walk(c, &format!("{}@TrashTopSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashBottomSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashBottomSecurity"),
                );
                f0.walk(c, &format!("{}@TrashBottomSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddBottomSecurityToHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddBottomSecurityToHand"),
                );
                f0.walk(c, &format!("{}@AddBottomSecurityToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashTopSecurityAndCancelReplacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashTopSecurityAndCancelReplacement"),
                );
                f0.walk(
                    c,
                    &format!("{}@TrashTopSecurityAndCancelReplacement.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::BounceSelf(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("BounceSelf"),
                );
                f0.walk(c, &format!("{}@BounceSelf.0", path), role);
            }
            digimon_dsl::step::StepSpec::MoveSelfOptionUnderPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MoveSelfOptionUnderPermanent"),
                );
                f0.walk(c, &format!("{}@MoveSelfOptionUnderPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceSelfUnderPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceSelfUnderPermanent"),
                );
                f0.walk(c, &format!("{}@PlaceSelfUnderPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::SecurityPlaceStackedCard(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SecurityPlaceStackedCard"),
                );
                f0.walk(c, &format!("{}@SecurityPlaceStackedCard.0", path), role);
            }
            digimon_dsl::step::StepSpec::SecurityPlaceTopStackedCard(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SecurityPlaceTopStackedCard"),
                );
                f0.walk(c, &format!("{}@SecurityPlaceTopStackedCard.0", path), role);
            }
            digimon_dsl::step::StepSpec::SecurityPlaceTopCard(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SecurityPlaceTopCard"),
                );
                f0.walk(c, &format!("{}@SecurityPlaceTopCard.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnAllTrashToDeckBottom(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnAllTrashToDeckBottom"),
                );
                f0.walk(c, &format!("{}@ReturnAllTrashToDeckBottom.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnTrashListToDeckBottom(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnTrashListToDeckBottom"),
                );
                f0.walk(c, &format!("{}@ReturnTrashListToDeckBottom.0", path), role);
            }
            digimon_dsl::step::StepSpec::MoveTrashCardToDeckTop(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MoveTrashCardToDeckTop"),
                );
                f0.walk(c, &format!("{}@MoveTrashCardToDeckTop.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashTopNDigivolutionCardsOfEach(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashTopNDigivolutionCardsOfEach"),
                );
                f0.walk(
                    c,
                    &format!("{}@TrashTopNDigivolutionCardsOfEach.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::TrashOpponentHandToCount(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("TrashOpponentHandToCount"),
                );
                f0.walk(c, &format!("{}@TrashOpponentHandToCount.0", path), role);
            }
            digimon_dsl::step::StepSpec::SearchOwnSecurityStack(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SearchOwnSecurityStack"),
                );
                f0.walk(c, &format!("{}@SearchOwnSecurityStack.0", path), role);
            }
            digimon_dsl::step::StepSpec::Recover(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Recover"),
                );
                f0.walk(c, &format!("{}@Recover.0", path), role);
            }
            digimon_dsl::step::StepSpec::MarkSecurityFaceUp(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MarkSecurityFaceUp"),
                );
                f0.walk(c, &format!("{}@MarkSecurityFaceUp.0", path), role);
            }
            digimon_dsl::step::StepSpec::FlipSecurityFaceUp(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("FlipSecurityFaceUp"),
                );
                f0.walk(c, &format!("{}@FlipSecurityFaceUp.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddDpModifier(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddDpModifier"),
                );
                f0.walk(c, &format!("{}@AddDpModifier.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddModifier(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddModifier"),
                );
                f0.walk(c, &format!("{}@AddModifier.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddPlayerModifier(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddPlayerModifier"),
                );
                f0.walk(c, &format!("{}@AddPlayerModifier.0", path), role);
            }
            digimon_dsl::step::StepSpec::GrantKeyword(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantKeyword"),
                );
                f0.walk(c, &format!("{}@GrantKeyword.0", path), role);
            }
            digimon_dsl::step::StepSpec::GrantKeywordContinuous(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantKeywordContinuous"),
                );
                f0.walk(c, &format!("{}@GrantKeywordContinuous.0", path), role);
            }
            digimon_dsl::step::StepSpec::AllowDigixrosMaterialZone(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AllowDigixrosMaterialZone"),
                );
                f0.walk(c, &format!("{}@AllowDigixrosMaterialZone.0", path), role);
            }
            digimon_dsl::step::StepSpec::AddDigixrosCostDelta(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddDigixrosCostDelta"),
                );
                f0.walk(c, &format!("{}@AddDigixrosCostDelta.0", path), role);
            }
            digimon_dsl::step::StepSpec::PreattachDigixrosMaterial(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PreattachDigixrosMaterial"),
                );
                f0.walk(c, &format!("{}@PreattachDigixrosMaterial.0", path), role);
            }
            digimon_dsl::step::StepSpec::RegisterDigixrosWildcardForTurn(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RegisterDigixrosWildcardForTurn"),
                );
                f0.walk(
                    c,
                    &format!("{}@RegisterDigixrosWildcardForTurn.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::AddDigixrosWildcardToPendingTransaction(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AddDigixrosWildcardToPendingTransaction"),
                );
                f0.walk(
                    c,
                    &format!("{}@AddDigixrosWildcardToPendingTransaction.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::GrantEffectImmunity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantEffectImmunity"),
                );
                f0.walk(c, &format!("{}@GrantEffectImmunity.0", path), role);
            }
            digimon_dsl::step::StepSpec::GrantNarrowOpponentEffectProtection(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantNarrowOpponentEffectProtection"),
                );
                f0.walk(
                    c,
                    &format!("{}@GrantNarrowOpponentEffectProtection.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::GrantStackProtectionFromOpponentEffects(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantStackProtectionFromOpponentEffects"),
                );
                f0.walk(
                    c,
                    &format!("{}@GrantStackProtectionFromOpponentEffects.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::GrantStackTrashImmunityFromOpponentEffects(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantStackTrashImmunityFromOpponentEffects"),
                );
                f0.walk(
                    c,
                    &format!("{}@GrantStackTrashImmunityFromOpponentEffects.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::GrantTriggeredEffect(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("GrantTriggeredEffect"),
                );
                f0.walk(c, &format!("{}@GrantTriggeredEffect.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOwnPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOwnPermanent"),
                );
                f0.walk(c, &format!("{}@SelectOwnPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOpponentPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOpponentPermanent"),
                );
                f0.walk(c, &format!("{}@SelectOpponentPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectAnyPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectAnyPermanent"),
                );
                f0.walk(c, &format!("{}@SelectAnyPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectDnaPair(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectDnaPair"),
                );
                f0.walk(c, &format!("{}@SelectDnaPair.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectHand(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectHand"),
                );
                f0.walk(c, &format!("{}@SelectHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectTrash(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectTrash"),
                );
                f0.walk(c, &format!("{}@SelectTrash.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectMaterial(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectMaterial"),
                );
                f0.walk(c, &format!("{}@SelectMaterial.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectMaterials(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectMaterials"),
                );
                f0.walk(c, &format!("{}@SelectMaterials.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOwnSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOwnSources"),
                );
                f0.walk(c, &format!("{}@SelectOwnSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectUnderTamerSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectUnderTamerSources"),
                );
                f0.walk(c, &format!("{}@SelectUnderTamerSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOpponentSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOpponentSources"),
                );
                f0.walk(c, &format!("{}@SelectOpponentSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::DigiBurst(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("DigiBurst"),
                );
                f0.walk(c, &format!("{}@DigiBurst.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOpponentDpBudget(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOpponentDpBudget"),
                );
                f0.walk(c, &format!("{}@SelectOpponentDpBudget.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOpponentPlayCostBudget(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOpponentPlayCostBudget"),
                );
                f0.walk(c, &format!("{}@SelectOpponentPlayCostBudget.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectZoneCards(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectZoneCards"),
                );
                f0.walk(c, &format!("{}@SelectZoneCards.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayCardsFree(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlayCardsFree"),
                );
                f0.walk(c, &format!("{}@PlayCardsFree.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlaceCardsAsBottomSources(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceCardsAsBottomSources"),
                );
                f0.walk(c, &format!("{}@PlaceCardsAsBottomSources.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnTopStackedToDeck(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReturnTopStackedToDeck"),
                );
                f0.walk(c, &format!("{}@ReturnTopStackedToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOwnBreedingPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOwnBreedingPermanent"),
                );
                f0.walk(c, &format!("{}@SelectOwnBreedingPermanent.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectReveal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectReveal"),
                );
                f0.walk(c, &format!("{}@SelectReveal.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectRevealBuckets(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectRevealBuckets"),
                );
                f0.walk(c, &format!("{}@SelectRevealBuckets.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectSecurity(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectSecurity"),
                );
                f0.walk(c, &format!("{}@SelectSecurity.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectUnionZone(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectUnionZone"),
                );
                f0.walk(c, &format!("{}@SelectUnionZone.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectOrderedPermutation(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectOrderedPermutation"),
                );
                f0.walk(c, &format!("{}@SelectOrderedPermutation.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectCountCappedMulti(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectCountCappedMulti"),
                );
                f0.walk(c, &format!("{}@SelectCountCappedMulti.0", path), role);
            }
            digimon_dsl::step::StepSpec::SelectEffectChoice(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SelectEffectChoice"),
                );
                f0.walk(c, &format!("{}@SelectEffectChoice.0", path), role);
            }
            digimon_dsl::step::StepSpec::RepeatEffectChoice(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RepeatEffectChoice"),
                );
                f0.walk(c, &format!("{}@RepeatEffectChoice.0", path), role);
            }
            digimon_dsl::step::StepSpec::AsSelectingPlayer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AsSelectingPlayer"),
                );
                f0.walk(c, &format!("{}@AsSelectingPlayer.0", path), role);
            }
            digimon_dsl::step::StepSpec::If(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("If"),
                );
                f0.walk(c, &format!("{}@If.0", path), role);
            }
            digimon_dsl::step::StepSpec::ForEach(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ForEach"),
                );
                f0.walk(c, &format!("{}@ForEach.0", path), role);
            }
            digimon_dsl::step::StepSpec::PerSelected(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PerSelected"),
                );
                f0.walk(c, &format!("{}@PerSelected.0", path), role);
            }
            digimon_dsl::step::StepSpec::ScheduleDelayed(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ScheduleDelayed"),
                );
                f0.walk(c, &format!("{}@ScheduleDelayed.0", path), role);
            }
            digimon_dsl::step::StepSpec::ScheduleDeletePlayedAtTurnEnd(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ScheduleDeletePlayedAtTurnEnd"),
                );
                f0.walk(
                    c,
                    &format!("{}@ScheduleDeletePlayedAtTurnEnd.0", path),
                    role,
                );
            }
            digimon_dsl::step::StepSpec::PlaceSelfAsDelayOption(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("PlaceSelfAsDelayOption"),
                );
                f0.walk(c, &format!("{}@PlaceSelfAsDelayOption.0", path), role);
            }
            digimon_dsl::step::StepSpec::LinkToOwnDigimon(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("LinkToOwnDigimon"),
                );
                f0.walk(c, &format!("{}@LinkToOwnDigimon.0", path), role);
            }
            digimon_dsl::step::StepSpec::AppFuse(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("AppFuse"),
                );
                f0.walk(c, &format!("{}@AppFuse.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReduceLinkCost(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ReduceLinkCost"),
                );
                f0.walk(c, &format!("{}@ReduceLinkCost.0", path), role);
            }
            digimon_dsl::step::StepSpec::LinkCards(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("LinkCards"),
                );
                f0.walk(c, &format!("{}@LinkCards.0", path), role);
            }
            digimon_dsl::step::StepSpec::RelinkSelfToOwnDigimon(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RelinkSelfToOwnDigimon"),
                );
                f0.walk(c, &format!("{}@RelinkSelfToOwnDigimon.0", path), role);
            }
            digimon_dsl::step::StepSpec::Optional(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Optional"),
                );
                f0.walk(c, &format!("{}@Optional.0", path), role);
            }
            digimon_dsl::step::StepSpec::Battle(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("Battle"),
                );
                f0.walk(c, &format!("{}@Battle.0", path), role);
            }
            digimon_dsl::step::StepSpec::MayAttackNow(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MayAttackNow"),
                );
                f0.walk(c, &format!("{}@MayAttackNow.0", path), role);
            }
            digimon_dsl::step::StepSpec::ForceAttack(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ForceAttack"),
                );
                f0.walk(c, &format!("{}@ForceAttack.0", path), role);
            }
            digimon_dsl::step::StepSpec::RedirectAttackTarget(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RedirectAttackTarget"),
                );
                f0.walk(c, &format!("{}@RedirectAttackTarget.0", path), role);
            }
            digimon_dsl::step::StepSpec::CancelAttack(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("CancelAttack"),
                );
                f0.walk(c, &format!("{}@CancelAttack.0", path), role);
            }
            digimon_dsl::step::StepSpec::OpenCounterWindow(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("OpenCounterWindow"),
                );
                f0.walk(c, &format!("{}@OpenCounterWindow.0", path), role);
            }
            digimon_dsl::step::StepSpec::RefundOpt(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RefundOpt"),
                );
                f0.walk(c, &format!("{}@RefundOpt.0", path), role);
            }
            digimon_dsl::step::StepSpec::RefireEffect(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RefireEffect"),
                );
                f0.walk(c, &format!("{}@RefireEffect.0", path), role);
            }
            digimon_dsl::step::StepSpec::RefireCardEffect(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RefireCardEffect"),
                );
                f0.walk(c, &format!("{}@RefireCardEffect.0", path), role);
            }
            digimon_dsl::step::StepSpec::EndAttack(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("EndAttack"),
                );
                f0.walk(c, &format!("{}@EndAttack.0", path), role);
            }
            digimon_dsl::step::StepSpec::CancelReplacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("CancelReplacement"),
                );
                f0.walk(c, &format!("{}@CancelReplacement.0", path), role);
            }
            digimon_dsl::step::StepSpec::HandleReplacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("HandleReplacement"),
                );
                f0.walk(c, &format!("{}@HandleReplacement.0", path), role);
            }
            digimon_dsl::step::StepSpec::RedirectReplacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RedirectReplacement"),
                );
                f0.walk(c, &format!("{}@RedirectReplacement.0", path), role);
            }
            digimon_dsl::step::StepSpec::SubstituteReplacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("SubstituteReplacement"),
                );
                f0.walk(c, &format!("{}@SubstituteReplacement.0", path), role);
            }
            digimon_dsl::step::StepSpec::ArmDigivolveCostReducer(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ArmDigivolveCostReducer"),
                );
                f0.walk(c, &format!("{}@ArmDigivolveCostReducer.0", path), role);
            }
            digimon_dsl::step::StepSpec::MayDnaDigivolveNow(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("MayDnaDigivolveNow"),
                );
                f0.walk(c, &format!("{}@MayDnaDigivolveNow.0", path), role);
            }
            digimon_dsl::step::StepSpec::RawRust(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("RawRust"),
                );
                f0.walk(c, &format!("{}@RawRust.0", path), role);
            }
            digimon_dsl::step::StepSpec::ActivationCost(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::StepSpec",
                    "VARIANT",
                    serde_json::json!("ActivationCost"),
                );
                f0.walk(c, &format!("{}@ActivationCost.0", path), role);
            }
                    digimon_dsl::step::StepSpec::PaySourcePrefix(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("PaySourcePrefix"));
                value.walk(c, &format!("{}@PaySourcePrefix.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnAllToHand(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("ReturnAllToHand"));
                value.walk(c, &format!("{}@ReturnAllToHand.0", path), role);
            }
            digimon_dsl::step::StepSpec::ReturnAllToDeck(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("ReturnAllToDeck"));
                value.walk(c, &format!("{}@ReturnAllToDeck.0", path), role);
            }
            digimon_dsl::step::StepSpec::SuspendAndCancelAllMatchingLeaves(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("SuspendAndCancelAllMatchingLeaves"));
                value.walk(c, &format!("{}@SuspendAndCancelAllMatchingLeaves.0", path), role);
            }
            digimon_dsl::step::StepSpec::TrashSelectedSourcesAsCost(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("TrashSelectedSourcesAsCost"));
                value.walk(c, &format!("{}@TrashSelectedSourcesAsCost.0", path), role);
            }
            digimon_dsl::step::StepSpec::PlayToBreedingFromHand(value) => {
                c.node(path, "digimon_dsl::step::StepSpec", "VARIANT", serde_json::json!("PlayToBreedingFromHand"));
                value.walk(c, &format!("{}@PlayToBreedingFromHand.0", path), role);
            }
}
    }
}
impl Walk for digimon_dsl::step::BindingRef {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::BindingRef",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::BindingRef::Structured(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::BindingRef",
                    "VARIANT",
                    serde_json::json!("Structured"),
                );
                f0.walk(
                    c,
                    &format!("{}@Structured.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::step::BindingRef::Named(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::BindingRef",
                    "VARIANT",
                    serde_json::json!("Named"),
                );
                f0.walk(c, &format!("{}@Named.0", path), "EXISTING_INSTANCE_BINDING");
            }
        }
    }
}
impl Walk for digimon_dsl::step::DeleteBoundPermanentsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeleteBoundPermanentsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DeleteBoundPermanentsArgs { binding } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::DeletePermanentsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeletePermanentsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DeletePermanentsArgs { targets } = self;
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DeleteAllPermanentsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeleteAllPermanentsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DeleteAllPermanentsArgs { over, selector } = self;
        over.walk(c, &format!("{}.over", path), "UNCLASSIFIED_STRING");
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DeleteOnePerOpponentColorArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeleteOnePerOpponentColorArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DeleteOnePerOpponentColorArgs {
            filter,
            prompt,
            prompt_key,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::ReduceLinkCostArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReduceLinkCostArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReduceLinkCostArgs { amount } = self;
        amount.walk(c, &format!("{}.amount", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashBreedingPermanentArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashBreedingPermanentArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashBreedingPermanentArgs { target } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::ReturnTrashListToDeckBottomArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnTrashListToDeckBottomArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnTrashListToDeckBottomArgs {
            of,
            cards,
            destination,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        cards.walk(c, &format!("{}.cards", path), "UNCLASSIFIED_STRING");
        destination.walk(c, &format!("{}.destination", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DeckDestination {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeckDestination",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::DeckDestination::Top => c.node(
                path,
                "digimon_dsl::step::DeckDestination",
                "ENUM_SCALAR",
                serde_json::json!("Top"),
            ),
            digimon_dsl::step::DeckDestination::Bottom => c.node(
                path,
                "digimon_dsl::step::DeckDestination",
                "ENUM_SCALAR",
                serde_json::json!("Bottom"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::MoveTrashCardToDeckTopArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MoveTrashCardToDeckTopArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MoveTrashCardToDeckTopArgs { of, card } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::ModifierTarget {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ModifierTarget",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::ModifierTarget::Binding(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::ModifierTarget",
                    "VARIANT",
                    serde_json::json!("Binding"),
                );
                f0.walk(
                    c,
                    &format!("{}@Binding.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::step::ModifierTarget::Filter(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::ModifierTarget",
                    "VARIANT",
                    serde_json::json!("Filter"),
                );
                f0.walk(c, &format!("{}@Filter.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::step::StructuredBindingRef {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::StructuredBindingRef",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::StructuredBindingRef {
            binding,
            permanent,
            source_permanent,
            zone,
            of_permanent,
            deck_top,
            security_top,
            own_breeding,
            top_of,
        } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        permanent.walk(
            c,
            &format!("{}.permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        source_permanent.walk(
            c,
            &format!("{}.source_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        of_permanent.walk(
            c,
            &format!("{}.of_permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        deck_top.walk(c, &format!("{}.deck_top", path), "UNCLASSIFIED_STRING");
        security_top.walk(c, &format!("{}.security_top", path), "UNCLASSIFIED_STRING");
        own_breeding.walk(c, &format!("{}.own_breeding", path), "UNCLASSIFIED_STRING");
            top_of.walk(c, &format!("{}.top_of", path), "EXISTING_INSTANCE_BINDING");
}
}
impl Walk for digimon_dsl::step::PlayerArg {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayerArg",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayerArg { of } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::MoveFromBreedingArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::MoveFromBreedingArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::MoveFromBreedingArgs { of, bind_as } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "BINDING_DEFINITION");
    }
}
impl Walk for digimon_dsl::step::TrashTopSecurityPreventionArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::TrashTopSecurityPreventionArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::TrashTopSecurityPreventionArgs { of, all_matching } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        all_matching.walk(c, &format!("{}.all_matching", path), role);
    }
}
impl Walk for digimon_dsl::step::ReturnTopSecurityToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnTopSecurityToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnTopSecurityToDeckArgs { of, position } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashTopSecurityArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashTopSecurityArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashTopSecurityArgs {
            of,
            count,
            leave,
            bind_count_as,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        leave.walk(c, &format!("{}.leave", path), "UNCLASSIFIED_STRING");
        bind_count_as.walk(c, &format!("{}.bind_count_as", path), "BINDING_DEFINITION");
    }
}
impl Walk for digimon_dsl::step::TrashBottomFaceDownSourceUnderTamerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashBottomFaceDownSourceUnderTamerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashBottomFaceDownSourceUnderTamerArgs { of, optional } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashBottomFaceDownSourcesUnderTamersArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashBottomFaceDownSourcesUnderTamersArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashBottomFaceDownSourcesUnderTamersArgs { of, count } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashLinkCardOfOwnDigimonArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashLinkCardOfOwnDigimonArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashLinkCardOfOwnDigimonArgs { of, optional } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashOptionFromOwnStacksArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashOptionFromOwnStacksArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashOptionFromOwnStacksArgs { of, optional } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::MoveSelfOptionUnderPermanentArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MoveSelfOptionUnderPermanentArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MoveSelfOptionUnderPermanentArgs { target, face_down } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        face_down.walk(c, &format!("{}.face_down", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlaceSelfUnderPermanentArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceSelfUnderPermanentArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceSelfUnderPermanentArgs { target, face_down } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        face_down.walk(c, &format!("{}.face_down", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::EmptyArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EmptyArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::EmptyArgs {} = self;
    }
}
impl Walk for digimon_dsl::step::ActivationCostArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ActivationCostArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ActivationCostArgs {
            suspend_self,
            return_self_to_deck_bottom,
            trash_self,
            place_self_at_security_bottom,
        } = self;
        suspend_self.walk(c, &format!("{}.suspend_self", path), "UNCLASSIFIED_STRING");
        return_self_to_deck_bottom.walk(
            c,
            &format!("{}.return_self_to_deck_bottom", path),
            "UNCLASSIFIED_STRING",
        );
        trash_self.walk(c, &format!("{}.trash_self", path), "UNCLASSIFIED_STRING");
        place_self_at_security_bottom.walk(
            c,
            &format!("{}.place_self_at_security_bottom", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::ArmDigivolveCostReducerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ArmDigivolveCostReducerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ArmDigivolveCostReducerArgs {
            amount,
            single_fire,
            target_color,
            suspend_cost,
        } = self;
        amount.walk(c, &format!("{}.amount", path), "UNCLASSIFIED_STRING");
        single_fire.walk(c, &format!("{}.single_fire", path), "UNCLASSIFIED_STRING");
        target_color.walk(c, &format!("{}.target_color", path), "UNCLASSIFIED_STRING");
        suspend_cost.walk(c, &format!("{}.suspend_cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TargetArg {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TargetArg",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TargetArg { target } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::TrashBottomSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashBottomSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashBottomSourcesArgs { target, count } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PermanentProperty {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PermanentProperty",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::PermanentProperty::Level => c.node(
                path,
                "digimon_dsl::step::PermanentProperty",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::BindPermanentProperty {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::BindPermanentProperty",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::BindPermanentProperty {
            from,
            property,
            bind_as,
        } = self;
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        property.walk(c, &format!("{}.property", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::BattleArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::BattleArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::BattleArgs {
            attacker,
            defender,
            compare,
        } = self;
        attacker.walk(c, &format!("{}.attacker", path), "UNCLASSIFIED_STRING");
        defender.walk(c, &format!("{}.defender", path), "UNCLASSIFIED_STRING");
        compare.walk(c, &format!("{}.compare", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::BattleCompareSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::BattleCompareSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::BattleCompareSpec::Dp => c.node(
                path,
                "digimon_dsl::step::BattleCompareSpec",
                "ENUM_SCALAR",
                serde_json::json!("Dp"),
            ),
            digimon_dsl::step::BattleCompareSpec::DigivolutionCards => c.node(
                path,
                "digimon_dsl::step::BattleCompareSpec",
                "ENUM_SCALAR",
                serde_json::json!("DigivolutionCards"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::ReturnAllSourcesToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnAllSourcesToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnAllSourcesToDeckArgs { target, position } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::AttackCostUpgradeArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AttackCostUpgradeArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AttackCostUpgradeArgs {
            dp,
            security_attack,
        } = self;
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        security_attack.walk(
            c,
            &format!("{}.security_attack", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::MayAttackNowArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MayAttackNowArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MayAttackNowArgs {
            attacker,
            targets,
            without_suspending,
            ignore_summoning_sickness,
            optional,
            windowed,
            prompt,
            cost_upgrade,
        } = self;
        attacker.walk(c, &format!("{}.attacker", path), "UNCLASSIFIED_STRING");
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        without_suspending.walk(
            c,
            &format!("{}.without_suspending", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_summoning_sickness.walk(
            c,
            &format!("{}.ignore_summoning_sickness", path),
            "UNCLASSIFIED_STRING",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        windowed.walk(c, &format!("{}.windowed", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        cost_upgrade.walk(c, &format!("{}.cost_upgrade", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ForceAttackArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ForceAttackArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ForceAttackArgs {
            attacker,
            targets,
            without_suspending,
            prompt,
            cost_upgrade,
        } = self;
        attacker.walk(c, &format!("{}.attacker", path), "UNCLASSIFIED_STRING");
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        without_suspending.walk(
            c,
            &format!("{}.without_suspending", path),
            "UNCLASSIFIED_STRING",
        );
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        cost_upgrade.walk(c, &format!("{}.cost_upgrade", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::MayDnaDigivolveNowArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MayDnaDigivolveNowArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MayDnaDigivolveNowArgs {
            anchor,
            partner_filter,
            target_filter,
            cost,
            ignore_requirements,
            optional,
            prompt,
        } = self;
        anchor.walk(c, &format!("{}.anchor", path), "UNCLASSIFIED_STRING");
        partner_filter.walk(
            c,
            &format!("{}.partner_filter", path),
            "UNCLASSIFIED_STRING",
        );
        target_filter.walk(c, &format!("{}.target_filter", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RedirectAttackTargetArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RedirectAttackTargetArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RedirectAttackTargetArgs {
            new_target,
            player,
            targets,
            optional,
            prompt,
        } = self;
        new_target.walk(c, &format!("{}.new_target", path), "UNCLASSIFIED_STRING");
        player.walk(c, &format!("{}.player", path), "UNCLASSIFIED_STRING");
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RefireEffectArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RefireEffectArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RefireEffectArgs {
            source,
            timing,
            optional,
            target_attribution,
        } = self;
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        timing.walk(c, &format!("{}.timing", path), "ENGINE_TIMING_NAME");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
            target_attribution.walk(c, &format!("{}.target_attribution", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::step::RefireCardEffectArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RefireCardEffectArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RefireCardEffectArgs { card, timing } = self;
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        timing.walk(c, &format!("{}.timing", path), "ENGINE_TIMING_NAME");
    }
}
impl Walk for digimon_dsl::step::AttackTargetSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AttackTargetSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::AttackTargetSpec::Any => c.node(
                path,
                "digimon_dsl::step::AttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::step::AttackTargetSpec::Player => c.node(
                path,
                "digimon_dsl::step::AttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Player"),
            ),
            digimon_dsl::step::AttackTargetSpec::Digimon => c.node(
                path,
                "digimon_dsl::step::AttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::RedirectReplacementArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RedirectReplacementArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RedirectReplacementArgs { zone } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SubstituteReplacementArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SubstituteReplacementArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SubstituteReplacementArgs { subject } = self;
        subject.walk(c, &format!("{}.subject", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DrawArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DrawArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DrawArgs { of, count } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashFromTopArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashFromTopArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashFromTopArgs { of, count } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::HandleMoveArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::HandleMoveArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::HandleMoveArgs { of, card } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::IndexedMoveArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::IndexedMoveArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::IndexedMoveArgs { of, hand_index } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        hand_index.walk(c, &format!("{}.hand_index", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ReturnToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnToDeckArgs { of, card, position } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ReturnPermanentArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnPermanentArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnPermanentArgs {
            target,
            position,
            include_sources,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        include_sources.walk(
            c,
            &format!("{}.include_sources", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::StackPosition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::StackPosition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::StackPosition::Top => c.node(
                path,
                "digimon_dsl::step::StackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Top"),
            ),
            digimon_dsl::step::StackPosition::Bottom => c.node(
                path,
                "digimon_dsl::step::StackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Bottom"),
            ),
            digimon_dsl::step::StackPosition::Random => c.node(
                path,
                "digimon_dsl::step::StackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Random"),
            ),
            digimon_dsl::step::StackPosition::Choice => c.node(
                path,
                "digimon_dsl::step::StackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Choice"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::SecurityFace {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecurityFace",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::SecurityFace::Up => c.node(
                path,
                "digimon_dsl::step::SecurityFace",
                "ENUM_SCALAR",
                serde_json::json!("Up"),
            ),
            digimon_dsl::step::SecurityFace::Down => c.node(
                path,
                "digimon_dsl::step::SecurityFace",
                "ENUM_SCALAR",
                serde_json::json!("Down"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::RevealArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RevealArgs {
            of,
            count,
            zone,
            bind_as,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::PlaceRemainderArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceRemainderArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceRemainderArgs { of, position } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::RevealSearchArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealSearchArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RevealSearchArgs {
            of,
            count,
            buckets,
            remainder,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        buckets.walk(c, &format!("{}.buckets", path), "UNCLASSIFIED_STRING");
        remainder.walk(c, &format!("{}.remainder", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::RevealSearchBucket {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealSearchBucket",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RevealSearchBucket {
            filter,
            to,
            max,
            optional,
            prompt,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        to.walk(c, &format!("{}.to", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RevealSearchDest {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealSearchDest",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::RevealSearchDest::Hand => c.node(
                path,
                "digimon_dsl::step::RevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::step::RevealSearchDest::Trash => c.node(
                path,
                "digimon_dsl::step::RevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::step::RevealSearchDest::Deck => c.node(
                path,
                "digimon_dsl::step::RevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Deck"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::RevealRemainder {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealRemainder",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::RevealRemainder::Top => c.node(
                path,
                "digimon_dsl::step::RevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Top"),
            ),
            digimon_dsl::step::RevealRemainder::Bottom => c.node(
                path,
                "digimon_dsl::step::RevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Bottom"),
            ),
            digimon_dsl::step::RevealRemainder::Choose => c.node(
                path,
                "digimon_dsl::step::RevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Choose"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::ChooseFromRevealArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ChooseFromRevealArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ChooseFromRevealArgs {
            of,
            filter,
            destination,
            bind_as,
            prompt,
            optional,
            prompt_key,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        destination.walk(c, &format!("{}.destination", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RevealDestination {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RevealDestination",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::RevealDestination::Hand => c.node(
                path,
                "digimon_dsl::step::RevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::step::RevealDestination::DeckTop => c.node(
                path,
                "digimon_dsl::step::RevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckTop"),
            ),
            digimon_dsl::step::RevealDestination::DeckBottom => c.node(
                path,
                "digimon_dsl::step::RevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
            digimon_dsl::step::RevealDestination::PlayFree => c.node(
                path,
                "digimon_dsl::step::RevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("PlayFree"),
            ),
            digimon_dsl::step::RevealDestination::BottomSourceOf { target } => {
                c.node(
                    path,
                    "digimon_dsl::step::RevealDestination",
                    "VARIANT",
                    serde_json::json!("BottomSourceOf"),
                );
                target.walk(
                    c,
                    &format!("{}.target", &format!("{}@BottomSourceOf", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::step::OrderRemainderArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::OrderRemainderArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::OrderRemainderArgs {
            of,
            destinations,
            prompt,
            prompt_key,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        destinations.walk(c, &format!("{}.destinations", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RemainderDestination {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RemainderDestination",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::RemainderDestination::DeckTop => c.node(
                path,
                "digimon_dsl::step::RemainderDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckTop"),
            ),
            digimon_dsl::step::RemainderDestination::DeckBottom => c.node(
                path,
                "digimon_dsl::step::RemainderDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::DeDigivolveArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeDigivolveArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DeDigivolveArgs {
            target,
            amount,
            stop_at_level,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        amount.walk(c, &format!("{}.amount", path), "UNCLASSIFIED_STRING");
        stop_at_level.walk(c, &format!("{}.stop_at_level", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::FormulaStepArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::FormulaStepArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::FormulaStepArgs { formula } = self;
        formula.walk(c, &format!("{}.formula", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlaceOnSecurityArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceOnSecurityArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceOnSecurityArgs {
            source,
            of,
            position,
            face_up,
            disposition,
            include_sources,
            to_owner,
            bind_placed_as,
        } = self;
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        face_up.walk(c, &format!("{}.face_up", path), "UNCLASSIFIED_STRING");
        disposition.walk(c, &format!("{}.disposition", path), "UNCLASSIFIED_STRING");
        include_sources.walk(
            c,
            &format!("{}.include_sources", path),
            "UNCLASSIFIED_STRING",
        );
        to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING");
        bind_placed_as.walk(
            c,
            &format!("{}.bind_placed_as", path),
            "BINDING_DEFINITION",
        );
    }
}
impl Walk for digimon_dsl::step::SecuritySource {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecuritySource",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::SecuritySource::Card { card } => {
                c.node(
                    path,
                    "digimon_dsl::step::SecuritySource",
                    "VARIANT",
                    serde_json::json!("Card"),
                );
                card.walk(
                    c,
                    &format!("{}.card", &format!("{}@Card", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::step::SecuritySource::Permanent { permanent } => {
                c.node(
                    path,
                    "digimon_dsl::step::SecuritySource",
                    "VARIANT",
                    serde_json::json!("Permanent"),
                );
                permanent.walk(
                    c,
                    &format!("{}.permanent", &format!("{}@Permanent", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::step::SecuritySource::Marker(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::SecuritySource",
                    "VARIANT",
                    serde_json::json!("Marker"),
                );
                f0.walk(c, &format!("{}@Marker.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::step::SecuritySelfMarker {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecuritySelfMarker",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::SecuritySelfMarker::SelfPermanent => c.node(
                path,
                "digimon_dsl::step::SecuritySelfMarker",
                "ENUM_SCALAR",
                serde_json::json!("SelfPermanent"),
            ),
            digimon_dsl::step::SecuritySelfMarker::SelfOption => c.node(
                path,
                "digimon_dsl::step::SecuritySelfMarker",
                "ENUM_SCALAR",
                serde_json::json!("SelfOption"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::SecurityReplacementDisposition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecurityReplacementDisposition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::SecurityReplacementDisposition::None => c.node(
                path,
                "digimon_dsl::step::SecurityReplacementDisposition",
                "ENUM_SCALAR",
                serde_json::json!("None"),
            ),
            digimon_dsl::step::SecurityReplacementDisposition::Cancel => c.node(
                path,
                "digimon_dsl::step::SecurityReplacementDisposition",
                "ENUM_SCALAR",
                serde_json::json!("Cancel"),
            ),
            digimon_dsl::step::SecurityReplacementDisposition::Handle => c.node(
                path,
                "digimon_dsl::step::SecurityReplacementDisposition",
                "ENUM_SCALAR",
                serde_json::json!("Handle"),
            ),
            digimon_dsl::step::SecurityReplacementDisposition::Observed => c.node(
                path,
                "digimon_dsl::step::SecurityReplacementDisposition",
                "ENUM_SCALAR",
                serde_json::json!("Observed"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::SecurityPlaceStackedCardArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecurityPlaceStackedCardArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SecurityPlaceStackedCardArgs {
            bind_success_as,
            to_owner,
            carrier,
            source,
            source_index_from_top,
            of,
            position,
            face,
        } = self;
        bind_success_as.walk(c, &format!("{}.bind_success_as", path), "BINDING_DEFINITION");
        to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING");
        carrier.walk(c, &format!("{}.carrier", path), "UNCLASSIFIED_STRING");
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        source_index_from_top.walk(
            c,
            &format!("{}.source_index_from_top", path),
            "UNCLASSIFIED_STRING",
        );
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        face.walk(c, &format!("{}.face", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SecurityPlaceTopStackedCardArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SecurityPlaceTopStackedCardArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SecurityPlaceTopStackedCardArgs {
            bind_success_as,
            to_owner,
            carrier,
            of,
            position,
            face,
        } = self;
        bind_success_as.walk(c, &format!("{}.bind_success_as", path), "BINDING_DEFINITION");
        to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING");
        carrier.walk(c, &format!("{}.carrier", path), "UNCLASSIFIED_STRING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        face.walk(c, &format!("{}.face", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashTopNDigivolutionCardsOfEachArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashTopNDigivolutionCardsOfEachArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashTopNDigivolutionCardsOfEachArgs { of, n } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashOpponentHandToCountArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashOpponentHandToCountArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashOpponentHandToCountArgs {
            opponent,
            target_count,
            bind_count_as,
        } = self;
        opponent.walk(c, &format!("{}.opponent", path), "UNCLASSIFIED_STRING");
        target_count.walk(c, &format!("{}.target_count", path), "UNCLASSIFIED_STRING");
        bind_count_as.walk(c, &format!("{}.bind_count_as", path), "BINDING_DEFINITION");
    }
}
impl Walk for digimon_dsl::step::SearchOwnSecurityStackArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SearchOwnSecurityStackArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SearchOwnSecurityStackArgs {
            filter,
            prompt,
            bind_as,
            optional,
            on_select,
            on_no_match,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        on_select.walk(c, &format!("{}.on_select", path), "UNCLASSIFIED_STRING");
        on_no_match.walk(c, &format!("{}.on_no_match", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlayTokenArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayTokenArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayTokenArgs {
            controller,
            token_name,
            bind_as,
        } = self;
        controller.walk(c, &format!("{}.controller", path), "UNCLASSIFIED_STRING");
        token_name.walk(c, &format!("{}.token_name", path), "TOKEN_CONSTRUCTION");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::PlaceAsBottomSourceArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceAsBottomSourceArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceAsBottomSourceArgs {
            source,
            target,
            face_down,
            bind_placed_as,
        } = self;
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        face_down.walk(c, &format!("{}.face_down", path), "UNCLASSIFIED_STRING");
        bind_placed_as.walk(
            c,
            &format!("{}.bind_placed_as", path),
            "BINDING_DEFINITION",
        );
    }
}
impl Walk for digimon_dsl::step::PlaceAsTopSourceArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceAsTopSourceArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceAsTopSourceArgs {
            source,
            target,
            face_down,
        } = self;
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        face_down.walk(c, &format!("{}.face_down", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::TrashSelectedSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashSelectedSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashSelectedSourcesArgs { source_refs } = self;
        source_refs.walk(c, &format!("{}.source_refs", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::TrashSelectedMaterialsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashSelectedMaterialsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashSelectedMaterialsArgs { target, cards } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        cards.walk(c, &format!("{}.cards", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SameBy {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SameBy",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::SameBy::Level => c.node(
                path,
                "digimon_dsl::step::SameBy",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::ReturnSelectedSourcesToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnSelectedSourcesToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnSelectedSourcesToDeckArgs {
            source_refs,
            position,
        } = self;
        source_refs.walk(c, &format!("{}.source_refs", path), "EXISTING_INSTANCE_BINDING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlaceSelectedCardUnderTamerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceSelectedCardUnderTamerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceSelectedCardUnderTamerArgs {
            card,
            tamer,
            face_down,
            bind_as,
        } = self;
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        tamer.walk(c, &format!("{}.tamer", path), "UNCLASSIFIED_STRING");
        face_down.walk(c, &format!("{}.face_down", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::PlaceSelectedSourcesUnderTamerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceSelectedSourcesUnderTamerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceSelectedSourcesUnderTamerArgs {
            source_refs,
            tamer,
            bind_count_as,
        } = self;
        source_refs.walk(c, &format!("{}.source_refs", path), "EXISTING_INSTANCE_BINDING");
        tamer.walk(c, &format!("{}.tamer", path), "UNCLASSIFIED_STRING");
        bind_count_as.walk(c, &format!("{}.bind_count_as", path), "BINDING_DEFINITION");
    }
}
impl Walk for digimon_dsl::step::MoveMatchingSourcesUnderTamerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MoveMatchingSourcesUnderTamerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MoveMatchingSourcesUnderTamerArgs {
            from,
            tamer,
            filter,
            bind_count_as,
        } = self;
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        tamer.walk(c, &format!("{}.tamer", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_count_as.walk(c, &format!("{}.bind_count_as", path), "BINDING_DEFINITION");
    }
}
impl Walk for digimon_dsl::step::TrashTopStackedSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::TrashTopStackedSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::TrashTopStackedSourcesArgs {
            target,
            count,
            include_top_card,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        include_top_card.walk(
            c,
            &format!("{}.include_top_card", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::PlayFromHandArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayFromHandArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayFromHandArgs {
            of,
            hand_index,
            cost_delta,
            suppress_on_play,
            suspended,
            bind_as,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        hand_index.walk(c, &format!("{}.hand_index", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        suppress_on_play.walk(
            c,
            &format!("{}.suppress_on_play", path),
            "UNCLASSIFIED_STRING",
        );
        suspended.walk(c, &format!("{}.suspended", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::PlayFromHandFreeArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayFromHandFreeArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayFromHandFreeArgs {
            of,
            hand_index,
            cost_delta,
            bind_as,
            suspended,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        hand_index.walk(c, &format!("{}.hand_index", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        suspended.walk(c, &format!("{}.suspended", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlayOrUseFromHandArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayOrUseFromHandArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayOrUseFromHandArgs {
            of,
            hand_index,
            cost_delta,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        hand_index.walk(c, &format!("{}.hand_index", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlayOrUseFromSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayOrUseFromSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayOrUseFromSourcesArgs { of, card, cost } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlayFromRevealedFreeArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayFromRevealedFreeArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayFromRevealedFreeArgs {
            of,
            card,
            bind_as,
            cost_delta,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::UseOptionFromHandArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UseOptionFromHandArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UseOptionFromHandArgs {
            of,
            filter,
            use_cost_lte_opponent_memory,
            optional,
            prompt,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        use_cost_lte_opponent_memory.walk(
            c,
            &format!("{}.use_cost_lte_opponent_memory", path),
            "UNCLASSIFIED_STRING",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::UseOptionFromTrashArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UseOptionFromTrashArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UseOptionFromTrashArgs {
            of,
            filter,
            cost,
            optional,
            prompt,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::UseOptionFromRevealedArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UseOptionFromRevealedArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UseOptionFromRevealedArgs { of, card, cost } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::UseOptionFromSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UseOptionFromSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UseOptionFromSourcesArgs { of, card, cost } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::UseOptionBoundArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UseOptionBoundArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UseOptionBoundArgs { binding, cost } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::CostDelta {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::CostDelta",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::CostDelta::Reduce { reduce } => {
                c.node(
                    path,
                    "digimon_dsl::step::CostDelta",
                    "VARIANT",
                    serde_json::json!("Reduce"),
                );
                reduce.walk(
                    c,
                    &format!("{}.reduce", &format!("{}@Reduce", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::step::CostDelta::Keyword(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::CostDelta",
                    "VARIANT",
                    serde_json::json!("Keyword"),
                );
                f0.walk(c, &format!("{}@Keyword.0", path), role);
            }
            digimon_dsl::step::CostDelta::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::CostDelta",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::step::CostDeltaKeyword {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::CostDeltaKeyword",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::CostDeltaKeyword::Free => c.node(
                path,
                "digimon_dsl::step::CostDeltaKeyword",
                "ENUM_SCALAR",
                serde_json::json!("Free"),
            ),
            digimon_dsl::step::CostDeltaKeyword::Printed => c.node(
                path,
                "digimon_dsl::step::CostDeltaKeyword",
                "ENUM_SCALAR",
                serde_json::json!("Printed"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::PlayUnionBoundFreeArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayUnionBoundFreeArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayUnionBoundFreeArgs {
            binding,
            bind_as,
            suppress_on_play,
            cost_delta,
        } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        suppress_on_play.walk(
            c,
            &format!("{}.suppress_on_play", path),
            "UNCLASSIFIED_STRING",
        );
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::UnionBoundArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UnionBoundArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UnionBoundArgs { binding } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::ReturnUnionBoundToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnUnionBoundToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnUnionBoundToDeckArgs { binding, position } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlayUnderTamerSourceArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayUnderTamerSourceArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayUnderTamerSourceArgs {
            source_refs,
            cost_delta,
            bind_as,
        } = self;
        source_refs.walk(c, &format!("{}.source_refs", path), "EXISTING_INSTANCE_BINDING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::PlayFromMaterialsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayFromMaterialsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayFromMaterialsArgs {
            target,
            source_index,
            cost_delta,
            suppress_on_play,
            bind_as,
            suspended,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        source_index.walk(c, &format!("{}.source_index", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        suppress_on_play.walk(
            c,
            &format!("{}.suppress_on_play", path),
            "UNCLASSIFIED_STRING",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
            suspended.walk(c, &format!("{}.suspended", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::step::PlayFromSecurityArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayFromSecurityArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayFromSecurityArgs { of } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::EffectDigivolveArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectDigivolveArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::EffectDigivolveArgs {
            target,
            from_hand,
            source,
            cost,
            ignore_requirements,
            ignore_level,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        from_hand.walk(c, &format!("{}.from_hand", path), "UNCLASSIFIED_STRING");
        source.walk(c, &format!("{}.source", path), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_level.walk(c, &format!("{}.ignore_level", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::AppFuseZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AppFuseZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::AppFuseZone::Hand => c.node(
                path,
                "digimon_dsl::step::AppFuseZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::step::AppFuseZone::Trash => c.node(
                path,
                "digimon_dsl::step::AppFuseZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::AppFuseArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AppFuseArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AppFuseArgs {
            from,
            result_filter,
            optional,
            bind_as,
        } = self;
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        result_filter.walk(c, &format!("{}.result_filter", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::EffectDnaDigivolveArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectDnaDigivolveArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::EffectDnaDigivolveArgs {
            target_a,
            target_b,
            from_hand,
            cost,
            ignore_requirements,
        } = self;
        target_a.walk(c, &format!("{}.target_a", path), "UNCLASSIFIED_STRING");
        target_b.walk(c, &format!("{}.target_b", path), "UNCLASSIFIED_STRING");
        from_hand.walk(c, &format!("{}.from_hand", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::EffectDnaDigivolveHandPartnerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectDnaDigivolveHandPartnerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::EffectDnaDigivolveHandPartnerArgs {
            target,
            hand_partner,
            from_hand,
            cost,
            ignore_requirements,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        hand_partner.walk(c, &format!("{}.hand_partner", path), "UNCLASSIFIED_STRING");
        from_hand.walk(c, &format!("{}.from_hand", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::EffectDnaDigivolveTrashPartnerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectDnaDigivolveTrashPartnerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::EffectDnaDigivolveTrashPartnerArgs {
            target,
            trash_partner,
            from_hand,
            cost,
            ignore_requirements,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        trash_partner.walk(c, &format!("{}.trash_partner", path), "UNCLASSIFIED_STRING");
        from_hand.walk(c, &format!("{}.from_hand", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::MarkSecurityArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::MarkSecurityArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::MarkSecurityArgs { of, card } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::AddDpModifierArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AddDpModifierArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AddDpModifierArgs {
            target,
            value,
            expiry,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
    }
}
impl Walk for digimon_dsl::step::AddModifierArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AddModifierArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AddModifierArgs {
            opponent_only,
            target,
            modifier,
            value,
            expiry,
            synth_identity,
            payload,
            continuous,
        } = self;
        opponent_only.walk(c, &format!("{}.opponent_only", path), "UNCLASSIFIED_STRING");
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        modifier.walk(c, &format!("{}.modifier", path), "ENGINE_MODIFIER_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
        synth_identity.walk(
            c,
            &format!("{}.synth_identity", path),
            "UNCLASSIFIED_STRING",
        );
        payload.walk(c, &format!("{}.payload", path), "UNCLASSIFIED_STRING");
        continuous.walk(c, &format!("{}.continuous", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SynthIdentitySpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SynthIdentitySpec",
            "EXISTING_INSTANCE_SYNTH_IDENTITY",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SynthIdentitySpec {
            dp,
            kind,
            level,
            colors,
            traits,
        } = self;
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
    }
}
impl Walk for digimon_dsl::step::ModifierPayloadSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ModifierPayloadSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ModifierPayloadSpec {
            name,
            colors,
            dp,
            traits,
            replace_traits,
        } = self;
        name.walk(c, &format!("{}.name", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        replace_traits.walk(
            c,
            &format!("{}.replace_traits", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
    }
}
impl Walk for digimon_dsl::step::AddPlayerModifierArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AddPlayerModifierArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AddPlayerModifierArgs {
            target_player,
            modifier,
            value,
            expiry,
        } = self;
        target_player.walk(c, &format!("{}.target_player", path), "UNCLASSIFIED_STRING");
        modifier.walk(c, &format!("{}.modifier", path), "ENGINE_MODIFIER_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
    }
}
impl Walk for digimon_dsl::step::GrantKeywordArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::GrantKeywordArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::GrantKeywordArgs {
            target,
            keyword,
            expiry,
            value,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        keyword.walk(c, &format!("{}.keyword", path), "ENGINE_KEYWORD_NAME");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::GrantKeywordContinuousArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::GrantKeywordContinuousArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::GrantKeywordContinuousArgs {
            targets,
            keyword,
            expiry,
            value,
        } = self;
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        keyword.walk(c, &format!("{}.keyword", path), "ENGINE_KEYWORD_NAME");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::AllowDigixrosMaterialZoneArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AllowDigixrosMaterialZoneArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AllowDigixrosMaterialZoneArgs { zone, max_count } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        max_count.walk(c, &format!("{}.max_count", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DigixrosCostDeltaArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DigixrosCostDeltaArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DigixrosCostDeltaArgs { delta } = self;
        delta.walk(c, &format!("{}.delta", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PreattachDigixrosMaterialArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PreattachDigixrosMaterialArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PreattachDigixrosMaterialArgs { card, cost_delta } = self;
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DigixrosWildcardArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DigixrosWildcardArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DigixrosWildcardArgs { card, zone } = self;
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::GrantTriggeredEffectArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::GrantTriggeredEffectArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::GrantTriggeredEffectArgs {
            target,
            timing,
            expiry,
            body,
            trigger_if,
            dynamic,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        timing.walk(c, &format!("{}.timing", path), "ENGINE_TIMING_NAME");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
        trigger_if.walk(c, &format!("{}.trigger_if", path), "UNCLASSIFIED_STRING");
            dynamic.walk(c, &format!("{}.dynamic", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::step::EffectSourceKindSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectSourceKindSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::EffectSourceKindSpec::Digimon => c.node(
                path,
                "digimon_dsl::step::EffectSourceKindSpec",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
            digimon_dsl::step::EffectSourceKindSpec::Tamer => c.node(
                path,
                "digimon_dsl::step::EffectSourceKindSpec",
                "ENUM_SCALAR",
                serde_json::json!("Tamer"),
            ),
            digimon_dsl::step::EffectSourceKindSpec::Option => c.node(
                path,
                "digimon_dsl::step::EffectSourceKindSpec",
                "ENUM_SCALAR",
                serde_json::json!("Option"),
            ),
            digimon_dsl::step::EffectSourceKindSpec::Rule => c.node(
                path,
                "digimon_dsl::step::EffectSourceKindSpec",
                "ENUM_SCALAR",
                serde_json::json!("Rule"),
            ),
            digimon_dsl::step::EffectSourceKindSpec::DigiEgg => c.node(
                path, "digimon_dsl::step::EffectSourceKindSpec", "ENUM_SCALAR",
                serde_json::json!("DigiEgg"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::EffectControllerSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::EffectControllerSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::EffectControllerSpec::Any => c.node(
                path,
                "digimon_dsl::step::EffectControllerSpec",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::step::EffectControllerSpec::Opponent => c.node(
                path,
                "digimon_dsl::step::EffectControllerSpec",
                "ENUM_SCALAR",
                serde_json::json!("Opponent"),
            ),
            digimon_dsl::step::EffectControllerSpec::Own => c.node(
                path,
                "digimon_dsl::step::EffectControllerSpec",
                "ENUM_SCALAR",
                serde_json::json!("Own"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::GrantEffectImmunityArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::GrantEffectImmunityArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::GrantEffectImmunityArgs {
            target,
            source_kind,
            source_controller,
            exclude_self,
            expiry,
            continuous,
            targets,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        source_kind.walk(c, &format!("{}.source_kind", path), "UNCLASSIFIED_STRING");
        source_controller.walk(
            c,
            &format!("{}.source_controller", path),
            "UNCLASSIFIED_STRING",
        );
        exclude_self.walk(c, &format!("{}.exclude_self", path), "UNCLASSIFIED_STRING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
        continuous.walk(c, &format!("{}.continuous", path), "UNCLASSIFIED_STRING");
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::GrantNarrowOpponentEffectProtectionArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::GrantNarrowOpponentEffectProtectionArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::GrantNarrowOpponentEffectProtectionArgs { target, expiry } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
    }
}
impl Walk for digimon_dsl::step::FieldSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::FieldSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::FieldSelector::LowestDp => c.node(
                path,
                "digimon_dsl::step::FieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestDp"),
            ),
            digimon_dsl::step::FieldSelector::HighestDp => c.node(
                path,
                "digimon_dsl::step::FieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestDp"),
            ),
            digimon_dsl::step::FieldSelector::LowestPlayCost => c.node(
                path,
                "digimon_dsl::step::FieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestPlayCost"),
            ),
            digimon_dsl::step::FieldSelector::HighestPlayCost => c.node(
                path,
                "digimon_dsl::step::FieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestPlayCost"),
            ),
            digimon_dsl::step::FieldSelector::LowestMaterialCount => c.node(
                path,
                "digimon_dsl::step::FieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestMaterialCount"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::SelectFieldArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectFieldArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectFieldArgs {
            filter,
            bind_as,
            selector,
            prompt,
            optional,
            continue_on_decline,
            prompt_key,
            then,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        continue_on_decline.walk(
            c,
            &format!("{}.continue_on_decline", path),
            "UNCLASSIFIED_STRING",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectDnaPairArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectDnaPairArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectDnaPairArgs {
            left_filter,
            right_filter,
            bind_left_as,
            bind_right_as,
            prompt,
            prompt_key,
            optional,
        } = self;
        left_filter.walk(c, &format!("{}.left_filter", path), "UNCLASSIFIED_STRING");
        right_filter.walk(c, &format!("{}.right_filter", path), "UNCLASSIFIED_STRING");
        bind_left_as.walk(c, &format!("{}.bind_left_as", path), "UNCLASSIFIED_STRING");
        bind_right_as.walk(c, &format!("{}.bind_right_as", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectZoneArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectZoneArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectZoneArgs {
            continue_on_decline,
            of,
            filter,
            bind_as,
            prompt,
            optional,
            cost,
            prompt_key,
            then,
        } = self;
        continue_on_decline.walk(c, &format!("{}.continue_on_decline", path), "UNCLASSIFIED_STRING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectRevealBucketsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectRevealBucketsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectRevealBucketsArgs {
            from,
            buckets,
            no_duplicate_cards,
            prompt,
        } = self;
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        buckets.walk(c, &format!("{}.buckets", path), "UNCLASSIFIED_STRING");
        no_duplicate_cards.walk(
            c,
            &format!("{}.no_duplicate_cards", path),
            "UNCLASSIFIED_STRING",
        );
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::SelectRevealBucketArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectRevealBucketArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectRevealBucketArgs {
            bind_as,
            filter,
            min,
            max,
        } = self;
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectMaterialArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectMaterialArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectMaterialArgs {
            of_permanent,
            filter,
            bind_as,
            prompt,
            optional,
            prompt_key,
        } = self;
        of_permanent.walk(
            c,
            &format!("{}.of_permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::SelectMaterialsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectMaterialsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectMaterialsArgs {
            of_permanent,
            max,
            filter,
            uniqueness,
            min,
            same_by,
            bind_as,
            prompt,
            optional_zero,
            prompt_key,
            named_groups,
            require_max,
        } = self;
        of_permanent.walk(
            c,
            &format!("{}.of_permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        uniqueness.walk(c, &format!("{}.uniqueness", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        same_by.walk(c, &format!("{}.same_by", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional_zero.walk(c, &format!("{}.optional_zero", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
            named_groups.walk(c, &format!("{}.named_groups", path), "MATCH_NAME_EXACT");
        require_max.walk(c, &format!("{}.require_max", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::step::SelectOwnSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectOwnSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectOwnSourcesArgs {
            target,
            from,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectOpponentSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectOpponentSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectOpponentSourcesArgs {
            target,
            filter,
            min,
            max,
            clamp_to_available,
            bind_as,
            prompt,
            then,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        clamp_to_available.walk(
            c,
            &format!("{}.clamp_to_available", path),
            "UNCLASSIFIED_STRING",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DigiBurstArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DigiBurstArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::DigiBurstArgs {
            count,
            bind_as,
            prompt,
            then,
        } = self;
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectOpponentDpBudgetArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectOpponentDpBudgetArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectOpponentDpBudgetArgs {
            dp_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } = self;
        dp_budget.walk(c, &format!("{}.dp_budget", path), "UNCLASSIFIED_STRING");
        min_picks.walk(c, &format!("{}.min_picks", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ZoneCardZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ZoneCardZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::ZoneCardZone::Hand => c.node(
                path,
                "digimon_dsl::step::ZoneCardZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::step::ZoneCardZone::Trash => c.node(
                path,
                "digimon_dsl::step::ZoneCardZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::step::ZoneCardZone::BattleArea => c.node(
                path,
                "digimon_dsl::step::ZoneCardZone",
                "ENUM_SCALAR",
                serde_json::json!("BattleArea"),
            ),
            digimon_dsl::step::ZoneCardZone::LinkCards => c.node(
                path,
                "digimon_dsl::step::ZoneCardZone",
                "ENUM_SCALAR",
                serde_json::json!("LinkCards"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::SelectZoneCardsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectZoneCardsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectZoneCardsArgs {
            of,
            zones,
            filter,
            min,
            max,
            optional_zero,
            distinct_by,
            play_cost_budget,
            exclude,
            bind_as,
            prompt,
            prompt_key,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        zones.walk(c, &format!("{}.zones", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        optional_zero.walk(c, &format!("{}.optional_zero", path), "UNCLASSIFIED_STRING");
        distinct_by.walk(c, &format!("{}.distinct_by", path), "UNCLASSIFIED_STRING");
        play_cost_budget.walk(
            c,
            &format!("{}.play_cost_budget", path),
            "UNCLASSIFIED_STRING",
        );
        exclude.walk(c, &format!("{}.exclude", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::PlayCardsFreeArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlayCardsFreeArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlayCardsFreeArgs { cards } = self;
        cards.walk(c, &format!("{}.cards", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PlaceCardsAsBottomSourcesArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PlaceCardsAsBottomSourcesArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PlaceCardsAsBottomSourcesArgs { cards, target } = self;
        cards.walk(c, &format!("{}.cards", path), "UNCLASSIFIED_STRING");
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::step::ReturnTopStackedToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ReturnTopStackedToDeckArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ReturnTopStackedToDeckArgs {
            targets,
            count,
            position,
            prompt,
            prompt_key,
        } = self;
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::SelectOpponentPlayCostBudgetArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectOpponentPlayCostBudgetArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectOpponentPlayCostBudgetArgs {
            play_cost_budget,
            min_picks,
            filter,
            bind_as,
            prompt,
            then,
        } = self;
        play_cost_budget.walk(
            c,
            &format!("{}.play_cost_budget", path),
            "UNCLASSIFIED_STRING",
        );
        min_picks.walk(c, &format!("{}.min_picks", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectOwnBreedingPermanentArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectOwnBreedingPermanentArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectOwnBreedingPermanentArgs {
            bind_as,
            prompt,
            optional,
            filter,
            then,
        } = self;
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::UnionZoneFilters {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::UnionZoneFilters",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::UnionZoneFilters {
            hand,
            trash,
            material,
        } = self;
        hand.walk(c, &format!("{}.hand", path), "UNCLASSIFIED_STRING");
        trash.walk(c, &format!("{}.trash", path), "UNCLASSIFIED_STRING");
        material.walk(c, &format!("{}.material", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectUnionArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectUnionArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectUnionArgs {
            of,
            zones,
            material_of,
            filter,
            zone_filters,
            material_carrier_filter,
            bind_as,
            prompt,
            optional,
            cost,
            prompt_key,
            then,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        zones.walk(c, &format!("{}.zones", path), "UNCLASSIFIED_STRING");
        material_of.walk(c, &format!("{}.material_of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        zone_filters.walk(c, &format!("{}.zone_filters", path), "UNCLASSIFIED_STRING");
        material_carrier_filter.walk(
            c,
            &format!("{}.material_carrier_filter", path),
            "UNCLASSIFIED_STRING",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::SelectPermutationArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectPermutationArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectPermutationArgs {
            items,
            bind_as,
            prompt,
            prompt_key,
        } = self;
        items.walk(c, &format!("{}.items", path), "EXISTING_INSTANCE_BINDING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::SelectCountCappedArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectCountCappedArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectCountCappedArgs {
            of,
            zone,
            max,
            min,
            clamp_to_available,
            filter,
            bind_as,
            prompt,
            optional_zero,
            distinct_by,
            prompt_key,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        clamp_to_available.walk(
            c,
            &format!("{}.clamp_to_available", path),
            "UNCLASSIFIED_STRING",
        );
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        optional_zero.walk(c, &format!("{}.optional_zero", path), "UNCLASSIFIED_STRING");
        distinct_by.walk(c, &format!("{}.distinct_by", path), "UNCLASSIFIED_STRING");
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::CountBound {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::CountBound",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::CountBound::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::CountBound",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::step::CountBound::Formula { formula } => {
                c.node(
                    path,
                    "digimon_dsl::step::CountBound",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                formula.walk(
                    c,
                    &format!("{}.formula", &format!("{}@Formula", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::step::SelectEffectChoiceArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::SelectEffectChoiceArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::SelectEffectChoiceArgs {
            labels,
            legal_when,
            bind_as,
            prompt,
            prompt_key,
        } = self;
        labels.walk(
            c,
            &format!("{}.labels", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        legal_when.walk(c, &format!("{}.legal_when", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::RepeatEffectChoiceArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RepeatEffectChoiceArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RepeatEffectChoiceArgs {
            count,
            labels,
            bind_as,
            prompt,
            prompt_key,
            body,
        } = self;
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        labels.walk(
            c,
            &format!("{}.labels", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        prompt_key.walk(
            c,
            &format!("{}.prompt_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::AsSelectingPlayerArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::AsSelectingPlayerArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::AsSelectingPlayerArgs { of, body } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::IfStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::IfStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::IfStep {
            condition,
            then,
            else_,
        } = self;
        c.node(
            &format!("{}.condition", path),
            "native_condition",
            "ORIGINAL_VALUE_RETAINED",
            serde_json::to_value(condition).unwrap(),
        );
        match serde_yml::from_value::<digimon_dsl::predicate::PredicateSpec>(condition.clone()) {
            Ok(v) => v.walk(c, &format!("{}.condition", path), role),
            Err(e) => c.unknown(path, "INVALID_TYPED_CONDITION", e.to_string()),
        };
        then.walk(c, &format!("{}.then", path), "UNCLASSIFIED_STRING");
        else_.walk(c, &format!("{}.else_", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ForEachStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ForEachStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ForEachStep {
            over,
            bind_as,
            body,
        } = self;
        over.walk(c, &format!("{}.over", path), "UNCLASSIFIED_STRING");
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::PerSelectedStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::PerSelectedStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::PerSelectedStep {
            selection,
            bind_as,
            body,
        } = self;
        selection.walk(
            c,
            &format!("{}.selection", path),
            "EXISTING_INSTANCE_BINDING",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::ScheduleDelayedStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ScheduleDelayedStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ScheduleDelayedStep { when, body } = self;
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
        body.walk(c, &format!("{}.body", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::DeleteTurnBoundary {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::DeleteTurnBoundary",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::DeleteTurnBoundary::YourTurn => c.node(
                path,
                "digimon_dsl::step::DeleteTurnBoundary",
                "ENUM_SCALAR",
                serde_json::json!("YourTurn"),
            ),
            digimon_dsl::step::DeleteTurnBoundary::OpponentsTurn => c.node(
                path,
                "digimon_dsl::step::DeleteTurnBoundary",
                "ENUM_SCALAR",
                serde_json::json!("OpponentsTurn"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::ScheduleDeletePlayedAtTurnEndArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::ScheduleDeletePlayedAtTurnEndArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::ScheduleDeletePlayedAtTurnEndArgs { binding, at } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        at.walk(c, &format!("{}.at", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::LinkToOwnDigimonArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkToOwnDigimonArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::LinkToOwnDigimonArgs {
            optional,
            free,
            filter,
        } = self;
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        free.walk(c, &format!("{}.free", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::step::LinkCardSourceZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkCardSourceZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::LinkCardSourceZone::Hand => c.node(
                path,
                "digimon_dsl::step::LinkCardSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::step::LinkCardSourceZone::Trash => c.node(
                path,
                "digimon_dsl::step::LinkCardSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::step::LinkCardSourceZone::SelfSources => c.node(
                path,
                "digimon_dsl::step::LinkCardSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("SelfSources"),
            ),
            digimon_dsl::step::LinkCardSourceZone::OwnDigimonSources => c.node(
                path,
                "digimon_dsl::step::LinkCardSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("OwnDigimonSources"),
            ),
            digimon_dsl::step::LinkCardSourceZone::SelfOption => c.node(
                path,
                "digimon_dsl::step::LinkCardSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("SelfOption"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::LinkCardsTo {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkCardsTo",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::LinkCardsTo::SelfPermanent => c.node(
                path,
                "digimon_dsl::step::LinkCardsTo",
                "ENUM_SCALAR",
                serde_json::json!("SelfPermanent"),
            ),
            digimon_dsl::step::LinkCardsTo::OwnDigimon => c.node(
                path,
                "digimon_dsl::step::LinkCardsTo",
                "ENUM_SCALAR",
                serde_json::json!("OwnDigimon"),
            ),
        }
    }
}
impl Walk for digimon_dsl::step::LinkCardsCount {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkCardsCount",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::LinkCardsCount::Exactly { exactly } => {
                c.node(
                    path,
                    "digimon_dsl::step::LinkCardsCount",
                    "VARIANT",
                    serde_json::json!("Exactly"),
                );
                exactly.walk(
                    c,
                    &format!("{}.exactly", &format!("{}@Exactly", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::step::LinkCardsCount::UpTo { up_to } => {
                c.node(
                    path,
                    "digimon_dsl::step::LinkCardsCount",
                    "VARIANT",
                    serde_json::json!("UpTo"),
                );
                up_to.walk(
                    c,
                    &format!("{}.up_to", &format!("{}@UpTo", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::step::LinkCardsCost {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkCardsCost",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::step::LinkCardsCost::Free => c.node(
                path,
                "digimon_dsl::step::LinkCardsCost",
                "ENUM_SCALAR",
                serde_json::json!("Free"),
            ),
            digimon_dsl::step::LinkCardsCost::Reduce(f0) => {
                c.node(
                    path,
                    "digimon_dsl::step::LinkCardsCost",
                    "VARIANT",
                    serde_json::json!("Reduce"),
                );
                f0.walk(c, &format!("{}@Reduce.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::step::LinkCardsArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::LinkCardsArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::LinkCardsArgs {
            from,
            filter,
            to,
            count,
            cost,
            prompt,
            bind_as,
            host_filter,
            exclude_source,
        } = self;
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        to.walk(c, &format!("{}.to", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        host_filter.walk(c, &format!("{}.host_filter", path), "UNCLASSIFIED_STRING");
        exclude_source.walk(
            c,
            &format!("{}.exclude_source", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::step::RelinkSelfToOwnDigimonArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RelinkSelfToOwnDigimonArgs",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RelinkSelfToOwnDigimonArgs {
            host_filter,
            prompt,
        } = self;
        host_filter.walk(c, &format!("{}.host_filter", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::step::OptionalStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::OptionalStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::OptionalStep(f0) = self;
        f0.walk(c, &format!("{}.0", path), role);
    }
}
impl Walk for digimon_dsl::step::RawRustStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::step::RawRustStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::step::RawRustStep {
            fn_name,
            consumes,
            binds,
        } = self;
        fn_name.walk(c, &format!("{}.fn_name", path), "HOOK_STEP");
        consumes.walk(
            c,
            &format!("{}.consumes", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binds.walk(c, &format!("{}.binds", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
digimon_dsl::compiled::CompiledStep::GainMemory(f0) => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GainMemory"));f0.walk(c,&format!("{}@GainMemory.0",path),role);},
digimon_dsl::compiled::CompiledStep::LoseMemory(f0) => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("LoseMemory"));f0.walk(c,&format!("{}@LoseMemory.0",path),role);},
digimon_dsl::compiled::CompiledStep::SetMemory(f0) => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SetMemory"));f0.walk(c,&format!("{}@SetMemory.0",path),role);},
digimon_dsl::compiled::CompiledStep::GainMemoryFn{formula} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GainMemoryFn"));formula.walk(c, &format!("{}.formula", &format!("{}@GainMemoryFn",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::LoseMemoryFn{formula} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("LoseMemoryFn"));formula.walk(c, &format!("{}.formula", &format!("{}@LoseMemoryFn",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::Draw{of,count} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Draw"));of.walk(c, &format!("{}.of", &format!("{}@Draw",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@Draw",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::DrawFn{of,formula} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DrawFn"));of.walk(c, &format!("{}.of", &format!("{}@DrawFn",path)), "UNCLASSIFIED_STRING");formula.walk(c, &format!("{}.formula", &format!("{}@DrawFn",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashFromTop{of,count,count_fn} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashFromTop"));of.walk(c, &format!("{}.of", &format!("{}@TrashFromTop",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@TrashFromTop",path)), "UNCLASSIFIED_STRING");count_fn.walk(c, &format!("{}.count_fn", &format!("{}@TrashFromTop",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddToHandFromDeck{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddToHandFromDeck"));of.walk(c, &format!("{}.of", &format!("{}@AddToHandFromDeck",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@AddToHandFromDeck",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::AddToHandFromTrash{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddToHandFromTrash"));of.walk(c, &format!("{}.of", &format!("{}@AddToHandFromTrash",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@AddToHandFromTrash",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::AddToHandFromSecurity{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddToHandFromSecurity"));of.walk(c, &format!("{}.of", &format!("{}@AddToHandFromSecurity",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@AddToHandFromSecurity",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlaySecurityCard{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaySecurityCard"));of.walk(c, &format!("{}.of", &format!("{}@PlaySecurityCard",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@PlaySecurityCard",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashSelectedSecurity{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashSelectedSecurity"));of.walk(c, &format!("{}.of", &format!("{}@TrashSelectedSecurity",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@TrashSelectedSecurity",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnSelectedSecurityToDeck{of,card,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnSelectedSecurityToDeck"));of.walk(c, &format!("{}.of", &format!("{}@ReturnSelectedSecurityToDeck",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@ReturnSelectedSecurityToDeck",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnSelectedSecurityToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReturnTopSecurityToDeck{of,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnTopSecurityToDeck"));of.walk(c, &format!("{}.of", &format!("{}@ReturnTopSecurityToDeck",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@ReturnTopSecurityToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddTopSecurityToHand{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddTopSecurityToHand"));of.walk(c, &format!("{}.of", &format!("{}@AddTopSecurityToHand",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MayAddTopSecurityToHand{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MayAddTopSecurityToHand"));of.walk(c, &format!("{}.of", &format!("{}@MayAddTopSecurityToHand",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddToHandFromReveal{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddToHandFromReveal"));of.walk(c, &format!("{}.of", &format!("{}@AddToHandFromReveal",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@AddToHandFromReveal",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::AddThisOptionToHand => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("AddThisOptionToHand")),
digimon_dsl::compiled::CompiledStep::TrashFromHandByIndex{of,hand_index} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashFromHandByIndex"));of.walk(c, &format!("{}.of", &format!("{}@TrashFromHandByIndex",path)), "UNCLASSIFIED_STRING");hand_index.walk(c, &format!("{}.hand_index", &format!("{}@TrashFromHandByIndex",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashFromReveal{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashFromReveal"));of.walk(c, &format!("{}.of", &format!("{}@TrashFromReveal",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@TrashFromReveal",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnToDeckFromReveal{of,card,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnToDeckFromReveal"));of.walk(c, &format!("{}.of", &format!("{}@ReturnToDeckFromReveal",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@ReturnToDeckFromReveal",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnToDeckFromReveal",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ShuffleDeck{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ShuffleDeck"));of.walk(c, &format!("{}.of", &format!("{}@ShuffleDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ShuffleSecurity{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ShuffleSecurity"));of.walk(c, &format!("{}.of", &format!("{}@ShuffleSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RevealSecurity{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RevealSecurity"));of.walk(c, &format!("{}@RevealSecurity.of",path), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SecurityPermanentPlacementFinalize{receipt,card,bind_as,source_anchor,source_breeding,rebind} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SecurityPermanentPlacementFinalize"));receipt.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.receipt",path),"EXISTING_INSTANCE_BINDING");card.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.card",path),"EXISTING_INSTANCE_BINDING");bind_as.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.bind_as",path),"EXISTING_INSTANCE_BINDING");source_anchor.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.source_anchor",path),"EXISTING_INSTANCE_BINDING");source_breeding.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.source_breeding",path),"UNCLASSIFIED_STRING");rebind.walk(c,&format!("{}@SecurityPermanentPlacementFinalize.rebind",path),"EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::RevealTopDeck{of,count,zone,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RevealTopDeck"));of.walk(c, &format!("{}.of", &format!("{}@RevealTopDeck",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@RevealTopDeck",path)), "UNCLASSIFIED_STRING");zone.walk(c, &format!("{}.zone", &format!("{}@RevealTopDeck",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@RevealTopDeck",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlaceRemainderOnDeck{of,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceRemainderOnDeck"));of.walk(c, &format!("{}.of", &format!("{}@PlaceRemainderOnDeck",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@PlaceRemainderOnDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RevealSearch{of,count,buckets,remainder} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RevealSearch"));of.walk(c, &format!("{}.of", &format!("{}@RevealSearch",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@RevealSearch",path)), "UNCLASSIFIED_STRING");buckets.walk(c, &format!("{}.buckets", &format!("{}@RevealSearch",path)), "UNCLASSIFIED_STRING");remainder.walk(c, &format!("{}.remainder", &format!("{}@RevealSearch",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ChooseFromReveal{of,filter,destination,bind_as,prompt,prompt_key,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ChooseFromReveal"));of.walk(c, &format!("{}.of", &format!("{}@ChooseFromReveal",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@ChooseFromReveal",path)), "UNCLASSIFIED_STRING");destination.walk(c, &format!("{}.destination", &format!("{}@ChooseFromReveal",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@ChooseFromReveal",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@ChooseFromReveal",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@ChooseFromReveal",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@ChooseFromReveal",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::OrderRemainder{of,destinations,prompt,prompt_key} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("OrderRemainder"));of.walk(c, &format!("{}.of", &format!("{}@OrderRemainder",path)), "UNCLASSIFIED_STRING");destinations.walk(c, &format!("{}.destinations", &format!("{}@OrderRemainder",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@OrderRemainder",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@OrderRemainder",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::DeletePermanent{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeletePermanent"));target.walk(c, &format!("{}.target", &format!("{}@DeletePermanent",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::DeleteForCostReduction{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeleteForCostReduction"));target.walk(c, &format!("{}.target", &format!("{}@DeleteForCostReduction",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::DeleteBoundPermanents{binding} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeleteBoundPermanents"));binding.walk(c, &format!("{}.binding", &format!("{}@DeleteBoundPermanents",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::DeletePermanents{targets} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeletePermanents"));targets.walk(c, &format!("{}.targets", &format!("{}@DeletePermanents",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::DeleteAllPermanents{over,selector} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeleteAllPermanents"));over.walk(c, &format!("{}.over", &format!("{}@DeleteAllPermanents",path)), "UNCLASSIFIED_STRING");selector.walk(c, &format!("{}.selector", &format!("{}@DeleteAllPermanents",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::DeleteOnePerOpponentColor{filter,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeleteOnePerOpponentColor"));filter.walk(c, &format!("{}.filter", &format!("{}@DeleteOnePerOpponentColor",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@DeleteOnePerOpponentColor",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::TrashBreedingPermanent{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashBreedingPermanent"));target.walk(c, &format!("{}.target", &format!("{}@TrashBreedingPermanent",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnToHand{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnToHand"));target.walk(c, &format!("{}.target", &format!("{}@ReturnToHand",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnToDeck{target,position,include_sources} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnToDeck"));target.walk(c, &format!("{}.target", &format!("{}@ReturnToDeck",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnToDeck",path)), "UNCLASSIFIED_STRING");include_sources.walk(c, &format!("{}.include_sources", &format!("{}@ReturnToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::Suspend{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Suspend"));target.walk(c, &format!("{}.target", &format!("{}@Suspend",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::Unsuspend{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Unsuspend"));target.walk(c, &format!("{}.target", &format!("{}@Unsuspend",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::DeDigivolve{target,amount,amount_fn,stop_at_level} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("DeDigivolve"));target.walk(c, &format!("{}.target", &format!("{}@DeDigivolve",path)), "EXISTING_INSTANCE_BINDING");amount.walk(c, &format!("{}.amount", &format!("{}@DeDigivolve",path)), "UNCLASSIFIED_STRING");amount_fn.walk(c, &format!("{}.amount_fn", &format!("{}@DeDigivolve",path)), "UNCLASSIFIED_STRING");stop_at_level.walk(c, &format!("{}.stop_at_level", &format!("{}@DeDigivolve",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceOnSecurity{of,source,position,face_up} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceOnSecurity"));of.walk(c, &format!("{}.of", &format!("{}@PlaceOnSecurity",path)), "UNCLASSIFIED_STRING");source.walk(c, &format!("{}.source", &format!("{}@PlaceOnSecurity",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@PlaceOnSecurity",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlaceOnSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayToken{controller,token_name,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayToken"));controller.walk(c, &format!("{}.controller", &format!("{}@PlayToken",path)), "UNCLASSIFIED_STRING");token_name.walk(c, &format!("{}.token_name", &format!("{}@PlayToken",path)), "TOKEN_CONSTRUCTION");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayToken",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlaceAsBottomSource{source,target,face_down,bind_placed_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceAsBottomSource"));source.walk(c, &format!("{}.source", &format!("{}@PlaceAsBottomSource",path)), "EXISTING_INSTANCE_BINDING");target.walk(c, &format!("{}.target", &format!("{}@PlaceAsBottomSource",path)), "EXISTING_INSTANCE_BINDING");face_down.walk(c, &format!("{}.face_down", &format!("{}@PlaceAsBottomSource",path)), "UNCLASSIFIED_STRING");bind_placed_as.walk(c, &format!("{}.bind_placed_as", &format!("{}@PlaceAsBottomSource",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::PlaceAsTopSource{source,target,face_down} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceAsTopSource"));source.walk(c, &format!("{}.source", &format!("{}@PlaceAsTopSource",path)), "EXISTING_INSTANCE_BINDING");target.walk(c, &format!("{}.target", &format!("{}@PlaceAsTopSource",path)), "EXISTING_INSTANCE_BINDING");face_down.walk(c, &format!("{}.face_down", &format!("{}@PlaceAsTopSource",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceTopSourceAsBottom{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceTopSourceAsBottom"));target.walk(c, &format!("{}.target", &format!("{}@PlaceTopSourceAsBottom",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashTopSource{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashTopSource"));target.walk(c, &format!("{}.target", &format!("{}@TrashTopSource",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashBottomSources{target,count} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashBottomSources"));target.walk(c, &format!("{}.target", &format!("{}@TrashBottomSources",path)), "EXISTING_INSTANCE_BINDING");count.walk(c, &format!("{}.count", &format!("{}@TrashBottomSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashAllSources{target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashAllSources"));target.walk(c, &format!("{}.target", &format!("{}@TrashAllSources",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashBottomFaceDownSourceUnderTamer{of,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashBottomFaceDownSourceUnderTamer"));of.walk(c, &format!("{}.of", &format!("{}@TrashBottomFaceDownSourceUnderTamer",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@TrashBottomFaceDownSourceUnderTamer",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashBottomFaceDownSourcesUnderTamers{of,count} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashBottomFaceDownSourcesUnderTamers"));of.walk(c, &format!("{}.of", &format!("{}@TrashBottomFaceDownSourcesUnderTamers",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@TrashBottomFaceDownSourcesUnderTamers",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashLinkCardOfOwnDigimon{of,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashLinkCardOfOwnDigimon"));of.walk(c, &format!("{}.of", &format!("{}@TrashLinkCardOfOwnDigimon",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@TrashLinkCardOfOwnDigimon",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashOptionFromOwnStacks{of,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashOptionFromOwnStacks"));of.walk(c, &format!("{}.of", &format!("{}@TrashOptionFromOwnStacks",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@TrashOptionFromOwnStacks",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashOptionFromOwnDigivolutionCardsAndCancelLeave => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("TrashOptionFromOwnDigivolutionCardsAndCancelLeave")),
digimon_dsl::compiled::CompiledStep::DeleteSelfAndCancelLeave => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("DeleteSelfAndCancelLeave")),
digimon_dsl::compiled::CompiledStep::PlaceSelectedCardUnderTamer{card,tamer,face_down,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceSelectedCardUnderTamer"));card.walk(c, &format!("{}.card", &format!("{}@PlaceSelectedCardUnderTamer",path)), "EXISTING_INSTANCE_BINDING");tamer.walk(c, &format!("{}.tamer", &format!("{}@PlaceSelectedCardUnderTamer",path)), "UNCLASSIFIED_STRING");face_down.walk(c, &format!("{}.face_down", &format!("{}@PlaceSelectedCardUnderTamer",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlaceSelectedCardUnderTamer",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::MoveSelfOptionUnderPermanent{target,face_down} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MoveSelfOptionUnderPermanent"));target.walk(c, &format!("{}.target", &format!("{}@MoveSelfOptionUnderPermanent",path)), "EXISTING_INSTANCE_BINDING");face_down.walk(c, &format!("{}.face_down", &format!("{}@MoveSelfOptionUnderPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceSelfUnderPermanent{target,face_down} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceSelfUnderPermanent"));target.walk(c, &format!("{}.target", &format!("{}@PlaceSelfUnderPermanent",path)), "EXISTING_INSTANCE_BINDING");face_down.walk(c, &format!("{}.face_down", &format!("{}@PlaceSelfUnderPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceSelectedSourcesUnderTamer{source_refs,tamer,bind_count_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceSelectedSourcesUnderTamer"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@PlaceSelectedSourcesUnderTamer",path)), "EXISTING_INSTANCE_BINDING");tamer.walk(c, &format!("{}.tamer", &format!("{}@PlaceSelectedSourcesUnderTamer",path)), "UNCLASSIFIED_STRING");bind_count_as.walk(c, &format!("{}.bind_count_as", &format!("{}@PlaceSelectedSourcesUnderTamer",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::MoveMatchingSourcesUnderTamer{from,tamer,filter,bind_count_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MoveMatchingSourcesUnderTamer"));from.walk(c, &format!("{}.from", &format!("{}@MoveMatchingSourcesUnderTamer",path)), "EXISTING_INSTANCE_BINDING");tamer.walk(c, &format!("{}.tamer", &format!("{}@MoveMatchingSourcesUnderTamer",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@MoveMatchingSourcesUnderTamer",path)), "UNCLASSIFIED_STRING");bind_count_as.walk(c, &format!("{}.bind_count_as", &format!("{}@MoveMatchingSourcesUnderTamer",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::TrashTopStackedSources{target,count,include_top_card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashTopStackedSources"));target.walk(c, &format!("{}.target", &format!("{}@TrashTopStackedSources",path)), "EXISTING_INSTANCE_BINDING");count.walk(c, &format!("{}.count", &format!("{}@TrashTopStackedSources",path)), "UNCLASSIFIED_STRING");include_top_card.walk(c, &format!("{}.include_top_card", &format!("{}@TrashTopStackedSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::Hatch{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Hatch"));of.walk(c, &format!("{}.of", &format!("{}@Hatch",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MoveFromBreeding{of,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MoveFromBreeding"));of.walk(c, &format!("{}.of", &format!("{}@MoveFromBreeding",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@MoveFromBreeding",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::PlayFromHand{of,hand_index,cost_delta,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromHand"));of.walk(c, &format!("{}.of", &format!("{}@PlayFromHand",path)), "UNCLASSIFIED_STRING");hand_index.walk(c, &format!("{}.hand_index", &format!("{}@PlayFromHand",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayFromHand",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayFromHand",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlayFromHandFree{of,hand_index,bind_as,suspended} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromHandFree"));of.walk(c, &format!("{}.of", &format!("{}@PlayFromHandFree",path)), "UNCLASSIFIED_STRING");hand_index.walk(c, &format!("{}.hand_index", &format!("{}@PlayFromHandFree",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayFromHandFree",path)), "EXISTING_INSTANCE_BINDING");suspended.walk(c, &format!("{}.suspended", &format!("{}@PlayFromHandFree",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::UseOptionFromHand{of,filter,use_cost_lte_opponent_memory,optional,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("UseOptionFromHand"));of.walk(c, &format!("{}.of", &format!("{}@UseOptionFromHand",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@UseOptionFromHand",path)), "UNCLASSIFIED_STRING");use_cost_lte_opponent_memory.walk(c, &format!("{}.use_cost_lte_opponent_memory", &format!("{}@UseOptionFromHand",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@UseOptionFromHand",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@UseOptionFromHand",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::UseOptionFromTrash{of,filter,cost_delta,optional,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("UseOptionFromTrash"));of.walk(c, &format!("{}.of", &format!("{}@UseOptionFromTrash",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@UseOptionFromTrash",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@UseOptionFromTrash",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@UseOptionFromTrash",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@UseOptionFromTrash",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::UseOptionFromRevealed{of,card,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("UseOptionFromRevealed"));of.walk(c, &format!("{}.of", &format!("{}@UseOptionFromRevealed",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@UseOptionFromRevealed",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@UseOptionFromRevealed",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::UseOptionFromSources{of,card,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("UseOptionFromSources"));of.walk(c, &format!("{}.of", &format!("{}@UseOptionFromSources",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@UseOptionFromSources",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@UseOptionFromSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::UseOptionBound{binding,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("UseOptionBound"));binding.walk(c, &format!("{}.binding", &format!("{}@UseOptionBound",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@UseOptionBound",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayOrUseFromHand{of,hand_index,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayOrUseFromHand"));of.walk(c, &format!("{}.of", &format!("{}@PlayOrUseFromHand",path)), "UNCLASSIFIED_STRING");hand_index.walk(c, &format!("{}.hand_index", &format!("{}@PlayOrUseFromHand",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayOrUseFromHand",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayOrUseFromSources{of,card,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayOrUseFromSources"));of.walk(c, &format!("{}.of", &format!("{}@PlayOrUseFromSources",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@PlayOrUseFromSources",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayOrUseFromSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayFromRevealedFree{of,card,bind_as,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromRevealedFree"));of.walk(c, &format!("{}.of", &format!("{}@PlayFromRevealedFree",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@PlayFromRevealedFree",path)), "EXISTING_INSTANCE_BINDING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayFromRevealedFree",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayFromRevealedFree",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayFromTrash{of,trash_index,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromTrash"));of.walk(c, &format!("{}.of", &format!("{}@PlayFromTrash",path)), "UNCLASSIFIED_STRING");trash_index.walk(c, &format!("{}.trash_index", &format!("{}@PlayFromTrash",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayFromTrash",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayFromTrashFree{of,trash_index,suppress_on_play,suspended} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromTrashFree"));of.walk(c, &format!("{}.of", &format!("{}@PlayFromTrashFree",path)), "UNCLASSIFIED_STRING");trash_index.walk(c, &format!("{}.trash_index", &format!("{}@PlayFromTrashFree",path)), "UNCLASSIFIED_STRING");suppress_on_play.walk(c, &format!("{}.suppress_on_play", &format!("{}@PlayFromTrashFree",path)), "UNCLASSIFIED_STRING");suspended.walk(c, &format!("{}.suspended", &format!("{}@PlayFromTrashFree",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayUnionBoundFree{binding,bind_as,suppress_on_play,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayUnionBoundFree"));binding.walk(c, &format!("{}.binding", &format!("{}@PlayUnionBoundFree",path)), "EXISTING_INSTANCE_BINDING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayUnionBoundFree",path)), "EXISTING_INSTANCE_BINDING");suppress_on_play.walk(c, &format!("{}.suppress_on_play", &format!("{}@PlayUnionBoundFree",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayUnionBoundFree",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashUnionBound{binding} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashUnionBound"));binding.walk(c, &format!("{}.binding", &format!("{}@TrashUnionBound",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnUnionBoundToDeck{binding,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnUnionBoundToDeck"));binding.walk(c, &format!("{}.binding", &format!("{}@ReturnUnionBoundToDeck",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnUnionBoundToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlayFromSecurity => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("PlayFromSecurity")),
digimon_dsl::compiled::CompiledStep::PlayFromMaterials{target,source_index,cost_delta,suppress_on_play,bind_as,
            suspended,
        } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayFromMaterials"));target.walk(c, &format!("{}.target", &format!("{}@PlayFromMaterials",path)), "EXISTING_INSTANCE_BINDING");source_index.walk(c, &format!("{}.source_index", &format!("{}@PlayFromMaterials",path)), "UNCLASSIFIED_STRING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayFromMaterials",path)), "UNCLASSIFIED_STRING");suppress_on_play.walk(c, &format!("{}.suppress_on_play", &format!("{}@PlayFromMaterials",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayFromMaterials",path)), "EXISTING_INSTANCE_BINDING");suspended.walk(c, &format!("{}@PlayFromMaterials.suspended", path), "UNCLASSIFIED_STRING");
},
digimon_dsl::compiled::CompiledStep::PlaySelectedSourcesFree{source_refs} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaySelectedSourcesFree"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@PlaySelectedSourcesFree",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlayUnderTamerSource{source_refs,cost_delta,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayUnderTamerSource"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@PlayUnderTamerSource",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PlayUnderTamerSource",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PlayUnderTamerSource",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::EffectInitiatedDigivolve{target,from_hand,cost,ignore_requirements,ignore_level} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("EffectInitiatedDigivolve"));target.walk(c, &format!("{}.target", &format!("{}@EffectInitiatedDigivolve",path)), "EXISTING_INSTANCE_BINDING");from_hand.walk(c, &format!("{}.from_hand", &format!("{}@EffectInitiatedDigivolve",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@EffectInitiatedDigivolve",path)), "UNCLASSIFIED_STRING");ignore_requirements.walk(c, &format!("{}.ignore_requirements", &format!("{}@EffectInitiatedDigivolve",path)), "UNCLASSIFIED_STRING");ignore_level.walk(c, &format!("{}.ignore_level", &format!("{}@EffectInitiatedDigivolve",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AppFuse{from_zone,result_filter,optional,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AppFuse"));from_zone.walk(c, &format!("{}.from_zone", &format!("{}@AppFuse",path)), "UNCLASSIFIED_STRING");result_filter.walk(c, &format!("{}.result_filter", &format!("{}@AppFuse",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@AppFuse",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@AppFuse",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::EffectInitiatedDnaDigivolve{target_a,target_b,from_hand,cost,ignore_requirements} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("EffectInitiatedDnaDigivolve"));target_a.walk(c, &format!("{}.target_a", &format!("{}@EffectInitiatedDnaDigivolve",path)), "UNCLASSIFIED_STRING");target_b.walk(c, &format!("{}.target_b", &format!("{}@EffectInitiatedDnaDigivolve",path)), "UNCLASSIFIED_STRING");from_hand.walk(c, &format!("{}.from_hand", &format!("{}@EffectInitiatedDnaDigivolve",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@EffectInitiatedDnaDigivolve",path)), "UNCLASSIFIED_STRING");ignore_requirements.walk(c, &format!("{}.ignore_requirements", &format!("{}@EffectInitiatedDnaDigivolve",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::EffectInitiatedDnaDigivolveHandPartner{target,hand_partner,from_hand,cost,ignore_requirements} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("EffectInitiatedDnaDigivolveHandPartner"));target.walk(c, &format!("{}.target", &format!("{}@EffectInitiatedDnaDigivolveHandPartner",path)), "EXISTING_INSTANCE_BINDING");hand_partner.walk(c, &format!("{}.hand_partner", &format!("{}@EffectInitiatedDnaDigivolveHandPartner",path)), "UNCLASSIFIED_STRING");from_hand.walk(c, &format!("{}.from_hand", &format!("{}@EffectInitiatedDnaDigivolveHandPartner",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@EffectInitiatedDnaDigivolveHandPartner",path)), "UNCLASSIFIED_STRING");ignore_requirements.walk(c, &format!("{}.ignore_requirements", &format!("{}@EffectInitiatedDnaDigivolveHandPartner",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::EffectInitiatedDnaDigivolveTrashPartner{target,trash_partner,from_hand,cost,ignore_requirements} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("EffectInitiatedDnaDigivolveTrashPartner"));target.walk(c, &format!("{}.target", &format!("{}@EffectInitiatedDnaDigivolveTrashPartner",path)), "EXISTING_INSTANCE_BINDING");trash_partner.walk(c, &format!("{}.trash_partner", &format!("{}@EffectInitiatedDnaDigivolveTrashPartner",path)), "UNCLASSIFIED_STRING");from_hand.walk(c, &format!("{}.from_hand", &format!("{}@EffectInitiatedDnaDigivolveTrashPartner",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@EffectInitiatedDnaDigivolveTrashPartner",path)), "UNCLASSIFIED_STRING");ignore_requirements.walk(c, &format!("{}.ignore_requirements", &format!("{}@EffectInitiatedDnaDigivolveTrashPartner",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashTopSecurity{of,count,leave,bind_count_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashTopSecurity"));of.walk(c, &format!("{}.of", &format!("{}@TrashTopSecurity",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@TrashTopSecurity",path)), "UNCLASSIFIED_STRING");leave.walk(c, &format!("{}.leave", &format!("{}@TrashTopSecurity",path)), "UNCLASSIFIED_STRING");bind_count_as.walk(c, &format!("{}.bind_count_as", &format!("{}@TrashTopSecurity",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::TrashBottomSecurity{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashBottomSecurity"));of.walk(c, &format!("{}.of", &format!("{}@TrashBottomSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddBottomSecurityToHand{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddBottomSecurityToHand"));of.walk(c, &format!("{}.of", &format!("{}@AddBottomSecurityToHand",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashTopSecurityAndCancelReplacement{of,all_matching} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashTopSecurityAndCancelReplacement"));of.walk(c, &format!("{}.of", &format!("{}@TrashTopSecurityAndCancelReplacement",path)), "UNCLASSIFIED_STRING");all_matching.walk(c, &format!("{}.all_matching", &format!("{}@TrashTopSecurityAndCancelReplacement",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::BounceSelf => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("BounceSelf")),
digimon_dsl::compiled::CompiledStep::PlaceSelfAtSecurity{position,face_up} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceSelfAtSecurity"));position.walk(c, &format!("{}.position", &format!("{}@PlaceSelfAtSecurity",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlaceSelfAtSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceSelfOptionAtSecurity{position,face_up} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceSelfOptionAtSecurity"));position.walk(c, &format!("{}.position", &format!("{}@PlaceSelfOptionAtSecurity",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlaceSelfOptionAtSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlacePermanentBottomSecurityAndCancelReplacement{of,target,
            to_owner,
        } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlacePermanentBottomSecurityAndCancelReplacement"));of.walk(c, &format!("{}.of", &format!("{}@PlacePermanentBottomSecurityAndCancelReplacement",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@PlacePermanentBottomSecurityAndCancelReplacement",path)), "EXISTING_INSTANCE_BINDING");to_owner.walk(c, &format!("{}@PlacePermanentBottomSecurityAndCancelReplacement.to_owner", path), "UNCLASSIFIED_STRING");
},
digimon_dsl::compiled::CompiledStep::PlacePermanentOnSecurity{of,target,position,face_up,to_owner,bind_placed_as,result_receipt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlacePermanentOnSecurity"));of.walk(c, &format!("{}.of", &format!("{}@PlacePermanentOnSecurity",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@PlacePermanentOnSecurity",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@PlacePermanentOnSecurity",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlacePermanentOnSecurity",path)), "UNCLASSIFIED_STRING");to_owner.walk(c, &format!("{}.to_owner", &format!("{}@PlacePermanentOnSecurity",path)), "UNCLASSIFIED_STRING");bind_placed_as.walk(c, &format!("{}.bind_placed_as", &format!("{}@PlacePermanentOnSecurity",path)), "BINDING_DEFINITION");result_receipt.walk(c,&format!("{}@PlacePermanentOnSecurity.result_receipt",path),"EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::PlacePermanentOnSecurityAndHandleReplacement{of,target,position,face_up} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlacePermanentOnSecurityAndHandleReplacement"));of.walk(c, &format!("{}.of", &format!("{}@PlacePermanentOnSecurityAndHandleReplacement",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@PlacePermanentOnSecurityAndHandleReplacement",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@PlacePermanentOnSecurityAndHandleReplacement",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlacePermanentOnSecurityAndHandleReplacement",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlacePermanentOnSecurityObserved{of,target,position,face_up,include_sources} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlacePermanentOnSecurityObserved"));of.walk(c, &format!("{}.of", &format!("{}@PlacePermanentOnSecurityObserved",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@PlacePermanentOnSecurityObserved",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@PlacePermanentOnSecurityObserved",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@PlacePermanentOnSecurityObserved",path)), "UNCLASSIFIED_STRING");include_sources.walk(c, &format!("{}.include_sources", &format!("{}@PlacePermanentOnSecurityObserved",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SecurityPlaceStackedCard{bind_success_as,to_owner,carrier,source,source_index_from_top,of,position,face_up} => { bind_success_as.walk(c, &format!("{}.bind_success_as", path), "BINDING_DEFINITION");to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING"); c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SecurityPlaceStackedCard"));carrier.walk(c, &format!("{}.carrier", &format!("{}@SecurityPlaceStackedCard",path)), "UNCLASSIFIED_STRING");source.walk(c, &format!("{}.source", &format!("{}@SecurityPlaceStackedCard",path)), "EXISTING_INSTANCE_BINDING");source_index_from_top.walk(c, &format!("{}.source_index_from_top", &format!("{}@SecurityPlaceStackedCard",path)), "UNCLASSIFIED_STRING");of.walk(c, &format!("{}.of", &format!("{}@SecurityPlaceStackedCard",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@SecurityPlaceStackedCard",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@SecurityPlaceStackedCard",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SecurityPlaceTopStackedCard{carrier,of,position,face_up,bind_success_as,to_owner} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SecurityPlaceTopStackedCard"));bind_success_as.walk(c, &format!("{}.bind_success_as", path), "BINDING_DEFINITION");to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING");carrier.walk(c, &format!("{}.carrier", &format!("{}@SecurityPlaceTopStackedCard",path)), "UNCLASSIFIED_STRING");of.walk(c, &format!("{}.of", &format!("{}@SecurityPlaceTopStackedCard",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@SecurityPlaceTopStackedCard",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@SecurityPlaceTopStackedCard",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SecurityPlaceTopCard{carrier,of,position,face_up,bind_success_as,to_owner} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SecurityPlaceTopCard"));bind_success_as.walk(c, &format!("{}.bind_success_as", path), "BINDING_DEFINITION");to_owner.walk(c, &format!("{}.to_owner", path), "UNCLASSIFIED_STRING");carrier.walk(c, &format!("{}.carrier", &format!("{}@SecurityPlaceTopCard",path)), "UNCLASSIFIED_STRING");of.walk(c, &format!("{}.of", &format!("{}@SecurityPlaceTopCard",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@SecurityPlaceTopCard",path)), "UNCLASSIFIED_STRING");face_up.walk(c, &format!("{}.face_up", &format!("{}@SecurityPlaceTopCard",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReturnAllTrashToDeckBottom{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnAllTrashToDeckBottom"));of.walk(c, &format!("{}.of", &format!("{}@ReturnAllTrashToDeckBottom",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReturnTrashListToDeckBottom{of,cards,to_top} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnTrashListToDeckBottom"));of.walk(c, &format!("{}.of", &format!("{}@ReturnTrashListToDeckBottom",path)), "UNCLASSIFIED_STRING");cards.walk(c, &format!("{}.cards", &format!("{}@ReturnTrashListToDeckBottom",path)), "UNCLASSIFIED_STRING");to_top.walk(c, &format!("{}.to_top", &format!("{}@ReturnTrashListToDeckBottom",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MoveTrashCardToDeckTop{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MoveTrashCardToDeckTop"));of.walk(c, &format!("{}.of", &format!("{}@MoveTrashCardToDeckTop",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@MoveTrashCardToDeckTop",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashTopNDigivolutionCardsOfEach{of,n} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashTopNDigivolutionCardsOfEach"));of.walk(c, &format!("{}.of", &format!("{}@TrashTopNDigivolutionCardsOfEach",path)), "UNCLASSIFIED_STRING");n.walk(c, &format!("{}.n", &format!("{}@TrashTopNDigivolutionCardsOfEach",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashOpponentHandToCount{opponent,target_count,bind_count_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashOpponentHandToCount"));opponent.walk(c, &format!("{}.opponent", &format!("{}@TrashOpponentHandToCount",path)), "UNCLASSIFIED_STRING");target_count.walk(c, &format!("{}.target_count", &format!("{}@TrashOpponentHandToCount",path)), "UNCLASSIFIED_STRING");bind_count_as.walk(c, &format!("{}.bind_count_as", &format!("{}@TrashOpponentHandToCount",path)), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::SearchOwnSecurityStack{filter,prompt,bind_as,optional,on_select,on_no_match} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SearchOwnSecurityStack"));filter.walk(c, &format!("{}.filter", &format!("{}@SearchOwnSecurityStack",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SearchOwnSecurityStack",path)), "METADATA_NO_EXECUTABLE_REFERENCE");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SearchOwnSecurityStack",path)), "EXISTING_INSTANCE_BINDING");optional.walk(c, &format!("{}.optional", &format!("{}@SearchOwnSecurityStack",path)), "UNCLASSIFIED_STRING");on_select.walk(c, &format!("{}.on_select", &format!("{}@SearchOwnSecurityStack",path)), "UNCLASSIFIED_STRING");on_no_match.walk(c, &format!("{}.on_no_match", &format!("{}@SearchOwnSecurityStack",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::Recover{of,count} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Recover"));of.walk(c, &format!("{}.of", &format!("{}@Recover",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@Recover",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MarkSecurityFaceUp{of,card} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MarkSecurityFaceUp"));of.walk(c, &format!("{}.of", &format!("{}@MarkSecurityFaceUp",path)), "UNCLASSIFIED_STRING");card.walk(c, &format!("{}.card", &format!("{}@MarkSecurityFaceUp",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::FlipSecurityFaceUp{of} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("FlipSecurityFaceUp"));of.walk(c, &format!("{}.of", &format!("{}@FlipSecurityFaceUp",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddDpModifier{target,value,expiry} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddDpModifier"));target.walk(c, &format!("{}.target", &format!("{}@AddDpModifier",path)), "EXISTING_INSTANCE_BINDING");value.walk(c, &format!("{}.value", &format!("{}@AddDpModifier",path)), "UNCLASSIFIED_STRING");expiry.walk(c, &format!("{}.expiry", &format!("{}@AddDpModifier",path)), "ENGINE_EXPIRY_NAME");},
digimon_dsl::compiled::CompiledStep::AddModifier{opponent_only,target,modifier,value,expiry,synth_identity,payload,continuous} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddModifier"));opponent_only.walk(c, &format!("{}.opponent_only", &format!("{}@AddModifier",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@AddModifier",path)), "EXISTING_INSTANCE_BINDING");modifier.walk(c, &format!("{}.modifier", &format!("{}@AddModifier",path)), "ENGINE_MODIFIER_NAME");value.walk(c, &format!("{}.value", &format!("{}@AddModifier",path)), "UNCLASSIFIED_STRING");expiry.walk(c, &format!("{}.expiry", &format!("{}@AddModifier",path)), "ENGINE_EXPIRY_NAME");synth_identity.walk(c, &format!("{}.synth_identity", &format!("{}@AddModifier",path)), "UNCLASSIFIED_STRING");payload.walk(c, &format!("{}.payload", &format!("{}@AddModifier",path)), "UNCLASSIFIED_STRING");continuous.walk(c, &format!("{}.continuous", &format!("{}@AddModifier",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddPlayerModifier{target_player,modifier,value,expiry} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddPlayerModifier"));target_player.walk(c, &format!("{}.target_player", &format!("{}@AddPlayerModifier",path)), "UNCLASSIFIED_STRING");modifier.walk(c, &format!("{}.modifier", &format!("{}@AddPlayerModifier",path)), "ENGINE_MODIFIER_NAME");value.walk(c, &format!("{}.value", &format!("{}@AddPlayerModifier",path)), "UNCLASSIFIED_STRING");expiry.walk(c, &format!("{}.expiry", &format!("{}@AddPlayerModifier",path)), "ENGINE_EXPIRY_NAME");},
digimon_dsl::compiled::CompiledStep::GrantKeyword{target,keyword,expiry,value} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantKeyword"));target.walk(c, &format!("{}.target", &format!("{}@GrantKeyword",path)), "EXISTING_INSTANCE_BINDING");keyword.walk(c, &format!("{}.keyword", &format!("{}@GrantKeyword",path)), "ENGINE_KEYWORD_NAME");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantKeyword",path)), "ENGINE_EXPIRY_NAME");value.walk(c, &format!("{}.value", &format!("{}@GrantKeyword",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::GrantKeywordContinuous{targets,keyword,expiry,value} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantKeywordContinuous"));targets.walk(c, &format!("{}.targets", &format!("{}@GrantKeywordContinuous",path)), "UNCLASSIFIED_STRING");keyword.walk(c, &format!("{}.keyword", &format!("{}@GrantKeywordContinuous",path)), "ENGINE_KEYWORD_NAME");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantKeywordContinuous",path)), "ENGINE_EXPIRY_NAME");value.walk(c, &format!("{}.value", &format!("{}@GrantKeywordContinuous",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AllowDigixrosMaterialZone{zone,max_count} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AllowDigixrosMaterialZone"));zone.walk(c, &format!("{}.zone", &format!("{}@AllowDigixrosMaterialZone",path)), "UNCLASSIFIED_STRING");max_count.walk(c, &format!("{}.max_count", &format!("{}@AllowDigixrosMaterialZone",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddDigixrosCostDelta{delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddDigixrosCostDelta"));delta.walk(c, &format!("{}.delta", &format!("{}@AddDigixrosCostDelta",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PreattachDigixrosMaterial{card,cost_delta} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PreattachDigixrosMaterial"));card.walk(c, &format!("{}.card", &format!("{}@PreattachDigixrosMaterial",path)), "EXISTING_INSTANCE_BINDING");cost_delta.walk(c, &format!("{}.cost_delta", &format!("{}@PreattachDigixrosMaterial",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RegisterDigixrosWildcardForTurn{card,zone} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RegisterDigixrosWildcardForTurn"));card.walk(c, &format!("{}.card", &format!("{}@RegisterDigixrosWildcardForTurn",path)), "EXISTING_INSTANCE_BINDING");zone.walk(c, &format!("{}.zone", &format!("{}@RegisterDigixrosWildcardForTurn",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AddDigixrosWildcardToPendingTransaction{card,zone} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AddDigixrosWildcardToPendingTransaction"));card.walk(c, &format!("{}.card", &format!("{}@AddDigixrosWildcardToPendingTransaction",path)), "EXISTING_INSTANCE_BINDING");zone.walk(c, &format!("{}.zone", &format!("{}@AddDigixrosWildcardToPendingTransaction",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::GrantEffectImmunity{target,source_kind,source_controller,expiry,continuous,targets} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantEffectImmunity"));target.walk(c, &format!("{}.target", &format!("{}@GrantEffectImmunity",path)), "EXISTING_INSTANCE_BINDING");source_kind.walk(c, &format!("{}.source_kind", &format!("{}@GrantEffectImmunity",path)), "UNCLASSIFIED_STRING");source_controller.walk(c, &format!("{}.source_controller", &format!("{}@GrantEffectImmunity",path)), "UNCLASSIFIED_STRING");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantEffectImmunity",path)), "ENGINE_EXPIRY_NAME");continuous.walk(c, &format!("{}.continuous", &format!("{}@GrantEffectImmunity",path)), "UNCLASSIFIED_STRING");targets.walk(c, &format!("{}.targets", &format!("{}@GrantEffectImmunity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::GrantNarrowOpponentEffectProtection{target,expiry} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantNarrowOpponentEffectProtection"));target.walk(c, &format!("{}.target", &format!("{}@GrantNarrowOpponentEffectProtection",path)), "EXISTING_INSTANCE_BINDING");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantNarrowOpponentEffectProtection",path)), "ENGINE_EXPIRY_NAME");},
digimon_dsl::compiled::CompiledStep::GrantStackProtectionFromOpponentEffects{target,expiry} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantStackProtectionFromOpponentEffects"));target.walk(c, &format!("{}.target", &format!("{}@GrantStackProtectionFromOpponentEffects",path)), "EXISTING_INSTANCE_BINDING");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantStackProtectionFromOpponentEffects",path)), "ENGINE_EXPIRY_NAME");},
digimon_dsl::compiled::CompiledStep::GrantStackTrashImmunityFromOpponentEffects{target,expiry} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantStackTrashImmunityFromOpponentEffects"));target.walk(c, &format!("{}.target", &format!("{}@GrantStackTrashImmunityFromOpponentEffects",path)), "EXISTING_INSTANCE_BINDING");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantStackTrashImmunityFromOpponentEffects",path)), "ENGINE_EXPIRY_NAME");},
digimon_dsl::compiled::CompiledStep::GrantTriggeredEffect{target,timing,expiry,body,
            dynamic,
            trigger_if,
        } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("GrantTriggeredEffect"));target.walk(c, &format!("{}.target", &format!("{}@GrantTriggeredEffect",path)), "EXISTING_INSTANCE_BINDING");timing.walk(c, &format!("{}.timing", &format!("{}@GrantTriggeredEffect",path)), "ENGINE_TIMING_NAME");expiry.walk(c, &format!("{}.expiry", &format!("{}@GrantTriggeredEffect",path)), "ENGINE_EXPIRY_NAME");body.walk(c, &format!("{}.body", &format!("{}@GrantTriggeredEffect",path)), "UNCLASSIFIED_STRING");dynamic.walk(c, &format!("{}@GrantTriggeredEffect.dynamic", path), "UNCLASSIFIED_STRING");
trigger_if.walk(c, &format!("{}@GrantTriggeredEffect.trigger_if", path), "UNCLASSIFIED_STRING");
},
digimon_dsl::compiled::CompiledStep::SelectOwnPermanent{filter,bind_as,selector,prompt,prompt_key,optional,continue_on_decline,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOwnPermanent"));filter.walk(c, &format!("{}.filter", &format!("{}@SelectOwnPermanent",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOwnPermanent",path)), "EXISTING_INSTANCE_BINDING");selector.walk(c, &format!("{}.selector", &format!("{}@SelectOwnPermanent",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOwnPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectOwnPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectOwnPermanent",path)), "UNCLASSIFIED_STRING");continue_on_decline.walk(c, &format!("{}.continue_on_decline", &format!("{}@SelectOwnPermanent",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectOwnPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectOpponentPermanent{filter,bind_as,selector,prompt,prompt_key,optional,continue_on_decline,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOpponentPermanent"));filter.walk(c, &format!("{}.filter", &format!("{}@SelectOpponentPermanent",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOpponentPermanent",path)), "EXISTING_INSTANCE_BINDING");selector.walk(c, &format!("{}.selector", &format!("{}@SelectOpponentPermanent",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOpponentPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectOpponentPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectOpponentPermanent",path)), "UNCLASSIFIED_STRING");continue_on_decline.walk(c, &format!("{}.continue_on_decline", &format!("{}@SelectOpponentPermanent",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectOpponentPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectAnyPermanent{filter,bind_as,selector,prompt,prompt_key,optional,then,continue_on_decline} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectAnyPermanent"));filter.walk(c, &format!("{}.filter", &format!("{}@SelectAnyPermanent",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectAnyPermanent",path)), "EXISTING_INSTANCE_BINDING");selector.walk(c, &format!("{}.selector", &format!("{}@SelectAnyPermanent",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectAnyPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectAnyPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectAnyPermanent",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectAnyPermanent",path)), "UNCLASSIFIED_STRING");continue_on_decline.walk(c, &format!("{}.continue_on_decline", &format!("{}@SelectAnyPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectDnaPair{left_filter,right_filter,bind_left_as,bind_right_as,prompt,prompt_key,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectDnaPair"));left_filter.walk(c, &format!("{}.left_filter", &format!("{}@SelectDnaPair",path)), "UNCLASSIFIED_STRING");right_filter.walk(c, &format!("{}.right_filter", &format!("{}@SelectDnaPair",path)), "UNCLASSIFIED_STRING");bind_left_as.walk(c, &format!("{}.bind_left_as", &format!("{}@SelectDnaPair",path)), "UNCLASSIFIED_STRING");bind_right_as.walk(c, &format!("{}.bind_right_as", &format!("{}@SelectDnaPair",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectDnaPair",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectDnaPair",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectDnaPair",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectHand{of,filter,bind_as,prompt,prompt_key,optional,cost,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectHand"));of.walk(c, &format!("{}.of", &format!("{}@SelectHand",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectHand",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectHand",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectHand",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectHand",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectHand",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@SelectHand",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectHand",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectTrash{of,filter,bind_as,prompt,prompt_key,optional,cost,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectTrash"));of.walk(c, &format!("{}.of", &format!("{}@SelectTrash",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectTrash",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectTrash",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectTrash",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectTrash",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectTrash",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@SelectTrash",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectTrash",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectMaterial{of_permanent,filter,bind_as,prompt,prompt_key,optional} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectMaterial"));of_permanent.walk(c, &format!("{}.of_permanent", &format!("{}@SelectMaterial",path)), "EXISTING_INSTANCE_BINDING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectMaterial",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectMaterial",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectMaterial",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectMaterial",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectMaterial",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectMaterials{of_permanent,max,filter,uniqueness,bind_as,prompt,prompt_key,optional_zero,min,same_by,
            named_groups,
            require_max,
        } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectMaterials"));of_permanent.walk(c, &format!("{}.of_permanent", &format!("{}@SelectMaterials",path)), "EXISTING_INSTANCE_BINDING");max.walk(c, &format!("{}.max", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");uniqueness.walk(c, &format!("{}.uniqueness", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectMaterials",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectMaterials",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectMaterials",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional_zero.walk(c, &format!("{}.optional_zero", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");min.walk(c, &format!("{}.min", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");same_by.walk(c, &format!("{}.same_by", &format!("{}@SelectMaterials",path)), "UNCLASSIFIED_STRING");named_groups.walk(c, &format!("{}@SelectMaterials.named_groups", path), "MATCH_NAME_EXACT");
require_max.walk(c, &format!("{}@SelectMaterials.require_max", path), "UNCLASSIFIED_STRING");
},
digimon_dsl::compiled::CompiledStep::SelectOwnSources{target,filter,min,max,bind_as,prompt,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOwnSources"));target.walk(c, &format!("{}.target", &format!("{}@SelectOwnSources",path)), "EXISTING_INSTANCE_BINDING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectOwnSources",path)), "UNCLASSIFIED_STRING");min.walk(c, &format!("{}.min", &format!("{}@SelectOwnSources",path)), "UNCLASSIFIED_STRING");max.walk(c, &format!("{}.max", &format!("{}@SelectOwnSources",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOwnSources",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOwnSources",path)), "METADATA_NO_EXECUTABLE_REFERENCE");then.walk(c, &format!("{}.then", &format!("{}@SelectOwnSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectOpponentSources{target,filter,min,max,clamp_to_available,bind_as,prompt,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOpponentSources"));target.walk(c, &format!("{}.target", &format!("{}@SelectOpponentSources",path)), "EXISTING_INSTANCE_BINDING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectOpponentSources",path)), "UNCLASSIFIED_STRING");min.walk(c, &format!("{}.min", &format!("{}@SelectOpponentSources",path)), "UNCLASSIFIED_STRING");max.walk(c, &format!("{}.max", &format!("{}@SelectOpponentSources",path)), "UNCLASSIFIED_STRING");clamp_to_available.walk(c, &format!("{}.clamp_to_available", &format!("{}@SelectOpponentSources",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOpponentSources",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOpponentSources",path)), "METADATA_NO_EXECUTABLE_REFERENCE");then.walk(c, &format!("{}.then", &format!("{}@SelectOpponentSources",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectOpponentDpBudget{dp_budget,min_picks,filter,bind_as,prompt,then,deletion_cap} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOpponentDpBudget"));dp_budget.walk(c, &format!("{}.dp_budget", &format!("{}@SelectOpponentDpBudget",path)), "UNCLASSIFIED_STRING");min_picks.walk(c, &format!("{}.min_picks", &format!("{}@SelectOpponentDpBudget",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectOpponentDpBudget",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOpponentDpBudget",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOpponentDpBudget",path)), "METADATA_NO_EXECUTABLE_REFERENCE");then.walk(c, &format!("{}.then", &format!("{}@SelectOpponentDpBudget",path)), "UNCLASSIFIED_STRING");deletion_cap.walk(c, &format!("{}.deletion_cap", &format!("{}@SelectOpponentDpBudget",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectZoneCards{of,zones,filter,min,max,optional_zero,distinct_by,play_cost_budget,exclude,bind_as,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectZoneCards"));of.walk(c, &format!("{}.of", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");zones.walk(c, &format!("{}.zones", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");min.walk(c, &format!("{}.min", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");max.walk(c, &format!("{}.max", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");optional_zero.walk(c, &format!("{}.optional_zero", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");distinct_by.walk(c, &format!("{}.distinct_by", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");play_cost_budget.walk(c, &format!("{}.play_cost_budget", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");exclude.walk(c, &format!("{}.exclude", &format!("{}@SelectZoneCards",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectZoneCards",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectZoneCards",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::PlayCardsFree{cards} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlayCardsFree"));cards.walk(c, &format!("{}.cards", &format!("{}@PlayCardsFree",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceCardsAsBottomSources{cards,target} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PlaceCardsAsBottomSources"));cards.walk(c, &format!("{}.cards", &format!("{}@PlaceCardsAsBottomSources",path)), "UNCLASSIFIED_STRING");target.walk(c, &format!("{}.target", &format!("{}@PlaceCardsAsBottomSources",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnTopStackedToDeck{targets,count,position,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnTopStackedToDeck"));targets.walk(c, &format!("{}.targets", &format!("{}@ReturnTopStackedToDeck",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@ReturnTopStackedToDeck",path)), "UNCLASSIFIED_STRING");position.walk(c, &format!("{}.position", &format!("{}@ReturnTopStackedToDeck",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@ReturnTopStackedToDeck",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::SelectOpponentPlayCostBudget{play_cost_budget,min_picks,filter,bind_as,prompt,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOpponentPlayCostBudget"));play_cost_budget.walk(c, &format!("{}.play_cost_budget", &format!("{}@SelectOpponentPlayCostBudget",path)), "UNCLASSIFIED_STRING");min_picks.walk(c, &format!("{}.min_picks", &format!("{}@SelectOpponentPlayCostBudget",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectOpponentPlayCostBudget",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOpponentPlayCostBudget",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOpponentPlayCostBudget",path)), "METADATA_NO_EXECUTABLE_REFERENCE");then.walk(c, &format!("{}.then", &format!("{}@SelectOpponentPlayCostBudget",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectOwnBreedingPermanent{bind_as,prompt,optional,filter,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOwnBreedingPermanent"));bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOwnBreedingPermanent",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOwnBreedingPermanent",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectOwnBreedingPermanent",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectOwnBreedingPermanent",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectOwnBreedingPermanent",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::TrashSelectedSources{source_refs} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashSelectedSources"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@TrashSelectedSources",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::TrashSelectedMaterials{target,cards} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashSelectedMaterials"));target.walk(c, &format!("{}.target", &format!("{}@TrashSelectedMaterials",path)), "EXISTING_INSTANCE_BINDING");cards.walk(c, &format!("{}.cards", &format!("{}@TrashSelectedMaterials",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReturnSelectedSourcesToHand{source_refs} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnSelectedSourcesToHand"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@ReturnSelectedSourcesToHand",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ReturnSelectedSourcesToDeck{source_refs,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnSelectedSourcesToDeck"));source_refs.walk(c, &format!("{}.source_refs", &format!("{}@ReturnSelectedSourcesToDeck",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnSelectedSourcesToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReturnAllSourcesToDeck{target,position} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReturnAllSourcesToDeck"));target.walk(c, &format!("{}.target", &format!("{}@ReturnAllSourcesToDeck",path)), "EXISTING_INSTANCE_BINDING");position.walk(c, &format!("{}.position", &format!("{}@ReturnAllSourcesToDeck",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::BindPermanentProperty{from,property,bind_as} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("BindPermanentProperty"));from.walk(c, &format!("{}.from", &format!("{}@BindPermanentProperty",path)), "EXISTING_INSTANCE_BINDING");property.walk(c, &format!("{}.property", &format!("{}@BindPermanentProperty",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@BindPermanentProperty",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::SelectReveal{of,filter,bind_as,prompt,prompt_key,optional,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectReveal"));of.walk(c, &format!("{}.of", &format!("{}@SelectReveal",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectReveal",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectReveal",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectReveal",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectReveal",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectReveal",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectReveal",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectRevealBuckets{from,buckets,no_duplicate_cards,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectRevealBuckets"));from.walk(c, &format!("{}.from", &format!("{}@SelectRevealBuckets",path)), "EXISTING_INSTANCE_BINDING");buckets.walk(c, &format!("{}.buckets", &format!("{}@SelectRevealBuckets",path)), "UNCLASSIFIED_STRING");no_duplicate_cards.walk(c, &format!("{}.no_duplicate_cards", &format!("{}@SelectRevealBuckets",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectRevealBuckets",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::SelectSecurity{continue_on_decline,of,filter,bind_as,prompt,prompt_key,optional,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectSecurity"));continue_on_decline.walk(c, &format!("{}.continue_on_decline", &format!("{}@SelectSecurity",path)), "UNCLASSIFIED_STRING");of.walk(c, &format!("{}.of", &format!("{}@SelectSecurity",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectSecurity",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectSecurity",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectSecurity",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectSecurity",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectSecurity",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectSecurity",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectUnionZone{of,zones,material_of,filter,zone_filters,material_carrier_filter,bind_as,prompt,prompt_key,optional,cost,then} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectUnionZone"));of.walk(c, &format!("{}.of", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");zones.walk(c, &format!("{}.zones", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");material_of.walk(c, &format!("{}.material_of", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");zone_filters.walk(c, &format!("{}.zone_filters", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");material_carrier_filter.walk(c, &format!("{}.material_carrier_filter", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectUnionZone",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectUnionZone",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectUnionZone",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional.walk(c, &format!("{}.optional", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@SelectUnionZone",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectOrderedPermutation{items,bind_as,prompt,prompt_key} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectOrderedPermutation"));items.walk(c, &format!("{}.items", &format!("{}@SelectOrderedPermutation",path)), "EXISTING_INSTANCE_BINDING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectOrderedPermutation",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectOrderedPermutation",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectOrderedPermutation",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::SelectCountCappedMulti{of,zone,max,min,clamp_to_available,filter,bind_as,prompt,prompt_key,optional_zero,distinct_by} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectCountCappedMulti"));of.walk(c, &format!("{}.of", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");zone.walk(c, &format!("{}.zone", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");max.walk(c, &format!("{}.max", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");min.walk(c, &format!("{}.min", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");clamp_to_available.walk(c, &format!("{}.clamp_to_available", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectCountCappedMulti",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectCountCappedMulti",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectCountCappedMulti",path)), "METADATA_NO_EXECUTABLE_REFERENCE");optional_zero.walk(c, &format!("{}.optional_zero", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");distinct_by.walk(c, &format!("{}.distinct_by", &format!("{}@SelectCountCappedMulti",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SelectEffectChoice{labels,legal_when,bind_as,prompt,prompt_key} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SelectEffectChoice"));labels.walk(c, &format!("{}.labels", &format!("{}@SelectEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");legal_when.walk(c, &format!("{}.legal_when", &format!("{}@SelectEffectChoice",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@SelectEffectChoice",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@SelectEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@SelectEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::RepeatEffectChoice{count,labels,bind_as,prompt,prompt_key,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RepeatEffectChoice"));count.walk(c, &format!("{}.count", &format!("{}@RepeatEffectChoice",path)), "UNCLASSIFIED_STRING");labels.walk(c, &format!("{}.labels", &format!("{}@RepeatEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@RepeatEffectChoice",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@RepeatEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@RepeatEffectChoice",path)), "METADATA_NO_EXECUTABLE_REFERENCE");body.walk(c, &format!("{}.body", &format!("{}@RepeatEffectChoice",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RepeatEffectChoiceRemaining{remaining,labels,bind_as,prompt,prompt_key,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RepeatEffectChoiceRemaining"));remaining.walk(c, &format!("{}.remaining", &format!("{}@RepeatEffectChoiceRemaining",path)), "UNCLASSIFIED_STRING");labels.walk(c, &format!("{}.labels", &format!("{}@RepeatEffectChoiceRemaining",path)), "METADATA_NO_EXECUTABLE_REFERENCE");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@RepeatEffectChoiceRemaining",path)), "EXISTING_INSTANCE_BINDING");prompt.walk(c, &format!("{}.prompt", &format!("{}@RepeatEffectChoiceRemaining",path)), "METADATA_NO_EXECUTABLE_REFERENCE");prompt_key.walk(c, &format!("{}.prompt_key", &format!("{}@RepeatEffectChoiceRemaining",path)), "METADATA_NO_EXECUTABLE_REFERENCE");body.walk(c, &format!("{}.body", &format!("{}@RepeatEffectChoiceRemaining",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::AsSelectingPlayer{of,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("AsSelectingPlayer"));of.walk(c, &format!("{}.of", &format!("{}@AsSelectingPlayer",path)), "UNCLASSIFIED_STRING");body.walk(c, &format!("{}.body", &format!("{}@AsSelectingPlayer",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::If{condition,then,else_branch} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("If"));condition.walk(c, &format!("{}.condition", &format!("{}@If",path)), "UNCLASSIFIED_STRING");then.walk(c, &format!("{}.then", &format!("{}@If",path)), "UNCLASSIFIED_STRING");else_branch.walk(c, &format!("{}.else_branch", &format!("{}@If",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ForEach{over,bind_as,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ForEach"));over.walk(c, &format!("{}.over", &format!("{}@ForEach",path)), "UNCLASSIFIED_STRING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@ForEach",path)), "EXISTING_INSTANCE_BINDING");body.walk(c, &format!("{}.body", &format!("{}@ForEach",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PerSelected{selection,bind_as,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("PerSelected"));selection.walk(c, &format!("{}.selection", &format!("{}@PerSelected",path)), "EXISTING_INSTANCE_BINDING");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@PerSelected",path)), "EXISTING_INSTANCE_BINDING");body.walk(c, &format!("{}.body", &format!("{}@PerSelected",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ScheduleDelayed{when,body} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ScheduleDelayed"));when.walk(c, &format!("{}.when", &format!("{}@ScheduleDelayed",path)), "UNCLASSIFIED_STRING");body.walk(c, &format!("{}.body", &format!("{}@ScheduleDelayed",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ScheduleDeletePlayedAtTurnEnd{binding,at_opponents_turn} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ScheduleDeletePlayedAtTurnEnd"));binding.walk(c, &format!("{}.binding", &format!("{}@ScheduleDeletePlayedAtTurnEnd",path)), "EXISTING_INSTANCE_BINDING");at_opponents_turn.walk(c, &format!("{}.at_opponents_turn", &format!("{}@ScheduleDeletePlayedAtTurnEnd",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceSelfAsDelayOption => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("PlaceSelfAsDelayOption")),
digimon_dsl::compiled::CompiledStep::LinkToOwnDigimon{optional,free,filter} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("LinkToOwnDigimon"));optional.walk(c, &format!("{}.optional", &format!("{}@LinkToOwnDigimon",path)), "UNCLASSIFIED_STRING");free.walk(c, &format!("{}.free", &format!("{}@LinkToOwnDigimon",path)), "UNCLASSIFIED_STRING");filter.walk(c, &format!("{}.filter", &format!("{}@LinkToOwnDigimon",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ReduceLinkCost{amount} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ReduceLinkCost"));amount.walk(c, &format!("{}.amount", &format!("{}@ReduceLinkCost",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::LinkCards{from,filter,to,count,cost,prompt,bind_as,host_filter,exclude_source,cost_reduce} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("LinkCards"));from.walk(c, &format!("{}.from", &format!("{}@LinkCards",path)), "EXISTING_INSTANCE_BINDING");filter.walk(c, &format!("{}.filter", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");to.walk(c, &format!("{}.to", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");count.walk(c, &format!("{}.count", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@LinkCards",path)), "METADATA_NO_EXECUTABLE_REFERENCE");bind_as.walk(c, &format!("{}.bind_as", &format!("{}@LinkCards",path)), "EXISTING_INSTANCE_BINDING");host_filter.walk(c, &format!("{}.host_filter", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");exclude_source.walk(c, &format!("{}.exclude_source", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");cost_reduce.walk(c, &format!("{}.cost_reduce", &format!("{}@LinkCards",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RelinkSelfToOwnDigimon{host_filter,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RelinkSelfToOwnDigimon"));host_filter.walk(c, &format!("{}.host_filter", &format!("{}@RelinkSelfToOwnDigimon",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@RelinkSelfToOwnDigimon",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::Optional(f0) => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Optional"));f0.walk(c,&format!("{}@Optional.0",path),role);},
digimon_dsl::compiled::CompiledStep::Battle{attacker,defender,compare} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("Battle"));attacker.walk(c, &format!("{}.attacker", &format!("{}@Battle",path)), "UNCLASSIFIED_STRING");defender.walk(c, &format!("{}.defender", &format!("{}@Battle",path)), "UNCLASSIFIED_STRING");compare.walk(c, &format!("{}.compare", &format!("{}@Battle",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MayAttackNow{attacker,targets,without_suspending,ignore_summoning_sickness,optional,windowed,prompt,cost_upgrade} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MayAttackNow"));attacker.walk(c, &format!("{}.attacker", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");targets.walk(c, &format!("{}.targets", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");without_suspending.walk(c, &format!("{}.without_suspending", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");ignore_summoning_sickness.walk(c, &format!("{}.ignore_summoning_sickness", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");windowed.walk(c, &format!("{}.windowed", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@MayAttackNow",path)), "METADATA_NO_EXECUTABLE_REFERENCE");cost_upgrade.walk(c, &format!("{}.cost_upgrade", &format!("{}@MayAttackNow",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ForceAttack{attacker,targets,without_suspending,prompt,cost_upgrade} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ForceAttack"));attacker.walk(c, &format!("{}.attacker", &format!("{}@ForceAttack",path)), "UNCLASSIFIED_STRING");targets.walk(c, &format!("{}.targets", &format!("{}@ForceAttack",path)), "UNCLASSIFIED_STRING");without_suspending.walk(c, &format!("{}.without_suspending", &format!("{}@ForceAttack",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@ForceAttack",path)), "METADATA_NO_EXECUTABLE_REFERENCE");cost_upgrade.walk(c, &format!("{}.cost_upgrade", &format!("{}@ForceAttack",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RedirectAttackTarget{new_target,player,targets,optional,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RedirectAttackTarget"));new_target.walk(c, &format!("{}.new_target", &format!("{}@RedirectAttackTarget",path)), "UNCLASSIFIED_STRING");player.walk(c, &format!("{}.player", &format!("{}@RedirectAttackTarget",path)), "UNCLASSIFIED_STRING");targets.walk(c, &format!("{}.targets", &format!("{}@RedirectAttackTarget",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@RedirectAttackTarget",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@RedirectAttackTarget",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
digimon_dsl::compiled::CompiledStep::CancelAttack => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("CancelAttack")),
digimon_dsl::compiled::CompiledStep::OpenCounterWindow => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("OpenCounterWindow")),
digimon_dsl::compiled::CompiledStep::RefundOpt => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("RefundOpt")),
digimon_dsl::compiled::CompiledStep::RefireEffect{source,timing,optional,
            target_attribution,
        } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RefireEffect"));source.walk(c, &format!("{}.source", &format!("{}@RefireEffect",path)), "EXISTING_INSTANCE_BINDING");timing.walk(c, &format!("{}.timing", &format!("{}@RefireEffect",path)), "ENGINE_TIMING_NAME");optional.walk(c, &format!("{}.optional", &format!("{}@RefireEffect",path)), "UNCLASSIFIED_STRING");target_attribution.walk(c, &format!("{}@RefireEffect.target_attribution", path), "UNCLASSIFIED_STRING");
},
digimon_dsl::compiled::CompiledStep::RefireCardEffect{card,timing} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RefireCardEffect"));card.walk(c, &format!("{}.card", &format!("{}@RefireCardEffect",path)), "EXISTING_INSTANCE_BINDING");timing.walk(c, &format!("{}.timing", &format!("{}@RefireCardEffect",path)), "ENGINE_TIMING_NAME");},
digimon_dsl::compiled::CompiledStep::EndAttack{enabled} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("EndAttack"));enabled.walk(c, &format!("{}.enabled", &format!("{}@EndAttack",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SecurityPlacementSuccessFinalize { card, bind_as } => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SecurityPlacementSuccessFinalize"));card.walk(c, &format!("{}.card", path), "PHYSICAL_CARD_HANDLE");bind_as.walk(c, &format!("{}.bind_as", path), "BINDING_DEFINITION");},
digimon_dsl::compiled::CompiledStep::CancelReplacement => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("CancelReplacement")),
digimon_dsl::compiled::CompiledStep::TrashOwnLinkCardAndCancelLeave => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("TrashOwnLinkCardAndCancelLeave")),
digimon_dsl::compiled::CompiledStep::TrashOwnLinkCardMatchingAndCancelLeave{filter} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("TrashOwnLinkCardMatchingAndCancelLeave"));filter.walk(c, &format!("{}.filter", &format!("{}@TrashOwnLinkCardMatchingAndCancelLeave",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::PlaceLinkCardAsBottomSourceAndCancelLeave => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("PlaceLinkCardAsBottomSourceAndCancelLeave")),
digimon_dsl::compiled::CompiledStep::HandleReplacement => c.node(path,"digimon_dsl::compiled::CompiledStep","ENUM_SCALAR",serde_json::json!("HandleReplacement")),
digimon_dsl::compiled::CompiledStep::RedirectReplacement{zone} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RedirectReplacement"));zone.walk(c, &format!("{}.zone", &format!("{}@RedirectReplacement",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::SubstituteReplacement{subject} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("SubstituteReplacement"));subject.walk(c, &format!("{}.subject", &format!("{}@SubstituteReplacement",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::RawRust{fn_name,consumes,binds} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("RawRust"));fn_name.walk(c, &format!("{}.fn_name", &format!("{}@RawRust",path)), "HOOK_STEP");consumes.walk(c, &format!("{}.consumes", &format!("{}@RawRust",path)), "EXISTING_INSTANCE_BINDING");binds.walk(c, &format!("{}.binds", &format!("{}@RawRust",path)), "EXISTING_INSTANCE_BINDING");},
digimon_dsl::compiled::CompiledStep::ActivationCost{kind} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ActivationCost"));kind.walk(c, &format!("{}.kind", &format!("{}@ActivationCost",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::ArmDigivolveCostReducer{amount,single_fire,target_color,suspend_cost} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("ArmDigivolveCostReducer"));amount.walk(c, &format!("{}.amount", &format!("{}@ArmDigivolveCostReducer",path)), "UNCLASSIFIED_STRING");single_fire.walk(c, &format!("{}.single_fire", &format!("{}@ArmDigivolveCostReducer",path)), "UNCLASSIFIED_STRING");target_color.walk(c, &format!("{}.target_color", &format!("{}@ArmDigivolveCostReducer",path)), "UNCLASSIFIED_STRING");suspend_cost.walk(c, &format!("{}.suspend_cost", &format!("{}@ArmDigivolveCostReducer",path)), "UNCLASSIFIED_STRING");},
digimon_dsl::compiled::CompiledStep::MayDnaDigivolveNow{anchor,partner_filter,target_filter,cost,ignore_requirements,optional,prompt} => { c.node(path,"digimon_dsl::compiled::CompiledStep","VARIANT",serde_json::json!("MayDnaDigivolveNow"));anchor.walk(c, &format!("{}.anchor", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");partner_filter.walk(c, &format!("{}.partner_filter", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");target_filter.walk(c, &format!("{}.target_filter", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");cost.walk(c, &format!("{}.cost", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");ignore_requirements.walk(c, &format!("{}.ignore_requirements", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");optional.walk(c, &format!("{}.optional", &format!("{}@MayDnaDigivolveNow",path)), "UNCLASSIFIED_STRING");prompt.walk(c, &format!("{}.prompt", &format!("{}@MayDnaDigivolveNow",path)), "METADATA_NO_EXECUTABLE_REFERENCE");},
            digimon_dsl::compiled::CompiledStep::CompleteCostPayment { receipt } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("CompleteCostPayment"));
                receipt.walk(c, &format!("{}@CompleteCostPayment.receipt", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::PaySourcePrefix { host_filter, minimum, optional } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("PaySourcePrefix"));
                host_filter.walk(c, &format!("{}@PaySourcePrefix.host_filter", path), "UNCLASSIFIED_STRING");
                minimum.walk(c, &format!("{}@PaySourcePrefix.minimum", path), "UNCLASSIFIED_STRING");
                optional.walk(c, &format!("{}@PaySourcePrefix.optional", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::DeleteSelfAndCancelAllMatchingLeaves => c.node(path, "digimon_dsl::compiled::CompiledStep", "ENUM_SCALAR", serde_json::json!("DeleteSelfAndCancelAllMatchingLeaves")),
            digimon_dsl::compiled::CompiledStep::PlayToBreedingFromHand { of, hand_index } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("PlayToBreedingFromHand"));
                of.walk(c, &format!("{}@PlayToBreedingFromHand.of", path), "UNCLASSIFIED_STRING");
                hand_index.walk(c, &format!("{}@PlayToBreedingFromHand.hand_index", path), "EXISTING_INSTANCE_BINDING");
            }
            digimon_dsl::compiled::CompiledStep::TrashSelectedSourcesAsCost { source_refs, min_count } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("TrashSelectedSourcesAsCost"));
                source_refs.walk(c, &format!("{}@TrashSelectedSourcesAsCost.source_refs", path), "EXISTING_INSTANCE_BINDING");
                min_count.walk(c, &format!("{}@TrashSelectedSourcesAsCost.min_count", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SourcePlayCaptured { player, host_identity, breeding, card, receipt, cost_delta, suppress_on_play, suspended } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SourcePlayCaptured"));
                player.walk(c, &format!("{}@SourcePlayCaptured.player", path), "UNCLASSIFIED_STRING");
                host_identity.walk(c, &format!("{}@SourcePlayCaptured.host_identity", path), "UNCLASSIFIED_STRING");
                breeding.walk(c, &format!("{}@SourcePlayCaptured.breeding", path), "UNCLASSIFIED_STRING");
                card.walk(c, &format!("{}@SourcePlayCaptured.card", path), "UNCLASSIFIED_STRING");
                receipt.walk(c, &format!("{}@SourcePlayCaptured.receipt", path), "UNCLASSIFIED_STRING");
                cost_delta.walk(c, &format!("{}@SourcePlayCaptured.cost_delta", path), "UNCLASSIFIED_STRING");
                suppress_on_play.walk(c, &format!("{}@SourcePlayCaptured.suppress_on_play", path), "UNCLASSIFIED_STRING");
                suspended.walk(c, &format!("{}@SourcePlayCaptured.suspended", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SourcePlayBatchCommit { receipts } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SourcePlayBatchCommit"));
                receipts.walk(c, &format!("{}@SourcePlayBatchCommit.receipts", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::WithSimultaneousPlaySubject { binding, body } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("WithSimultaneousPlaySubject"));
                binding.walk(c, &format!("{}@WithSimultaneousPlaySubject.binding", path), "EXISTING_INSTANCE_BINDING");
                body.walk(c, &format!("{}@WithSimultaneousPlaySubject.body", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SourcePlayFinalize { receipt, card, bind_as } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SourcePlayFinalize"));
                receipt.walk(c, &format!("{}@SourcePlayFinalize.receipt", path), "UNCLASSIFIED_STRING");
                card.walk(c, &format!("{}@SourcePlayFinalize.card", path), "UNCLASSIFIED_STRING");
                bind_as.walk(c, &format!("{}@SourcePlayFinalize.bind_as", path), "BINDING_DEFINITION");
            }
            digimon_dsl::compiled::CompiledStep::DeleteCapturedBatch { receipt, targets } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("DeleteCapturedBatch"));
                receipt.walk(c, &format!("{}@DeleteCapturedBatch.receipt", path), "UNCLASSIFIED_STRING");
                targets.walk(c, &format!("{}@DeleteCapturedBatch.targets", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::DeleteReceiptFinalize { receipt, cost_credit } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("DeleteReceiptFinalize"));
                receipt.walk(c, &format!("{}@DeleteReceiptFinalize.receipt", path), "UNCLASSIFIED_STRING");
                cost_credit.walk(c, &format!("{}@DeleteReceiptFinalize.cost_credit", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SecurityPlacementCancelFinalize { card } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SecurityPlacementCancelFinalize"));
                card.walk(c, &format!("{}@SecurityPlacementCancelFinalize.card", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::PaidDeletionPreventionFinalize { receipt, payer } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("PaidDeletionPreventionFinalize"));
                receipt.walk(c, &format!("{}@PaidDeletionPreventionFinalize.receipt", path), "UNCLASSIFIED_STRING");
                payer.walk(c, &format!("{}@PaidDeletionPreventionFinalize.payer", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::PaidDeletionBatchPreventionFinalize { receipt, payer, members, filter, protect_all, cause, cause_player } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("PaidDeletionBatchPreventionFinalize"));
                receipt.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.receipt", path), "UNCLASSIFIED_STRING");
                payer.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.payer", path), "UNCLASSIFIED_STRING");
                members.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.members", path), "UNCLASSIFIED_STRING");
                filter.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.filter", path), "UNCLASSIFIED_STRING");
                protect_all.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.protect_all", path), "UNCLASSIFIED_STRING");
                cause.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.cause", path), "UNCLASSIFIED_STRING");
                cause_player.walk(c, &format!("{}@PaidDeletionBatchPreventionFinalize.cause_player", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::PreventDeletionMemberFinalize { binding, members, filter, cause, cause_player } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("PreventDeletionMemberFinalize"));
                binding.walk(c, &format!("{}@PreventDeletionMemberFinalize.binding", path), "EXISTING_INSTANCE_BINDING");
                members.walk(c, &format!("{}@PreventDeletionMemberFinalize.members", path), "UNCLASSIFIED_STRING");
                filter.walk(c, &format!("{}@PreventDeletionMemberFinalize.filter", path), "UNCLASSIFIED_STRING");
                cause.walk(c, &format!("{}@PreventDeletionMemberFinalize.cause", path), "UNCLASSIFIED_STRING");
                cause_player.walk(c, &format!("{}@PreventDeletionMemberFinalize.cause_player", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::ForEachRemaining { members, bind_as, body } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("ForEachRemaining"));
                members.walk(c, &format!("{}@ForEachRemaining.members", path), "UNCLASSIFIED_STRING");
                bind_as.walk(c, &format!("{}@ForEachRemaining.bind_as", path), "BINDING_DEFINITION");
                body.walk(c, &format!("{}@ForEachRemaining.body", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::ReturnAllToHand { over, selector } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("ReturnAllToHand"));
                over.walk(c, &format!("{}@ReturnAllToHand.over", path), "UNCLASSIFIED_STRING");
                selector.walk(c, &format!("{}@ReturnAllToHand.selector", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::ReturnAllToDeck { over, selector, position, include_sources } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("ReturnAllToDeck"));
                over.walk(c, &format!("{}@ReturnAllToDeck.over", path), "UNCLASSIFIED_STRING");
                selector.walk(c, &format!("{}@ReturnAllToDeck.selector", path), "UNCLASSIFIED_STRING");
                position.walk(c, &format!("{}@ReturnAllToDeck.position", path), "UNCLASSIFIED_STRING");
                include_sources.walk(c, &format!("{}@ReturnAllToDeck.include_sources", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::ReturnCapturedBatch { receipt, targets, position, include_sources } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("ReturnCapturedBatch"));
                receipt.walk(c, &format!("{}@ReturnCapturedBatch.receipt", path), "UNCLASSIFIED_STRING");
                targets.walk(c, &format!("{}@ReturnCapturedBatch.targets", path), "UNCLASSIFIED_STRING");
                position.walk(c, &format!("{}@ReturnCapturedBatch.position", path), "UNCLASSIFIED_STRING");
                include_sources.walk(c, &format!("{}@ReturnCapturedBatch.include_sources", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::ReturnReceiptFinalize { receipt } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("ReturnReceiptFinalize"));
                receipt.walk(c, &format!("{}@ReturnReceiptFinalize.receipt", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SuspendAndCancelAllMatchingLeaves { target, filter } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SuspendAndCancelAllMatchingLeaves"));
                target.walk(c, &format!("{}@SuspendAndCancelAllMatchingLeaves.target", path), "UNCLASSIFIED_STRING");
                filter.walk(c, &format!("{}@SuspendAndCancelAllMatchingLeaves.filter", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::FieldSourceCaptured { receipt, donor, target, target_breeding, face_down } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("FieldSourceCaptured"));
                receipt.walk(c, &format!("{}@FieldSourceCaptured.receipt", path), "UNCLASSIFIED_STRING");
                donor.walk(c, &format!("{}@FieldSourceCaptured.donor", path), "UNCLASSIFIED_STRING");
                target.walk(c, &format!("{}@FieldSourceCaptured.target", path), "UNCLASSIFIED_STRING");
                target_breeding.walk(c, &format!("{}@FieldSourceCaptured.target_breeding", path), "UNCLASSIFIED_STRING");
                face_down.walk(c, &format!("{}@FieldSourceCaptured.face_down", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SecurityPlayCaptured { receipt, player, card } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SecurityPlayCaptured"));
                receipt.walk(c, &format!("{}@SecurityPlayCaptured.receipt", path), "UNCLASSIFIED_STRING");
                player.walk(c, &format!("{}@SecurityPlayCaptured.player", path), "UNCLASSIFIED_STRING");
                card.walk(c, &format!("{}@SecurityPlayCaptured.card", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::SecurityPlayFinalize { receipt, player, card } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("SecurityPlayFinalize"));
                receipt.walk(c, &format!("{}@SecurityPlayFinalize.receipt", path), "UNCLASSIFIED_STRING");
                player.walk(c, &format!("{}@SecurityPlayFinalize.player", path), "UNCLASSIFIED_STRING");
                card.walk(c, &format!("{}@SecurityPlayFinalize.card", path), "UNCLASSIFIED_STRING");
            }
            digimon_dsl::compiled::CompiledStep::FieldSourceFinalize { receipt, bind_as, source_anchor, source_breeding, rebind } => {
                c.node(path, "digimon_dsl::compiled::CompiledStep", "VARIANT", serde_json::json!("FieldSourceFinalize"));
                receipt.walk(c, &format!("{}@FieldSourceFinalize.receipt", path), "UNCLASSIFIED_STRING");
                bind_as.walk(c, &format!("{}@FieldSourceFinalize.bind_as", path), "BINDING_DEFINITION");
                source_anchor.walk(c, &format!("{}@FieldSourceFinalize.source_anchor", path), "UNCLASSIFIED_STRING");
                source_breeding.walk(c, &format!("{}@FieldSourceFinalize.source_breeding", path), "UNCLASSIFIED_STRING");
                rebind.walk(c, &format!("{}@FieldSourceFinalize.rebind", path), "EXISTING_INSTANCE_BINDING");
            }
}
    }
}

impl Walk for digimon_dsl::step::PaySourcePrefixArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::PaySourcePrefixArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::PaySourcePrefixArgs { host_filter, minimum, optional } = self;
        host_filter.walk(c, &format!("{}.host_filter", path), "UNCLASSIFIED_STRING");
        minimum.walk(c, &format!("{}.minimum", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
    }
}

impl Walk for digimon_dsl::step::ReturnAllToDeckArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::ReturnAllToDeckArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::ReturnAllToDeckArgs { over, position, include_sources, selector } = self;
        over.walk(c, &format!("{}.over", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
        include_sources.walk(c, &format!("{}.include_sources", path), "UNCLASSIFIED_STRING");
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
    }
}

impl Walk for digimon_dsl::step::SuspendAllPreventionArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::SuspendAllPreventionArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::SuspendAllPreventionArgs { target, filter } = self;
        target.walk(c, &format!("{}.target", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}

impl Walk for digimon_dsl::step::TrashSelectedSourcesCostArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::TrashSelectedSourcesCostArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::TrashSelectedSourcesCostArgs { source_refs, min_count } = self;
        source_refs.walk(c, &format!("{}.source_refs", path), "EXISTING_INSTANCE_BINDING");
        min_count.walk(c, &format!("{}.min_count", path), "UNCLASSIFIED_STRING");
    }
}

impl Walk for digimon_dsl::step::PlayToBreedingFromHandArgs {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::step::PlayToBreedingFromHandArgs", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::step::PlayToBreedingFromHandArgs { of, hand_index } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        hand_index.walk(c, &format!("{}.hand_index", path), "EXISTING_INSTANCE_BINDING");
    }
}
