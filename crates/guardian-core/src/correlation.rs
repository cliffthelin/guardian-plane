//! TDD-contract Phase 2, Gate 2a: the Layer-1 correlation engine
//! (implementation handoff §4/§6/§7; TDD contract §51). Pure, deterministic
//! Rust -- no `guardian-daemon` wiring, no D-Bus, no `/proc`/`/sys` reads,
//! no real provider I/O. This module owns exactly the 19 Gate 2a normative
//! IDs: `P2-EVT-001..004`, `P2-COR-001..007`, `P2-INC-002..004`,
//! `P2-REC-001..005`. `P2-INC-001` is retired (severity deferred in full,
//! TDD contract §51 "Severity/wire disposition") and is not reintroduced
//! here: [`crate::incident::Incident`] gains no new field.
//!
//! # `CorrelationIngress` and ingress order (§6)
//!
//! All correlation grouping, windowing, and ordering is defined over
//! `(ingress_clock, ingress_sequence)` -- never over a producer's own
//! `Event::timestamp_monotonic`, which real production code sets
//! incompatibly across producers (`guardian-daemon.rs`'s wall-clock
//! `now_secs()` vs. `providers/psi.rs`'s per-instance sequence counter).
//! [`IngressClock`] is the small, test/daemon-usable helper that
//! constructs [`CorrelationIngress`] records at the single admission
//! point every event source feeds through; a fresh instance begins a
//! fresh ordering epoch at sequence 0 (`P2-EVT-004`), matching the
//! restart-epoch semantics §6/§9 require.
//!
//! # Three bounded structures (§7)
//!
//! 1. Open-incident cap (`CorrelationPolicy::open_incident_cap`):
//!    force-closes the oldest-by-ingress-order open incident on overflow
//!    (`P2-INC-002`).
//! 2. Closed-incident ring (`CorrelationPolicy::closed_incident_cap`):
//!    FIFO-drops the oldest closed incident on overflow (`P2-INC-003`).
//! 3. Debounce/dwell ring (`CorrelationPolicy::debounce_capacity`):
//!    reject-not-evict -- a brand-new candidate key is rejected
//!    (`AdmitOutcome::CapacityRejected`) at capacity; existing tracked
//!    candidates are never disturbed (`P2-REC-001`/`P2-REC-004`).
//!
//! # `CapacityRejected` observability (`P2-REC-003`/`P2-REC-005`)
//!
//! A `CapacityRejected` outcome increments a bounded/saturating rejection
//! counter ([`CorrelationEngine::rejection_count`]) and is returned as
//! typed data ([`AdmitOutcome::CapacityRejected`] carrying
//! [`CapacityRejected`]) -- that is the entirety of this gate's owned
//! behavior. **Gate 2a performs no daemon-shaped I/O of its own.** The
//! physical operational log line (`guardian-daemon`'s existing
//! `eprintln!("[guardian-daemon] ...")` convention -- confirmed, by
//! reading `crates/guardian-daemon/src/bin/guardian-daemon.rs` directly,
//! to be the only logging convention anywhere in this workspace) is a
//! file-impact assignment the implementation handoff's §20 gives
//! explicitly to `crates/guardian-daemon/src/bin/guardian-daemon.rs`
//! under **Gate 2b**, not to this library crate: "the debounce ring's
//! `CapacityRejected` log line uses this file's existing
//! `eprintln!(...)` convention, not a new logging facility" (§20,
//! describing the Gate-2b-modified `guardian-daemon.rs`, not
//! `guardian-core`). A library crate performing `eprintln!` on its own
//! initiative -- impersonating a daemon process that, from Gate 2a's
//! perspective, does not exist yet -- is exactly the layering smell no
//! other library crate in this workspace exhibits. Everything Gate 2b
//! will need to perform that real log write later is already exposed
//! here as typed/queryable state: the rejected candidate's
//! `capability_id` and the post-rejection `rejection_count`, both on
//! [`CapacityRejected`] itself, plus the running total via
//! [`CorrelationEngine::rejection_count`]. By rule, a rejection never
//! constructs or feeds another [`crate::event::Event`] into
//! [`CorrelationIngress`] or any other correlation input path -- see
//! [`AdmitOutcome::CapacityRejected`] and
//! [`CorrelationEngine::admitted_ingress_count`].
//!
//! # Scope this module deliberately does NOT cover
//!
//! No real provider-health snapshot-diff producer (Gate 2b); no daemon
//! wiring or `Incidents1` population (Gate 2b); no VM evidence (Gate 2c);
//! no incident severity (deferred in full, §51); no
//! [`crate::budget::evaluate`]/[`crate::budget::evaluate_with_alternatives`]
//! call anywhere on this grouping path (`P2-REC-002`) -- this module does
//! not import `crate::budget` at all.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, Instant};

