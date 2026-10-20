#![cfg(all(feature = "dsl-yaml-loader", feature = "test-helpers"))]
#[path="support/named_material_kind_witness.rs"]mod witness;
use serde_json::json;
fn raw(id:&str,name:&str,kind:u8)->Vec<u8>{serde_json::to_vec(&json!({id:{"card_id":id,"card_name_eng":name,"card_kind":kind,"index":1,"card_colors":[0]}})).unwrap()}
#[test]fn zero_matches_must_remain_blocked(){let b=raw("REVIEW-ZERO","Other",0);let p=witness::prove(&b,&witness::sha(&b),&["Titamon"]).unwrap();assert_eq!(p["names"][0]["equal"],true);assert_eq!(p["names"][0]["zero_matches"],true);assert_eq!(p["set_correspondence_blocked"],true);}
#[test]fn target_names_are_exact_nonempty_unique(){let b=raw("REVIEW-NAMES","Titamon",0);for names in [vec![],vec![""],vec!["Titamon","Titamon"]]{assert!(witness::prove(&b,&witness::sha(&b),&names).is_err());}}
#[test]fn malformed_indices_refuse_before_load(){for value in [json!(false),json!(1.0),json!(-1),json!(0),json!(20000),json!("1")]{let mut v:serde_json::Value=serde_json::from_slice(&raw("REVIEW-INDEX","Titamon",0)).unwrap();v["REVIEW-INDEX"]["index"]=value;let b=serde_json::to_vec(&v).unwrap();assert!(witness::strict_records(&b,&witness::sha(&b)).is_err());}}
#[test]fn eggs_are_not_digimon_and_flags_stay_false(){let b=raw("REVIEW-EGG","Titamon",3);let p=witness::prove(&b,&witness::sha(&b),&["Titamon"]).unwrap();assert_eq!(p["names"][0]["equal"],false);assert_eq!(p["set_correspondence_blocked"],true);for k in ["source_authority","runtime_admission","executable_closure","engine_behavior_proven"]{assert_eq!(p[k],false);}}
