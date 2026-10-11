"""Shadow ``cards.json`` generation and structural Dual projection tests."""

from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path

import pytest

from tools.card_data.generate import (
    COMPATIBILITY_PROJECTION_PATH,
    bootstrap_legacy_canonical_cards,
    bootstrap_legacy_compatibility_fields,
    build_compatibility_projection,
    generate_cards_json,
    main,
    project_canonical_card,
    render_legacy_cards_json,
)
from tools.card_data.models import (
    SCHEMA_VERSION,
    AdmissionStatus,
    AuthorityClass,
    CanonicalCard,
    CardFace,
    FaceKind,
    FieldProvenance,
    Printing,
    ReviewedCorrection,
    ScopeManifest,
    ScopeObservationRef,
    SourceName,
    SourceObservation,
)
from tools.card_data.normalization import normalize_card
from tools.card_data.registry import StableCardRegistry, read_registry
from tools.card_data.serialization import canonical_json_bytes, content_digest, sha256_hex
from tools.card_data.source_manifest import build_full_pool_source_manifest
from tools.card_data.sources.official import parse_card_list
from tools.card_data.transaction import (
    EMPTY_REVISION,
    PlanToken,
    RevisionMarker,
    TransactionJournal,
    write_revision_marker,
    write_transaction_journal,
)
from tools.card_data.validation import require_valid_canonical_cards
from tools.card_data.errors import ValidationError


DIGEST = "a" * 64


def _dual_card() -> CanonicalCard:
    fields = {
        "card_name_eng": "Habakirimon",
        "card_kind": "Dual",
        "card_colors": ["Yellow"],
        "type_eng": ["Shaman", "Glowing Dawn"],
        "form_eng": ["Mega"],
        "attribute_eng": ["Virus"],
        "evo_costs": [{"colors": ["Yellow"], "from_level": 5, "cost": 4}],
    }
    provenance = {
        key: FieldProvenance(
            selected_observation_id=DIGEST,
            observation_ids=(DIGEST,),
            authority=AuthorityClass.OFFICIAL,
        )
        for key in fields
    }
    return CanonicalCard(
        schema_version=SCHEMA_VERSION,
        card_id="BT25-043",
        admission=AdmissionStatus.ADMITTED,
        fields=fields,
        provenance=provenance,
        printings=(),
        faces=(
            CardFace(
                face_id="digimon",
                kind=FaceKind.DIGIMON,
                colors=("Yellow",),
                level=6,
                dp=12000,
                effect_text="Digimon face effect",
            ),
            CardFace(
                face_id="option",
                kind=FaceKind.OPTION,
                colors=("Black",),
                play_cost=6,
                effect_text="Option face effect",
                security_text="Option security effect",
            ),
        ),
    )


def _observation(source: SourceName, raw_digest: str) -> SourceObservation:
    draft = SourceObservation(
        schema_version=SCHEMA_VERSION,
        observation_id="0" * 64,
        source=source,
        source_record_key=f"{source.value}:BT1-001",
        source_url=f"https://example.test/{source.value}/BT1-001",
        raw_payload_digest=raw_digest,
        parser_version=f"{source.value}-fixture-v1",
        source_schema_version="fixture-v1",
        first_acquired_at="2026-07-13T00:00:00Z",
        extracted={"card_id": "BT1-001"},
    )
    return replace(draft, observation_id=content_digest(draft.identity_payload()))


def _scope(observation: SourceObservation) -> ScopeManifest:
    return ScopeManifest(
        schema_version=SCHEMA_VERSION,
        source=observation.source,
        scope=f"full-pool:{observation.source.value}-fixture",
        source_locations=(observation.source_url,),
        first_acquired_at=observation.first_acquired_at,
        parser_version=observation.parser_version,
        source_schema_version=observation.source_schema_version,
        observation_ids=(observation.observation_id,),
        observation_refs=(
            ScopeObservationRef(
                observation_id=observation.observation_id,
                gameplay_card_id="BT1-001",
                printing_id="BT1-001",
            ),
        ),
        complete=True,
        expected_count=1,
    )


