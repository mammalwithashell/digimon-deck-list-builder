use digimon_engine::{card_store::shared_card_store, debug_runner::make_test_card};
use std::collections::HashMap;
#[test]
fn equal_length_printed_name_changes_rekey_shared_card_store() {
    let mut original = make_test_card("CACHE-NAME-IDENTITY", "Greymon");
    let mut db = HashMap::from([(original.card_id.clone(), original.clone())]);
    let first = shared_card_store(&db);
    original.card_name = "OtherL6".into();
    db.insert(original.card_id.clone(), original);
    let second = shared_card_store(&db);
    assert_eq!(
        first.data[first.index["CACHE-NAME-IDENTITY"]].card_name,
        "Greymon"
    );
    assert_eq!(
        second.data[second.index["CACHE-NAME-IDENTITY"]].card_name,
        "OtherL6"
    );
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
    let repeat = shared_card_store(&db);
    assert!(std::sync::Arc::ptr_eq(&second, &repeat));
}