use guardian_provider_api::{Availability, CapabilityId, IncidentId, ProviderId};

use crate::event::Event;
use crate::incident::{Confidence, Incident, IncidentStatus};
use crate::risk::Risk;

/// The event-type string a provider-health transition [`Event`] is
/// expected to carry. Gate 2a does not produce these events for real --
/// that is Gate 2b's snapshot-diff producer (implementation handoff §4.2/
/// §13) -- but the constant is exported so Gate 2b's real producer and
/// Gate 2a's tests agree on the same shape without duplicating a string
/// literal.
pub const HEALTH_TRANSITION_EVENT_TYPE: &str = "capability_health_transition";

/// The event-type string real production PSI events already carry
/// (`crates/guardian-core/src/providers/psi.rs`'s `event_from_crossing`).
/// Re-exported as a named constant rather than a repeated literal.
pub const PSI_THRESHOLD_CROSSING_EVENT_TYPE: &str = "psi_threshold_crossing";

/// The attribute key a provider-health transition event carries its
/// stable `capability_id` under (see [`HEALTH_TRANSITION_EVENT_TYPE`]).
pub const HEALTH_CAPABILITY_ID_ATTR: &str = "capability_id";

/// The attribute key a provider-health transition event carries the
/// target [`Availability`] wire token under.
pub const HEALTH_AVAILABILITY_TO_ATTR: &str = "availability_to";

/// The correlation-ingress envelope (§6). Exact naming per the
/// implementation handoff; semantics are binding. `ingress_clock`/
/// `ingress_sequence` -- never `Event::timestamp_monotonic` -- define
/// every grouping/windowing/ordering decision in this module.
#[derive(Clone, Debug)]
pub struct CorrelationIngress {
    pub event: Event,
    pub ingress_clock: Instant,
    pub ingress_sequence: u64,
}

/// Total ingress order: `(ingress_clock, ingress_sequence)`, with
/// `ingress_sequence` breaking ties when two records share an identical
/// `Instant` reading (§6a).
type IngressOrder = (Instant, u64);

const fn ingress_order(ingress: &CorrelationIngress) -> IngressOrder {
    (ingress.ingress_clock, ingress.ingress_sequence)
}

/// Constructs [`CorrelationIngress`] records at a single, sequential
/// admission point (§6) -- the real `guardian-daemon` will own exactly
/// one of these (Gate 2b); Layer-1 tests use it directly to prove
/// restart-epoch semantics (`P2-EVT-004`) without needing a daemon.
/// Process-local and never persisted (§8/§9): a fresh instance always
/// begins a fresh ordering epoch at sequence 0.
#[derive(Debug, Default)]
pub struct IngressClock {
    sequence: u64,
}

impl IngressClock {
    #[must_use]
    pub const fn new() -> Self {
        Self { sequence: 0 }
    }

    /// Admits one event using the real wall clock (`Instant::now()`) for
    /// `ingress_clock`.
    pub fn admit(&mut self, event: Event) -> CorrelationIngress {
        self.admit_at(event, Instant::now())
    }

