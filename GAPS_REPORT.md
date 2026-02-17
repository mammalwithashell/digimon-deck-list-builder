# Transpiler Gaps Report - Set BT13

The following gaps and issues were identified in the transpiled Python scripts for Set BT13:

## 1. Complex Conditional Logic
The transpiler struggles with complex control flow, particularly "If X, then Y, else Z" logic.
-   **Issue**: It tends to combine multiple conditions into a single filter using logical AND, resulting in impossible conditions (e.g., `DP <= 6000` AND `DP >= 13000`).
-   **Example**: `BT13_111` (Gallantmon) "Delete 1... with 6000 DP or less. If no opponent's Digimon was deleted... delete 1... with 13000 DP or more."
-   **Mitigation**: Manual intervention is required to implement the branching logic in the `process` callback.

## 2. Cost Reduction Effects
Cost reduction effects (especially `ChangeCostClass` in C#) are often detected but not fully implemented.
-   **Issue**: The transpiler generates a comment `# Cost reduction... handled via cost_reduction property` but does not generate the logic to calculate and set this property.
-   **Example**: `BT13_111` (Gallantmon) play cost reduction based on trash count.
-   **Mitigation**: The `can_use_condition` callback should calculate the reduction and set `effect.cost_reduction = value`.

## 3. Event Context Checking
Triggered effects often lack specific context checks.
-   **Issue**: Triggers like `OnTappedAnyone` or `OnDestroyedAnyone` activate based on the timing flag but the generated condition function often fails to verify *who* or *what* triggered the event (e.g., checking if the suspended card is a Tamer of a specific color).
-   **Example**: `BT13_008` (Agumon) triggers when "one of your red or yellow Tamers becomes suspended" but the condition only checks `is_my_turn`.
-   **Mitigation**: Manually add checks against `context['caused_by_permanent']` or similar in the condition function.

## 4. Main Phase Effects (OnDeclaration)
Effects that activate during the Main Phase (via card usage or ability activation) are sometimes incomplete.
-   **Issue**: Effects with `EffectTiming.OnDeclaration` are generated but often lack the `process` callback or the logic to handle the effect (e.g., "Treat as Digimon").
-   **Example**: `BT13_008` (Agumon) "Treat Marcus Damon as a Digimon".
-   **Mitigation**: Needs manual implementation of the effect logic.

## 5. Static Effects (EffectTiming.None)
Static effects or effects with `EffectTiming.None` are often skipped or stubbed with `pass`.
-   **Issue**: Logic for static modifiers (like "Your Turn: +1000 DP") might be missing if not handled by standard keyword mappings.
-   **Mitigation**: Verify and implement static effects manually.

## 6. Token Generation
-   **Issue**: Token generation logic might be missing or incomplete if the token card definition is not found or handled correctly.

## Summary
While simple "On Play: Delete X" effects work well, complex competitive cards require manual review and adjustment after transpilation.
