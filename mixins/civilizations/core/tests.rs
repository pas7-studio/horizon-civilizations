// Unit tests moved out of the crate root: `use super::super::*` still reaches
// every production item through core/mod.rs.
mod actor_identity_tests;
mod assigned_resource_courier_tests;
mod civic_council_tests;
mod civic_hall_warehouse_tests;
mod civic_sign_protection_tests;
mod domain_tests;
mod guarded_partial_native_actor_recovery_tests;
mod hall_sign_render_tests;
mod harvest_full_cycle_in_memory_tests;
mod harvest_receipt_tests;
mod legacy_witnessed_gather_migration_tests;
mod manual_warehouse_recovery_tests;
mod native_bakery_economy_tests;
mod native_barrel_transactions_tests;
mod native_edit_ack_and_fair_gather_tests;
mod native_forest_origin_tests;
mod native_gather_pickup_witness_tests;
mod native_gather_quarantine_tests;
mod native_worker_distance_contract_tests;
mod npc_daily_routine_tests;
mod physical_bakery_stock_control_tests;
mod physical_home_tests;
mod physical_logistics_hardening_tests;
mod safe_housing_proposal_tests;
mod site_sign_tests;
mod verified_short_oak_stem_tests;
mod village_chunk_tests;
mod village_land_and_autonomy_tests;
mod villager_dialogue_tests;
mod visual_farm_recovery_tests;
mod visual_farm_tests;
mod visual_farm_transition_tests;
mod work_post_tests;
mod workplace_staffing_tests;
mod world_host_clock_tests;

// Test modules live one level below the crate root; keep super::ITEM working.
pub(crate) use super::*;