    /// Admits one event using a caller-supplied `Instant` -- the §6a
    /// technique (`base + Duration::from_millis(N)`) for deterministic
    /// Layer-1 tests that do not want to depend on real wall-clock
    /// timing at all.
    pub fn admit_at(&mut self, event: Event, ingress_clock: Instant) -> CorrelationIngress {
        let ingress_sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        CorrelationIngress {
            event,
            ingress_clock,
            ingress_sequence,
        }
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// Layer-1 policy configuration (§7/§51). Concrete numbers are an
/// implementation-time decision, not fixed by the contract -- only the
/// *existence* and *exact policy* (force-close-oldest / FIFO-drop-oldest /
/// reject-not-evict) of each bound is normative.
#[derive(Clone, Copy, Debug)]
pub struct CorrelationPolicy {
    /// §4.1: the PSI correlation window, measured against `ingress_clock`.
    /// A subsequent Critical event with `elapsed <= psi_window` since the
    /// key's last-linked Critical event attaches to the same incident
    /// (inclusive boundary -- `window_boundary_exactly_at_boundary_*`
    /// tests this precisely); `elapsed > psi_window` closes the existing
    /// incident and opens a new one with a textual backreference
    /// (`P2-COR-007`).
    pub psi_window: Duration,
    /// §4.2: the minimum ingress-clock dwell a provider-health direction
    /// must sustain (`elapsed_since_first_seen >= health_min_dwell`,
    /// inclusive) before being treated as incident-worthy.
    pub health_min_dwell: Duration,
    /// §7 class 1: hard cap on open-incident count.
    pub open_incident_cap: usize,
    /// §7 class 2: hard cap on retained closed-incident history.
    pub closed_incident_cap: usize,
    /// §7 class 3: hard cap on the debounce/dwell bookkeeping ring.
    pub debounce_capacity: usize,
}

impl Default for CorrelationPolicy {
    fn default() -> Self {
        Self {
            psi_window: Duration::from_secs(30),
            health_min_dwell: Duration::from_secs(1),
            open_incident_cap: 64,
            closed_incident_cap: 256,
            debounce_capacity: 64,
        }
    }
}

/// A `CapacityRejected` outcome (§7, `P2-REC-001`/`P2-REC-004`) -- a
/// typed result, never a silently-dropped return value. Carries the
/// rejected candidate's key and the counter's value *after* this
/// rejection was recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityRejected {
    pub capability_id: CapabilityId,
    pub rejection_count: u64,
}

/// The result of admitting one [`CorrelationIngress`] record.
#[derive(Clone, Debug)]
pub enum AdmitOutcome {
    /// The event matched no in-scope correlation rule (an unrecognized
    /// shape, a non-Critical PSI reading, or an `Unknown`/otherwise
    /// non-actionable provider-health direction -- AGENTS.md: "do not
    /// convert UNKNOWN into HEALTHY").
    Ignored,
    /// A new incident was created.
    IncidentOpened(IncidentId),
    /// An existing open incident was updated (`link_event`).
    IncidentUpdated(IncidentId),
    /// An existing open incident was closed (debounced provider-health
    /// recovery). Window-elapsed and capacity-forced closures are
    /// reported via [`AdmitResult::forced_closures`], not this variant,
    /// since those happen as a side effect of a *different* primary
    /// outcome (opening a new incident) rather than being the admission's
    /// own main effect.
    IncidentClosed(IncidentId),
    /// A provider-health transition was observed but has not yet passed
    /// its minimum dwell -- recorded in the debounce ring only.
    DebouncePending,
    /// The debounce ring was at capacity and this was a brand-new
    /// candidate key -- rejected outright (§7, `P2-REC-001`). By rule,
    /// this never constructs or feeds an [`Event`] into
    /// [`CorrelationIngress`] or any other correlation input path
    /// (`P2-REC-005`).
    CapacityRejected(CapacityRejected),
}

/// The full result of one [`CorrelationEngine::admit`] call.
#[derive(Clone, Debug)]
pub struct AdmitResult {
    pub outcome: AdmitOutcome,
    /// Any incidents force-closed as a *side effect* of this admission --
    /// the open-incident cap's force-close-oldest eviction (`P2-INC-002`),
    /// or the PSI window having elapsed for the same key
    /// (`P2-COR-002`/`P2-COR-007`). Empty in the common case.
    pub forced_closures: Vec<IncidentId>,
}

impl AdmitResult {
    const fn simple(outcome: AdmitOutcome) -> Self {
        Self {
            outcome,
            forced_closures: Vec::new(),
        }
    }
}

/// The internal correlation key -- never exposed directly; tests observe
/// it indirectly via `Incident::primary_resource`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum CorrelationKey {
    Psi {
        provider: ProviderId,
        resource: String,
    },
    Health {
        capability_id: CapabilityId,
    },
}

