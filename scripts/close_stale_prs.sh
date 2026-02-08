#!/bin/bash
# Close all 12 stale PRs on mammalwithashell/digimon-deck-list-builder
# All use old python_impl/ imports (codebase migrated to digimon_gym/ in PR #17+)
# Run locally: chmod +x scripts/close_stale_prs.sh && ./scripts/close_stale_prs.sh

REPO="mammalwithashell/digimon-deck-list-builder"

gh pr close 10 --repo "$REPO" --comment "Closing: superseded by PR #30 which implemented HeadlessGame/InteractiveGame runners, phase decoders, and digivolve validation. This PR also uses old \`python_impl/\` imports — the codebase migrated to \`digimon_gym/\` in PR #17."

gh pr close 11 --repo "$REPO" --comment "Closing: BT20 scaffolding stubs superseded by later attempts (#19, #20, #21, #23). Uses old \`python_impl/\` imports incompatible with current \`digimon_gym/\` codebase."

gh pr close 12 --repo "$REPO" --comment "Closing: BT22/Koromon implementation uses old \`python_impl/\` imports. Dynamic DP support is now in the engine via merged PRs. BT22 cards can be re-implemented against the current codebase."

gh pr close 13 --repo "$REPO" --comment "Closing: EX10 implementation has only 4/74 card scripts and uses old \`python_impl/\` imports. The Link Card mechanic and card data can be re-implemented against the current \`digimon_gym/\` codebase."

gh pr close 16 --repo "$REPO" --comment "Closing: C#-via-pythonnet approach was abandoned. PR #17 migrated the project to a pure Python engine, making this wrapper unnecessary."

gh pr close 18 --repo "$REPO" --comment "Closing: Frontend PR includes build artifacts (2,264 files) and uses old \`python_impl/\` imports. React frontend should be re-implemented against the current \`digimon_gym/\` API when ready."

gh pr close 19 --repo "$REPO" --comment "Closing: BT20 scaffolding stubs superseded by #20, #21, #23. Uses old \`python_impl/\` imports."

gh pr close 20 --repo "$REPO" --comment "Closing: BT20 cards + engine bug fixes superseded by #21 and #23. Engine bugs (PendingAction NameError) already fixed in main. Uses old \`python_impl/\` imports."

gh pr close 21 --repo "$REPO" --comment "Closing: BT20 cards + OnSecurityCheck support superseded by #23. Uses old \`python_impl/\` imports incompatible with current \`digimon_gym/\` codebase."

gh pr close 22 --repo "$REPO" --comment "Closing: BT13 cards use old \`python_impl/\` imports. Engine effect logic fixes are already in main. BT13 cards can be re-implemented against the current codebase."

gh pr close 23 --repo "$REPO" --comment "Closing: BT20+BT23 cards use old \`python_impl/\` imports. BT23 scripts already exist in main (3 cards). Superseded by #24 for BT23 work."

gh pr close 24 --repo "$REPO" --comment "Closing: BT23 cards use old \`python_impl/\` imports. 3 BT23 scripts (BT23-002, BT23-006, BT23-008) already exist in main via prior merges."

echo ""
echo "All 12 stale PRs closed."
echo ""
echo "Stale remote branches to clean up (optional):"
echo "  git push origin --delete headless-sim-layer-5112873804251500595"
echo "  git push origin --delete bt20-scaffold-8551969060624260889"
echo "  git push origin --delete bt22-impl-8791746460671765750"
echo "  git push origin --delete ex10-implementation-18305237886536576125"
echo "  git push origin --delete digimon-gym-csharp-wrapper-14096912858858617723"
echo "  git push origin --delete feature/frontend-ui-17894407450604313491"
echo "  git push origin --delete feat/bt20-cards-3872011649460425590"
echo "  git push origin --delete bt20-implementation-9652948181899819948"
echo "  git push origin --delete add-bt20-cards-10224014577650935889"
echo "  git push origin --delete bt13-implementation-16514673244300186647"
echo "  git push origin --delete feat/implement-bt20-cards-15355563517739359955"
echo "  git push origin --delete implement-bt23-cards-4389602071278091427"