def _committed_generation_fixture(
    root: Path,
    *,
    compatibility_fields: dict[str, object] | None = None,
) -> dict[str, bytes]:
    official = _observation(SourceName.OFFICIAL, "1" * 64)
    secondary = _observation(SourceName.DIGIMONCARD_IO, "2" * 64)
    scopes = (_scope(official), _scope(secondary))
    aggregate = build_full_pool_source_manifest(scopes)
    fields = {
        "card_name_eng": "Agumon",
        "card_kind": "Digimon",
        "card_colors": ["Red"],
        "play_cost": 3,
        "dp": 2000,
        "level": 3,
        "type_eng": ["Reptile"],
        "form_eng": ["Rookie"],
        "attribute_eng": ["Vaccine"],
        "effect_description_eng": "",
        "inherited_effect_description_eng": "",
        "security_effect_description_eng": "",
        "evo_costs": [],
    }
    provenance = {
        field_name: FieldProvenance(
            selected_observation_id=official.observation_id,
            observation_ids=(official.observation_id, secondary.observation_id),
            authority=AuthorityClass.OFFICIAL,
        )
        for field_name in fields
    }
    card = CanonicalCard(
        schema_version=SCHEMA_VERSION,
        card_id="BT1-001",
        admission=AdmissionStatus.ADMITTED,
        fields=fields,
        provenance=provenance,
        printings=(
            Printing(
                printing_id="BT1-001",
                observation_ids=(official.observation_id, secondary.observation_id),
            ),
        ),
        faces=(
            CardFace(
                face_id="primary",
                kind=FaceKind.DIGIMON,
                colors=("Red",),
                play_cost=3,
                level=3,
                dp=2000,
            ),
        ),
    )
    canonical = {card.card_id: card}
    registry = StableCardRegistry(
        schema_version=1,
        registry_version=1,
        capacity=20_000,
        padding_index=0,
        assignments={card.card_id: 1},
    )
    compatibility = {
        card.card_id: compatibility_fields
        or {"card_name_jpn": "", "rarity": 2, "xros_req": ""}
    }
    generated = generate_cards_json(
        canonical, registry, legacy_bootstrap=compatibility
    )
    cards_bytes = render_legacy_cards_json(generated)
    projection = build_compatibility_projection(
        canonical,
        registry,
        corrections=(),
        source_manifest_revision=aggregate.manifest_revision,
        compatibility_fields=compatibility,
        fixture_rows={},
        projected_cards=cards_bytes,
    )
    canonical_payload = {
        "schema_version": SCHEMA_VERSION,
        "source_manifest_digest": aggregate.manifest_revision,
        "source_manifests": {
            binding.source.value: binding.scope_manifest_digest
            for binding in aggregate.sources
        },
        "secondary_only_ids": [],
        "cards": canonical,
    }
    artifacts = {
        "data/cards.json": cards_bytes,
        "data/cards_canonical.json": canonical_json_bytes(canonical_payload),
        "data/card_corrections.json": canonical_json_bytes(
            {"schema_version": SCHEMA_VERSION, "corrections": []}
        ),
        "data/card_registry.json": canonical_json_bytes(registry.to_dict()),
        COMPATIBILITY_PROJECTION_PATH: canonical_json_bytes(projection.to_dict()),
        "data/card_sources/manifest.json": aggregate.to_bytes(),
        "data/card_sources/scopes/official/full-pool.json": canonical_json_bytes(
            scopes[0]
        ),
        "data/card_sources/scopes/digimoncard_io/full-pool.json": canonical_json_bytes(
            scopes[1]
        ),
        (
            "data/card_sources/observations/official/"
            f"{official.observation_id}.json"
        ): canonical_json_bytes(official),
        (
            "data/card_sources/observations/digimoncard_io/"
            f"{secondary.observation_id}.json"
        ): canonical_json_bytes(secondary),
    }
    for relative, body in artifacts.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
    _seal_generation_revision(root, artifacts)
    return artifacts


