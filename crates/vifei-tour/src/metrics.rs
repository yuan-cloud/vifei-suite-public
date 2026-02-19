use serde::{Deserialize, Serialize};
use vifei_core::projection::ViewModel;
use vifei_core::reducer::State;

/// Metrics emitted by Tour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourMetrics {
    /// Projection invariants version.
    pub projection_invariants_version: String,
    /// Total number of events processed.
    pub event_count_total: usize,
    /// Tier A drops (must be 0 for CI pass).
    pub tier_a_drops: u64,
    /// Maximum degradation level reached.
    pub max_degradation_level: String,
    /// Final degradation level.
    pub degradation_level_final: String,
    /// Degradation transitions (ordered list).
    pub degradation_transitions: Vec<DegradationTransition>,
    /// Aggregation mode.
    pub aggregation_mode: String,
    /// Aggregation bin size (if applicable).
    pub aggregation_bin_size: Option<u64>,
    /// Queue pressure (normalized 0.0-1.0).
    pub queue_pressure: f64,
    /// Export safety state.
    pub export_safety_state: String,
}

/// A degradation level transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationTransition {
    /// Level before transition.
    pub from_level: String,
    /// Level after transition.
    pub to_level: String,
    /// What triggered the transition.
    pub trigger: String,
    /// Queue pressure at transition time.
    pub queue_pressure: f64,
}

/// Build deterministic Tour metrics from reduced state and projected view model.
pub(crate) fn build_metrics(
    state: &State,
    viewmodel: &ViewModel,
    committed_event_count: usize,
) -> TourMetrics {
    // Populate degradation_transitions from reducer's policy_decisions
    let degradation_transitions: Vec<DegradationTransition> = state
        .policy_decisions
        .iter()
        .map(|pd| DegradationTransition {
            from_level: pd.from_level.clone(),
            to_level: pd.to_level.clone(),
            trigger: pd.trigger.clone(),
            queue_pressure: pd.queue_pressure_micro as f64 / 1_000_000.0,
        })
        .collect();

    // Compute max degradation level from transitions + final level
    let final_level = format!("{}", viewmodel.degradation_level);
    let max_degradation_level = state
        .policy_decisions
        .iter()
        .map(|pd| pd.to_level.as_str())
        .chain(std::iter::once(final_level.as_str()))
        .max()
        .unwrap_or("L0")
        .to_string();

    TourMetrics {
        projection_invariants_version: viewmodel.projection_invariants_version.clone(),
        event_count_total: committed_event_count,
        tier_a_drops: viewmodel.tier_a_drops,
        max_degradation_level,
        degradation_level_final: final_level,
        degradation_transitions,
        aggregation_mode: viewmodel.aggregation_mode.clone(),
        aggregation_bin_size: viewmodel.aggregation_bin_size,
        queue_pressure: viewmodel.queue_pressure(),
        export_safety_state: format!("{}", viewmodel.export_safety_state),
    }
}