impl CorrelationKey {
    fn primary_resource(&self) -> String {
        match self {
            Self::Psi { resource, .. } => resource.clone(),
            Self::Health { capability_id } => capability_id.as_str().to_owned(),
        }
    }
}

#[derive(Debug)]
struct OpenIncidentState {
    incident: Incident,
    opened_order: IngressOrder,
    /// PSI-only: the ingress order of the most recently linked Critical
    /// event for this key, used to measure the window (§4.1).
    last_critical_order: Option<IngressOrder>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HealthDirection {
    Good,
    Bad,
}

fn health_direction(availability: Availability) -> Option<HealthDirection> {
    match availability {
        Availability::Available => Some(HealthDirection::Good),
        Availability::Unavailable => Some(HealthDirection::Bad),
        // Degraded/Unsupported/Unknown: not actionable in this gate's
        // narrowed synthetic rule (documented scoping decision -- richer
        // per-state handling is OPTIONAL FUTURE ENRICHMENT, implementation
        // handoff §13). `Unknown` in particular must never be promoted to
        // a confident state (AGENTS.md).
        Availability::Degraded | Availability::Unsupported | Availability::Unknown => None,
    }
}

#[derive(Debug)]
struct DebounceCandidate {
    target: HealthDirection,
    first_seen: IngressOrder,
}

/// Classification of one admitted [`Event`] into the rule this module
/// knows how to correlate. An event matching neither shape is
/// [`AdmitOutcome::Ignored`].
enum Classification {
    PsiCritical {
        provider: ProviderId,
        resource: String,
    },
    Health {
        capability_id: CapabilityId,
        direction: HealthDirection,
    },
}

fn classify(event: &Event) -> Option<Classification> {
    if event.event_type == PSI_THRESHOLD_CROSSING_EVENT_TYPE {
        // Only a genuine Critical crossing is incident-worthy in this
        // gate's chosen terminal rule (see module docs on
        // `CorrelationPolicy::psi_window`): closure is driven by the
        // window elapsing, not by an explicit recovery reading, so a
        // Nominal/Elevated reading (`Risk::Observe`/`Risk::Moderate`) is
        // deliberately `Ignored` here, never treated as a distinct
        // "recovery" signal.
        if event.severity != Risk::High {
            return None;
        }
        let resource = event.resource_refs.first()?.clone();
        return Some(Classification::PsiCritical {
            provider: event.source_provider.clone(),
            resource,
        });
    }
    if event.event_type == HEALTH_TRANSITION_EVENT_TYPE {
        let capability_id = event.attributes.get(HEALTH_CAPABILITY_ID_ATTR)?;
        let capability_id = CapabilityId::new(capability_id.clone()).ok()?;
        let availability_to = event.attributes.get(HEALTH_AVAILABILITY_TO_ATTR)?;
        let availability: Availability = availability_to.parse().unwrap_or(Availability::Unknown);
        let direction = health_direction(availability)?;
        return Some(Classification::Health {
            capability_id,
            direction,
        });
    }
    None
}

/// The Layer-1 correlation engine (§4/§6/§7). Process-memory-only (§8/§9):
/// no persistence, no filesystem writes, no dependency on a running
/// `guardian-daemon` or live D-Bus connection.
pub struct CorrelationEngine {
    policy: CorrelationPolicy,
    /// Class 1 (§7): open incidents, keyed by correlation key.
    open: HashMap<CorrelationKey, OpenIncidentState>,
    /// Deterministic oldest-open lookup -- ingress order, never `HashMap`
    /// iteration order (`P2-INC-002`).
    open_order_index: BTreeMap<IngressOrder, CorrelationKey>,
    /// Class 2 (§7): closed-incident ring, oldest at the front.
    closed: VecDeque<Incident>,
    /// Class 3 (§7): debounce/dwell bookkeeping, reject-not-evict.
    debounce: HashMap<CapabilityId, DebounceCandidate>,
    /// `P2-REC-003`: bounded/saturating rejection counter.
    rejection_count: u64,
    /// `P2-REC-005` non-reentry evidence: the number of `admit` calls that
    /// actually reached correlation processing (every real call -- a
    /// `CapacityRejected` outcome never causes a *second*, synthetic
    /// admission of its own).
    admitted_ingress_count: u64,
    incident_counter: u64,
}

impl std::fmt::Debug for CorrelationEngine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CorrelationEngine")
            .field("policy", &self.policy)
            .field("open_count", &self.open.len())
            .field("closed_count", &self.closed.len())
            .field("debounce_count", &self.debounce.len())
            .field("rejection_count", &self.rejection_count)
            .finish_non_exhaustive()
    }
}