def _seal_generation_revision(root: Path, artifacts: dict[str, bytes]) -> None:
    marker = RevisionMarker.create(
        {relative: sha256_hex(body) for relative, body in artifacts.items()}
    )
    write_revision_marker(root / "data/card_data_revision.json", marker)


def _write_empty_base_rollback_journal(root: Path) -> None:
    token = PlanToken.create(
        requested_scope="full-pool",
        base_revision=EMPTY_REVISION,
        staged_digest="1" * 64,
        diff_digest="2" * 64,
    )
    journal = (
        TransactionJournal.begin(token, ())
        .start_rollback("generation fixture rollback")
        .finish_rollback()
    )
    write_transaction_journal(
        root / "data/.card_data_transaction.json",
        journal,
    )


def test_structural_dual_projection_is_explicit_and_id_agnostic():
    row = project_canonical_card(_dual_card())

    assert row["card_kind"] == 4
    assert row["play_cost"] == 6
    assert row["level"] == 6
    assert row["dp"] == 12000
    assert row["card_colors"] == [2, 5]
    assert row["effect_description_eng"] == "Digimon face effect"
    assert row["inherited_effect_description_eng"] == ""
    assert row["security_effect_description_eng"] == ""
    assert row["evo_costs"] == [
        {"card_color": 2, "level": 5, "memory_cost": 4}
    ]
    assert row["dual"] == {
        "digimon": {
            "level": 6, "dp": 12000, "colors": ["Yellow"],
            "traits": ["Shaman", "Glowing Dawn"],
            "evo_costs": [{"card_color": 2, "level": 5, "memory_cost": 4}],
            "effect_text": "Digimon face effect", "inherited_text": "",
            "keywords": ["ArtsDigivolve"],
        },
        "option": {
            "use_cost": 6, "colors": ["Black"],
            "effect_text": "Option face effect",
            "security_text": "Option security effect",
            "keywords": ["ArtsDigivolve"],
        },
    }


def test_dual_projection_keeps_inherited_effect_on_digimon_face_only():
    card = _dual_card()
    card = replace(card, faces=(
        replace(card.faces[0], inherited_text="Inherited ability"),
        replace(card.faces[1], effect_text="Option line 1\nOption line 2"),
    ))
    row = project_canonical_card(card)
    assert row["inherited_effect_description_eng"] == "Inherited ability"
    assert row["dual"]["digimon"]["inherited_text"] == "Inherited ability"
    assert row["dual"]["option"]["effect_text"] == "Option line 1\r\nOption line 2"
    assert "Option" not in row["inherited_effect_description_eng"]


def test_compatibility_projection_uses_legacy_crlf_inside_multiline_text():
    card = _dual_card()
    card = replace(
        card,
        faces=(
            replace(card.faces[0], effect_text="first line\nsecond line"),
            *card.faces[1:],
        ),
    )

    row = project_canonical_card(card)

    assert row["effect_description_eng"] == "first line\r\nsecond line"


def test_compatibility_projection_preserves_preformatted_legacy_text():
    card = _dual_card()
    legacy_text = "first line\nsecond line\r\nthird line"
    card = replace(
        card,
        faces=(
            replace(card.faces[0], effect_text=legacy_text),
            *card.faces[1:],
        ),
    )

    row = project_canonical_card(card)

    assert row["effect_description_eng"] == legacy_text


