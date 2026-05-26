//! Configuration types for the replication protocol.

/// Configuration for the Paxos consensus subsystem used in view changes.
///
/// These parameters control the leader election and heartbeat behavior
/// of the Paxos proposers that sequence view change proposals.
#[derive(Clone, Copy, Debug)]
pub struct PaxosConfig {
    /// Maximum number of faulty nodes tolerated by the Paxos acceptor group.
    pub f: usize,
    /// How often (in ticks) to send "I am leader" heartbeats.
    pub i_am_leader_send_timeout: u64,
    /// How often (in ticks) to check if the leader has expired.
    pub i_am_leader_check_timeout: u64,
    /// Initial delay multiplied by proposer ID to stagger timeout checks.
    pub i_am_leader_check_timeout_delay_multiplier: usize,
}

/// Top-level configuration for the transparent replication protocol.
///
/// Controls cluster membership, fault tolerance parameters, timing for
/// failure detection, and the Paxos subsystem used for view changes.
#[derive(Clone, Debug)]
pub struct ReplicateConfig {
    /// Initial cluster members (replica IDs). The first member (index 0) is
    /// the initial primary. Must contain at least `f + 1` members for the
    /// data path quorum.
    pub initial_members: Vec<u32>,
    /// Fault tolerance parameter. The system tolerates up to `f` simultaneous
    /// failures. Requires `2f + 1` replicas for Paxos acceptors and `f + 1`
    /// replicas for the data path.
    pub f: usize,
    /// Timeout in milliseconds before a backup suspects primary failure.
    /// If no commit notification is received within this window, the backup
    /// proposes a view change.
    pub commit_timeout_ms: u64,
    /// Interval in milliseconds between commit notification broadcasts from
    /// the primary. Should be significantly less than `commit_timeout_ms`
    /// (typically `commit_timeout_ms / 3`) to avoid false failure detections.
    pub notification_interval_ms: u64,
    /// Whether backups apply commands to maintain a hot standby.
    /// When `true`, backups call `apply()` on received commands, keeping
    /// their state up-to-date for fast failover. When `false`, backups
    /// only log commands without applying them.
    pub backup_apply: bool,
    /// Configuration for the Paxos consensus subsystem that sequences
    /// view change proposals.
    pub paxos_config: PaxosConfig,
}

impl Default for ReplicateConfig {
    fn default() -> Self {
        Self {
            initial_members: vec![0, 1, 2],
            f: 1,
            commit_timeout_ms: 5000,
            notification_interval_ms: 1666, // commit_timeout_ms / 3
            backup_apply: true,
            paxos_config: PaxosConfig {
                f: 1,
                i_am_leader_send_timeout: 5,
                i_am_leader_check_timeout: 10,
                i_am_leader_check_timeout_delay_multiplier: 15,
            },
        }
    }
}