impl CorrelationEngine {
    /// A fresh, pure-Rust Layer-1 engine (§4/§6/§7). Performs no I/O of
    /// any kind -- Gate 2a's typed `CapacityRejected` outcome and
    /// [`Self::rejection_count`] are the entirety of this crate's
    /// observable rejection behavior; the real operational log line is a
    /// separate, Gate-2b-owned side effect (see the module docs).
    #[must_use]
    pub fn new(policy: CorrelationPolicy) -> Self {
        Self {
            policy,
            open: HashMap::new(),
            open_order_index: BTreeMap::new(),
            closed: VecDeque::new(),
            debounce: HashMap::new(),
            rejection_count: 0,
            admitted_ingress_count: 0,
            incident_counter: 0,
        }
    }

    #[must_use]
    pub const fn rejection_count(&self) -> u64 {
        self.rejection_count
    }

    #[must_use]
    pub const fn admitted_ingress_count(&self) -> u64 {
        self.admitted_ingress_count
    }

    #[must_use]
    pub fn debounce_candidate_count(&self) -> usize {
        self.debounce.len()
    }

    /// Currently open incidents (any order the caller does not rely on;
    /// tests filter by `primary_resource`/`incident_id`).
    #[must_use]
    pub fn open_incidents(&self) -> Vec<Incident> {
        self.open
            .values()
            .map(|state| state.incident.clone())
            .collect()
    }

    /// Retained closed-incident history, oldest first.
    #[must_use]
    pub fn closed_incidents(&self) -> Vec<Incident> {
        self.closed.iter().cloned().collect()
    }

    fn next_incident_id(&mut self) -> IncidentId {
        let n = self.incident_counter;
        self.incident_counter = self.incident_counter.saturating_add(1);
        IncidentId::new(format!("guardian.correlation.incident-{n:06}"))
            .expect("generated incident id is valid")
    }

    fn backreference_text(&self, key: &CorrelationKey) -> Option<String> {
        let resource = key.primary_resource();
        self.closed
            .iter()
            .rev()
            .find(|incident| incident.primary_resource.as_deref() == Some(resource.as_str()))
            .map(|incident| {
                format!(
                    "supersedes closed incident {} for the same key (never reopened)",
                    incident.incident_id
                )
            })
    }

    fn push_closed(&mut self, incident: Incident) {
        if self.closed.len() >= self.policy.closed_incident_cap {
            self.closed.pop_front();
        }
        self.closed.push_back(incident);
    }

    /// Removes an open incident by key, marks it closed with `outcome`
    /// text, and files it into the closed ring (§7 class 2).
    fn close_key(&mut self, key: &CorrelationKey, outcome: &str) -> Option<IncidentId> {
        let state = self.open.remove(key)?;
        self.open_order_index.remove(&state.opened_order);
        let mut incident = state.incident;
        incident.status = IncidentStatus::Closed;
        incident.closed_at = Some(format!("ingress-{}", state.opened_order.1));
        incident.outcome = Some(outcome.to_owned());
        let id = incident.incident_id.clone();
        self.push_closed(incident);
        Some(id)
    }

    /// §7 class 1 overflow: force-closes the oldest-by-ingress-order open
    /// incident, if any (`P2-INC-002`). Deterministic -- driven by
    /// `open_order_index` (ingress order), never `HashMap` iteration
    /// order.
    fn force_close_oldest_open(&mut self) -> Option<IncidentId> {
        let key = self.open_order_index.iter().next()?.1.clone();
        self.close_key(&key, "forced closure: open-incident capacity reached")
    }