def test_official_normalization_flows_into_structural_dual_projection():
    fixture = (
        Path(__file__).parents[1]
        / "fixtures/card_data/official/bt25_dual.html"
    ).read_bytes()
    (printing,) = parse_card_list(
        fixture,
        source_url="https://world.digimoncard.com/cards/?category=522036",
        category_id="522036",
    )
    observation = SourceObservation(
        schema_version=SCHEMA_VERSION,
        observation_id=DIGEST,
        source=SourceName.OFFICIAL,
        source_record_key=printing.printing_id,
        source_url=printing.source_url,
        raw_payload_digest=DIGEST,
        parser_version="official-v1",
        source_schema_version="fixture-v1",
        first_acquired_at="2026-07-12T00:00:00Z",
        extracted=printing.extracted,
    )

    row = project_canonical_card(
        normalize_card(printing.card_id, (observation,))
    )

    assert row["play_cost"] == 6
    assert row["dp"] == 12000
    assert row["level"] == 6
    assert row["card_colors"] == [2]
    assert "Recovery +1" in row["effect_description_eng"]
    assert row["card_kind"] == 4
    assert row["inherited_effect_description_eng"] == ""
    assert "Use Req." in row["dual"]["option"]["effect_text"]


def test_shadow_generator_matches_current_legacy_snapshot_semantically_and_by_byte():
    root = Path(__file__).resolve().parents[3]
    snapshot_path = root / "data/cards.json"
    legacy_bytes = snapshot_path.read_bytes()
    legacy = json.loads(legacy_bytes.decode("utf-8"))
    registry = read_registry(root / "data/card_registry.json")

    canonical = bootstrap_legacy_canonical_cards(legacy)
    compatibility = bootstrap_legacy_compatibility_fields(legacy)
    require_valid_canonical_cards(canonical)
    generated = generate_cards_json(
        canonical, registry, legacy_bootstrap=compatibility, card_order=legacy
    )

    assert generated == legacy
    assert render_legacy_cards_json(generated) == legacy_bytes


def test_shadow_cli_checks_without_replacing_the_legacy_producer(tmp_path: Path):
    root = Path(__file__).resolve().parents[3]
    snapshot = root / "data/cards.json"
    registry = root / "data/card_registry.json"
    shadow = tmp_path / "cards.shadow.json"

    assert main(
        [
            "--legacy-snapshot",
            str(snapshot),
            "--registry",
            str(registry),
            "--output",
            str(shadow),
        ]
    ) == 0
    assert shadow.read_bytes() == snapshot.read_bytes()
    assert main(
        [
            "--legacy-snapshot",
            str(snapshot),
            "--registry",
            str(registry),
            "--output",
            str(snapshot),
        ]
    ) == 2


def test_shadow_cli_treats_terminal_empty_base_rollback_as_pre_cutover(
    tmp_path: Path,
):
    source_root = Path(__file__).resolve().parents[3]
    data = tmp_path / "data"
    data.mkdir()
    for name in ("cards.json", "card_registry.json"):
        (data / name).write_bytes((source_root / "data" / name).read_bytes())
    _write_empty_base_rollback_journal(tmp_path)

    assert main(["--repo-root", str(tmp_path), "--check"]) == 0
    assert not (data / "card_data_revision.json").exists()


def test_post_cutover_check_regenerates_from_marker_owned_canonical_inputs(
    tmp_path: Path,
):
    _committed_generation_fixture(tmp_path)

    assert main(["--repo-root", str(tmp_path), "--check"]) == 0


def test_post_cutover_check_preserves_nested_legacy_abi_key_order(
    tmp_path: Path,
):
    _committed_generation_fixture(
        tmp_path,
        compatibility_fields={
            "card_name_jpn": "",
            "rarity": 2,
            "xros_req": "",
            "dna_costs": [
                {
                    "requirement1": {
                        "level": 4,
                        "card_colors": ["Blue"],
                        "name_contains": "",
                        "text_contains": "",
                    },
                    "requirement2": {
                        "level": 4,
                        "card_colors": ["Green"],
                        "name_contains": "",
                        "text_contains": "",
                    },
                    "memory_cost": 0,
                }
            ],
            "digixros_costs": [
                {
                    "elements": [
                        {
                            "name_contains": "Agumon",
                            "trait_match": "",
                            "trait_alternatives": [],
                            "level_max": None,
                            "count": 1,
                            "is_digimon_only": False,
                            "color": None,
                        }
                    ],
                    "reduce_cost_per_card": 2,
                    "max_materials": 1,
                    "different_card_numbers": False,
                    "different_names": False,
                    "has_text": "",
                    "source_zones": ["hand", "field"],
                }
            ],
        },
    )

    assert main(["--repo-root", str(tmp_path), "--check"]) == 0


