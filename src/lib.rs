pub mod consequences;
pub mod economy;
pub mod events;
pub mod history;
pub mod interactions;
pub mod ids;
pub mod invariants;
pub mod ledger;
pub mod merchant;
pub mod npc;
pub mod persistence;
pub mod property;
pub mod materials;
pub mod scenarios;
pub mod simulation;
pub mod snapshot;
pub mod structure_sync;
pub mod structures;
pub mod world;
pub mod world_action;

pub use consequences::{apply_world_action, ActionOutcome, Actor, WorldAction};
pub use history::{
    ancestors_of, canonical_history_digest, descendants_of, find_cause_for_event,
    summarize_bridge_collapse, trace_from_cause, validate_cause_graph, CausalLink,
};
pub use invariants::{
    check_no_aggregate_and_npc_double_consume, check_world_invariants, validate_world,
    IntegrityViolation, ValidationReport, ViolationSeverity,
};
pub use ledger::{
    reconcile_grain_conservation, transfer_settlement_grain, GrainConservationReport, GrainLedger,
    MoneyLedger,
};
pub use materials::{material, MaterialId};
pub use snapshot::{
    assert_snapshots_equal, canonical_snapshot, canonical_snapshot_from, snapshot_digest,
    CanonicalSnapshot,
};
pub use scenarios::{run_all_scenarios, ScenarioKind, ScenarioReport};
pub use structures::{
    apply_impact, apply_repair, apply_sustained_load, BridgeStructure, CrossingSafety,
    INTEGRITY_MAX,
};