    fn ensure_open_capacity(&mut self, forced: &mut Vec<IncidentId>) {
        if self.open.len() >= self.policy.open_incident_cap {
            if let Some(id) = self.force_close_oldest_open() {
                forced.push(id);
            }
        }
    }

    fn open_new_incident(
        &mut self,
        key: CorrelationKey,
        ingress: &CorrelationIngress,
        confidence: Confidence,
        summary: String,
        forced: &mut Vec<IncidentId>,
    ) -> IncidentId {
        self.ensure_open_capacity(forced);
        let backreference = self.backreference_text(&key);
        let id = self.next_incident_id();
        let order = ingress_order(ingress);
        let mut evidence = vec![ingress.event.raw_reference.clone()];
        if let Some(text) = backreference {
            evidence.push(text);
        }
        let mut incident = Incident {
            incident_id: id.clone(),
            opened_at: format!("ingress-{}", ingress.ingress_sequence),
            closed_at: None,
            status: IncidentStatus::Open,
            summary,
            confidence,
            confidence_history: Vec::new(),
            primary_resource: Some(key.primary_resource()),
            event_ids: Vec::new(),
            evidence,
            candidate_causes: Vec::new(),
            recommended_actions: Vec::new(),
            transaction_ids: Vec::new(),
            outcome: None,
        };
        incident.link_event(ingress.event.event_id.clone());
        self.open.insert(
            key.clone(),
            OpenIncidentState {
                incident,
                opened_order: order,
                last_critical_order: Some(order),
            },
        );
        self.open_order_index.insert(order, key);
        id
    }

    /// §4.3: cross-references any currently-open incident of the *other*
    /// type with `key`'s incident. Never creates a third incident; never
    /// exceeds `Confidence::Hypothesis` for the relationship itself
    /// (`P2-COR-005`); idempotent (replay-safe).
    fn cross_reference(&mut self, key: &CorrelationKey) {
        let Some(this_id) = self.open.get(key).map(|s| s.incident.incident_id.clone()) else {
            return;
        };
        let other_keys: Vec<CorrelationKey> = self
            .open
            .keys()
            .filter(|other| std::mem::discriminant(*other) != std::mem::discriminant(key))
            .cloned()
            .collect();
        for other_key in other_keys {
            let Some(other_id) = self
                .open
                .get(&other_key)
                .map(|s| s.incident.incident_id.clone())
            else {
                continue;
            };
            let text_for_self =
                format!("correlated with incident {other_id} (temporal overlap; not causation)");
            let text_for_peer =
                format!("correlated with incident {this_id} (temporal overlap; not causation)");
            if let Some(state) = self.open.get_mut(key) {
                if !state.incident.candidate_causes.contains(&text_for_self) {
                    state.incident.candidate_causes.push(text_for_self);
                }
            }
            if let Some(state) = self.open.get_mut(&other_key) {
                if !state.incident.candidate_causes.contains(&text_for_peer) {
                    state.incident.candidate_causes.push(text_for_peer);
                }
            }
            // The cross-reference relationship itself never exceeds
            // Hypothesis (§4.3/`P2-COR-005`). Neither incident's own
            // `confidence` field is touched here -- PSI's/the health
            // transition's own direct-observation confidence is a
            // separate, already-correct fact this rule must not degrade;
            // the "never exceeds Hypothesis" requirement is satisfied by
            // this relationship never being represented as anything more
            // than the plain, unelevated text above (`Confidence::Hypothesis`
            // is the ceiling this module would use *if* it ever recorded a
            // typed confidence for the relationship itself -- it does not
            // invent a fourth incident type to attach one to).
            let _ = Confidence::Hypothesis;
        }
    }