def test_post_cutover_check_does_not_bootstrap_from_cards_json(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
):
    artifacts = _committed_generation_fixture(tmp_path)
    tampered = json.loads(artifacts["data/cards.json"].decode("utf-8"))
    tampered["BT1-001"]["card_name_eng"] = "Hand Edited"
    tampered_cards = render_legacy_cards_json(tampered)
    projection = json.loads(
        artifacts[COMPATIBILITY_PROJECTION_PATH].decode("utf-8")
    )
    # Keep the sidecar's output hash current so the failure can only come from
    # independently regenerating the canonical fields, not a stale hash check.
    projection["projected_cards_sha256"] = sha256_hex(tampered_cards)
    artifacts["data/cards.json"] = tampered_cards
    artifacts[COMPATIBILITY_PROJECTION_PATH] = canonical_json_bytes(projection)
    (tmp_path / "data/cards.json").write_bytes(tampered_cards)
    (tmp_path / COMPATIBILITY_PROJECTION_PATH).write_bytes(
        artifacts[COMPATIBILITY_PROJECTION_PATH]
    )
    _seal_generation_revision(tmp_path, artifacts)

    assert main(["--repo-root", str(tmp_path), "--check"]) == 1
    assert "regenerated canonical cards.json differs" in capsys.readouterr().err


def test_post_cutover_check_requires_marker_owned_projection_sidecar(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
):
    artifacts = _committed_generation_fixture(tmp_path)
    artifacts.pop(COMPATIBILITY_PROJECTION_PATH)
    (tmp_path / COMPATIBILITY_PROJECTION_PATH).unlink()
    _seal_generation_revision(tmp_path, artifacts)

    assert main(["--repo-root", str(tmp_path), "--check"]) == 1
    assert "required generation input" in capsys.readouterr().err


def test_post_cutover_check_rejects_stale_projection_binding(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
):
    artifacts = _committed_generation_fixture(tmp_path)
    projection = json.loads(
        artifacts[COMPATIBILITY_PROJECTION_PATH].decode("utf-8")
    )
    projection["canonical_revision"] = "f" * 64
    artifacts[COMPATIBILITY_PROJECTION_PATH] = canonical_json_bytes(projection)
    (tmp_path / COMPATIBILITY_PROJECTION_PATH).write_bytes(
        artifacts[COMPATIBILITY_PROJECTION_PATH]
    )
    _seal_generation_revision(tmp_path, artifacts)

    assert main(["--repo-root", str(tmp_path), "--check"]) == 1
    assert "stale or malformed" in capsys.readouterr().err


def test_post_cutover_check_rejects_hash_changing_canonical_input(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
):
    _committed_generation_fixture(tmp_path)
    (tmp_path / "data/cards_canonical.json").write_bytes(b"{}")

    assert main(["--repo-root", str(tmp_path), "--check"]) == 1
    assert "digest validation" in capsys.readouterr().err


def test_generated_index_fields_come_only_from_stable_registry():
    card = _dual_card()
    registry = StableCardRegistry(
        schema_version=1,
        registry_version=1,
        capacity=20_000,
        padding_index=0,
        assignments={card.card_id: 777},
    )

    row = generate_cards_json({card.card_id: card}, registry)[card.card_id]

    assert row["index"] == 777
    assert row["norm_id"] == 777 / 20_000
    assert row["card_index"] == 43

    with pytest.raises(ValidationError, match="must be derived"):
        project_canonical_card(
            card, legacy_bootstrap={"legacy_card_index": 999}
        )


