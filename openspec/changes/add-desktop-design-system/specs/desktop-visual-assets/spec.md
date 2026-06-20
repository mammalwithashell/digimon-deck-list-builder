## ADDED Requirements

### Requirement: A vendored visual-asset pack is committed under the design module

The repository SHALL include a committed asset pack of card sleeves and Digimon
pixel sprites under the design module (`code/frontend/src/design/assets/`),
organized by asset kind. These assets MUST be referenced through the manifest
rather than by ad-hoc paths scattered across the codebase.

#### Scenario: Build includes the vendored assets

- **WHEN** the desktop build is produced
- **THEN** the bundled output includes the vendored sleeve and sprite assets referenced by the manifest

### Requirement: A typed manifest maps logical ids to assets with provenance

The asset layer SHALL provide a typed manifest that maps a stable logical id to
an asset reference plus metadata including a human-readable name and the source
(provenance) for attribution. Lookups by logical id SHALL go through the
manifest.

#### Scenario: Manifest entry carries provenance

- **WHEN** the manifest is queried for a known sleeve or sprite id
- **THEN** it returns the asset reference, a name, and the source/credit for that asset

### Requirement: Slot components render assets with pixelated rendering and a fallback

The library SHALL provide `CardSleeve` and `DigimonSprite` slot components that
resolve an asset from the manifest and render it with `image-rendering:
pixelated`. When the requested id is absent from the manifest, the component
SHALL render a procedural fallback rather than a broken image.

#### Scenario: Known id renders the pixel asset

- **WHEN** a `DigimonSprite` is rendered for an id present in the manifest
- **THEN** it displays that sprite with pixelated image rendering

#### Scenario: Unknown id renders a fallback

- **WHEN** a `CardSleeve` or `DigimonSprite` is rendered for an id absent from the manifest
- **THEN** it renders a procedural fallback and no broken-image placeholder appears

### Requirement: Vendored assets are theme-stable

`CardSleeve` and `DigimonSprite` SHALL render identically regardless of the
active theme; the theme MUST NOT recolor or filter the asset pixels.

#### Scenario: Asset is unchanged across a theme switch

- **WHEN** a sleeve or sprite is on screen and the user switches themes
- **THEN** the asset's pixels are rendered identically before and after the switch

### Requirement: Bundled community art is attributed in a credits surface

The client SHALL include a credits surface that attributes the bundled
community art to its sources (WE-Kaito's digimon-tcg-simulator for sleeves and
Project Drasil for pixel sprites), and the surface MUST be reachable in-app.

#### Scenario: Credits list the asset sources

- **WHEN** the user opens the credits surface
- **THEN** it lists WE-Kaito and Project Drasil as the sources of the bundled sleeve and sprite art

### Requirement: The landing footer reflects bundled community art

The landing page footer SHALL NOT claim the distribution contains no proprietary
assets. It MUST be reworded to acknowledge bundled community art and point the
reader at the credits.

#### Scenario: Footer acknowledges bundled art

- **WHEN** the landing page is rendered
- **THEN** its footer no longer states that no proprietary assets are distributed
- **AND** it acknowledges the bundled community art and references the credits