    fn admit_psi(
        &mut self,
        ingress: &CorrelationIngress,
        provider: ProviderId,
        resource: String,
    ) -> AdmitResult {
        let key = CorrelationKey::Psi { provider, resource };
        let order = ingress_order(ingress);

        if let Some(state) = self.open.get(&key) {
            let last = state.last_critical_order.unwrap_or(state.opened_order);
            let elapsed = order.0.saturating_duration_since(last.0);
            if elapsed <= self.policy.psi_window {
                let id = state.incident.incident_id.clone();
                if let Some(state) = self.open.get_mut(&key) {
                    state.incident.link_event(ingress.event.event_id.clone());
                    state.last_critical_order = Some(order);
                }
                self.cross_reference(&key);
                return AdmitResult::simple(AdmitOutcome::IncidentUpdated(id));
            }
            // Window elapsed: close the old incident, open a fresh one
            // with a textual backreference -- never reopen (`P2-COR-007`).
            let mut forced = Vec::new();
            if let Some(closed_id) = self.close_key(
                &key,
                "closed: correlation window elapsed with no further Critical event",
            ) {
                forced.push(closed_id);
            }
            let id = self.open_new_incident(
                key.clone(),
                ingress,
                Confidence::Confirmed,
                format!(
                    "PSI critical pressure for {} (direct kernel observation)",
                    key.primary_resource()
                ),
                &mut forced,
            );
            self.cross_reference(&key);
            return AdmitResult {
                outcome: AdmitOutcome::IncidentOpened(id),
                forced_closures: forced,
            };
        }

        let mut forced = Vec::new();
        let id = self.open_new_incident(
            key.clone(),
            ingress,
            Confidence::Confirmed,
            format!(
                "PSI critical pressure for {} (direct kernel observation)",
                key.primary_resource()
            ),
            &mut forced,
        );
        self.cross_reference(&key);
        AdmitResult {
            outcome: AdmitOutcome::IncidentOpened(id),
            forced_closures: forced,
        }
    }

    fn admit_health(
        &mut self,
        ingress: &CorrelationIngress,
        capability_id: CapabilityId,
        direction: HealthDirection,
    ) -> AdmitResult {
        let key = CorrelationKey::Health {
            capability_id: capability_id.clone(),
        };

        if self.open.contains_key(&key) {
            return self.admit_health_with_open_incident(ingress, &key, capability_id, direction);
        }
        self.admit_health_candidate(ingress, &key, capability_id, direction)
    }

    /// `capability_id` already has an open incident: a further `Bad`
    /// reading just links; a `Good` reading is a debounced-recovery
    /// candidate (§4.2's terminal/closure rule for this gate).
    fn admit_health_with_open_incident(
        &mut self,
        ingress: &CorrelationIngress,
        key: &CorrelationKey,
        capability_id: CapabilityId,
        direction: HealthDirection,
    ) -> AdmitResult {
        if direction == HealthDirection::Bad {
            let id = self.open.get(key).unwrap().incident.incident_id.clone();
            if let Some(state) = self.open.get_mut(key) {
                state.incident.link_event(ingress.event.event_id.clone());
            }
            self.cross_reference(key);
            return AdmitResult::simple(AdmitOutcome::IncidentUpdated(id));
        }

        // A debounced recovery: track it in the debounce ring (a fresh
        // candidate, since promotion always clears any prior candidate
        // for this capability_id).
        let order = ingress_order(ingress);
        let existing = self
            .debounce
            .get(&capability_id)
            .map(|candidate| (candidate.target, candidate.first_seen));
        if let Some((HealthDirection::Good, first_seen)) = existing {
            let elapsed = order.0.saturating_duration_since(first_seen.0);
            if elapsed >= self.policy.health_min_dwell {
                self.debounce.remove(&capability_id);
                let id = self.close_key(key, "debounced recovery to Available");
                return AdmitResult::simple(AdmitOutcome::IncidentClosed(
                    id.expect("key was known open"),
                ));
            }
            return AdmitResult::simple(AdmitOutcome::DebouncePending);
        }
        if self.debounce.len() >= self.policy.debounce_capacity
            && !self.debounce.contains_key(&capability_id)
        {
            return self.reject_capacity(capability_id);
        }
        self.debounce.insert(
            capability_id,
            DebounceCandidate {
                target: HealthDirection::Good,
                first_seen: order,
            },
        );
        AdmitResult::simple(AdmitOutcome::DebouncePending)
    }