def test_reviewed_corrections_apply_before_compatibility_projection():
    card = _dual_card()
    evidence_digest = "b" * 64
    observation = SourceObservation(
        schema_version=SCHEMA_VERSION,
        observation_id=evidence_digest,
        source=SourceName.OFFICIAL,
        source_record_key=card.card_id,
        source_url="https://example.test/BT25-043",
        raw_payload_digest=evidence_digest,
        parser_version="fixture-v1",
        source_schema_version="fixture-v1",
        first_acquired_at="2026-07-12T00:00:00Z",
        extracted={"card_id": card.card_id},
    )
    correction = ReviewedCorrection(
        schema_version=SCHEMA_VERSION,
        correction_id="BT25-043-name",
        card_id=card.card_id,
        field_path="card_name_eng",
        value="Reviewed Habakirimon",
        evidence_observation_ids=(evidence_digest,),
        reason="Reviewed official name",
        reviewer="qa",
        reviewed_at="2026-07-12",
    )
    registry = StableCardRegistry(
        schema_version=1,
        registry_version=1,
        capacity=20_000,
        padding_index=0,
        assignments={card.card_id: 777},
    )

    row = generate_cards_json(
        {card.card_id: card},
        registry,
        corrections=(correction,),
        observations={evidence_digest: observation},
    )[card.card_id]

    assert row["card_name_eng"] == "Reviewed Habakirimon"


def test_official_japanese_digimon_type_normalizes_without_dropping_printing():
    observation = replace(_observation(SourceName.OFFICIAL, DIGEST), extracted={
        "card_id": "EX4-019", "name": "Gaomon", "card_type": "\u30c7\u30b8\u30e2\u30f3",
        "colors": ["Blue"], "level": 3, "dp": 2000, "play_cost": 3,
        "faces": [{"face_id": "primary", "kind": "\u30c7\u30b8\u30e2\u30f3",
                   "colors": ["Blue"], "level": 3, "dp": 2000, "play_cost": 3}],
    })
    card = normalize_card("EX4-019", (observation,))
    assert card.fields["card_kind"] == "Digimon"
    assert card.faces[0].kind is FaceKind.DIGIMON
    assert card.printings
    assert card.provenance["card_kind"].selected_observation_id == observation.observation_id


@pytest.mark.parametrize("order", [[], ["BT25-043", "BT25-043"], ["BT1-001"]])
def test_explicit_shadow_order_requires_exact_card_universe(order):
    card = _dual_card()
    registry = StableCardRegistry(schema_version=1, registry_version=1,
        capacity=20000, padding_index=0, assignments={card.card_id: 23})
    with pytest.raises(ValidationError, match="each eligible card exactly once"):
        generate_cards_json({card.card_id: card}, registry, card_order=order)


@pytest.mark.parametrize("marker", ["ー", "－", "—", "-"])
def test_printed_empty_text_marker_is_not_an_inherited_ability(marker):
    observation = replace(_observation(SourceName.OFFICIAL, DIGEST), extracted={
        "card_id": "BT26-103", "name": "Jupitermon: Wrath Mode", "card_type": "Digimon",
        "colors": ["Yellow"], "level": 7, "dp": 16000, "play_cost": 16,
        "text_sections": [{"label": "Inherited Effect", "text": marker}],
        "faces": [{"face_id": "primary", "kind": "digimon", "colors": ["Yellow"],
                   "text_sections": [{"label": "Inherited Effect", "text": marker}]}],
    })
    card = normalize_card("BT26-103", (observation,))
    assert card.fields["inherited_effect_description_eng"] == ""
    assert card.faces[0].inherited_text == ""


def test_empty_marker_normalization_preserves_negative_dp_and_hyphenated_text():
    from tools.card_data.normalization import _normalize_printed_effect_text
    text = "[All Turns] A face-up Digimon gets -15000 DP."
    assert _normalize_printed_effect_text(text) == text
