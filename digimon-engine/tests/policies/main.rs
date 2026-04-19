//! Policy integration tests. Covers the heuristic greedy opponent
//! (`greedy.rs`) and a self-play tripwire (`self_play.rs`) that catches
//! action-mask drift or policy off-by-ones by running many games to
//! conclusion across a range of seeds.

mod greedy;
mod self_play;