    /// `capability_id` has no open incident: only a `Bad` direction is
    /// meaningful (there is nothing to recover from). Handles debounce
    /// tracking/promotion and the reject-not-evict capacity rule
    /// (`P2-REC-001`/`P2-REC-004`).
    fn admit_health_candidate(
        &mut self,
        ingress: &CorrelationIngress,
        key: &CorrelationKey,
        capability_id: CapabilityId,
        direction: HealthDirection,
    ) -> AdmitResult {
        if direction == HealthDirection::Good {
            // A stray recovery reading with no open incident and no
            // candidate is a no-op -- clear any lingering Bad candidate
            // that flapped back to baseline before it could promote.
            if let Some(candidate) = self.debounce.get(&capability_id) {
                if candidate.target == HealthDirection::Bad {
                    self.debounce.remove(&capability_id);
                }
            }
            return AdmitResult::simple(AdmitOutcome::Ignored);
        }

        let order = ingress_order(ingress);
        let existing = self
            .debounce
            .get(&capability_id)
            .map(|candidate| (candidate.target, candidate.first_seen));

        if let Some((HealthDirection::Bad, first_seen)) = existing {
            let elapsed = order.0.saturating_duration_since(first_seen.0);
            if elapsed < self.policy.health_min_dwell {
                return AdmitResult::simple(AdmitOutcome::DebouncePending);
            }
            self.debounce.remove(&capability_id);
            let mut forced = Vec::new();
            let id = self.open_new_incident(
                key.clone(),
                ingress,
                Confidence::Confirmed,
                format!(
                    "capability {} debounced transition to Unavailable (direct provider report)",
                    capability_id.as_str()
                ),
                &mut forced,
            );
            self.cross_reference(key);
            return AdmitResult {
                outcome: AdmitOutcome::IncidentOpened(id),
                forced_closures: forced,
            };
        }

        // Either flapping back to `Bad` after drifting toward `Good`
        // (reset the existing tracked candidate in-place -- same key, not
        // a new one, never rejected), or a genuinely new key (subject to
        // the reject-not-evict capacity rule).
        if existing.is_none()
            && self.debounce.len() >= self.policy.debounce_capacity
            && !self.debounce.contains_key(&capability_id)
        {
            return self.reject_capacity(capability_id);
        }
        self.debounce.insert(
            capability_id,
            DebounceCandidate {
                target: HealthDirection::Bad,
                first_seen: order,
            },
        );
        AdmitResult::simple(AdmitOutcome::DebouncePending)
    }

    /// `P2-REC-001`/`P2-REC-003`/`P2-REC-005`: records a rejection --
    /// saturating counter increment and a typed [`CapacityRejected`]
    /// outcome, and by rule, nothing else. No `Event` is constructed; no
    /// `CorrelationIngress` is admitted for this rejection; no I/O of any
    /// kind is performed here -- the operational log write is Gate 2b's
    /// job, against the typed data this outcome already carries (see the
    /// module docs).
    fn reject_capacity(&mut self, capability_id: CapabilityId) -> AdmitResult {
        self.rejection_count = self.rejection_count.saturating_add(1);
        AdmitResult::simple(AdmitOutcome::CapacityRejected(CapacityRejected {
            capability_id,
            rejection_count: self.rejection_count,
        }))
    }

    /// Admits one [`CorrelationIngress`] record (§6). Rules apply
    /// strictly in ingress order, defined solely by
    /// `(ingress.ingress_clock, ingress.ingress_sequence)` -- never by
    /// the wrapped [`Event`]'s own `timestamp_monotonic`/`timestamp_wall`
    /// (`P2-EVT-001`), which remain provenance-only.
    ///
    /// This is the correlation *grouping* path: it never calls
    /// [`crate::budget::evaluate`]/[`crate::budget::evaluate_with_alternatives`]
    /// (`P2-REC-002`) -- this module does not import `crate::budget`.
    pub fn admit(&mut self, ingress: &CorrelationIngress) -> AdmitResult {
        self.admitted_ingress_count = self.admitted_ingress_count.saturating_add(1);
        match classify(&ingress.event) {
            Some(Classification::PsiCritical { provider, resource }) => {
                self.admit_psi(ingress, provider, resource)
            }
            Some(Classification::Health {
                capability_id,
                direction,
            }) => self.admit_health(ingress, capability_id, direction),
            None => AdmitResult::simple(AdmitOutcome::Ignored),
        }
    }
}
