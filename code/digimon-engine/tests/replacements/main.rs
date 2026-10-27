// Match the DSL/card harnesses for allocation-heavy clone tests on Windows.
// This allocator is local to the test binary; production allocation is unchanged.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod attack_cancel;
mod behavioral_end_to_end;
mod context_predicates;
mod cross_permanent;
mod declined_replacements;
mod deletion_replacements;
mod deletion_event_identity;
mod deletion_event_routes;
mod dispatcher_core;
mod dispatcher_guard;
mod enum_and_context;
mod granted_keywords;
mod native_keywords;
mod nested_select_decoy;
mod nested_select_fragment;
mod nested_select_regression;
mod nested_select_save;
mod nested_select_substrate;
mod partition;
mod passive_modifier_migration;
mod route_replacements;
mod source_scoped_immunity;
