//! Environment-variable and CLI-override coverage for the config resolver (CFG-02).
//!
//! `config_resolver_tests.rs` covers precedence; these tests drive every `ROCKSTREAM__...`
//! variable and every CLI override through `ConfigResolver::resolve`, including values that do
//! not parse, so no override path is exercised only in production.

use std::path::PathBuf;
use std::sync::Mutex;

use rockstream_types::config_resolver::{CliConfigOverrides, ConfigOrigin, ConfigResolver};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Removes the variables it set when dropped, so a failing assertion cannot leak them into other tests.
struct EnvGuard(Vec<&'static str>);

impl EnvGuard {
    fn set(vars: &[(&'static str, &str, &str)]) -> Self {
        let mut names = Vec::new();
        for (name, value, _) in vars {
            std::env::set_var(name, value);
            names.push(*name);
        }
        Self(names)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for name in &self.0 {
            std::env::remove_var(name);
        }
    }
}

/// (variable, value, origin key it must be recorded under)
const VALID_ENV: &[(&str, &str, &str)] = &[
    ("ROCKSTREAM__NODE__ROLE", "env-value", "node.role"),
    ("ROCKSTREAM__NODE__HOST_ID", "env-value", "node.host_id"),
    (
        "ROCKSTREAM__NODE__AVAILABILITY_ZONE",
        "env-value",
        "node.availability_zone",
    ),
    (
        "ROCKSTREAM__GATEWAY__LISTEN_ADDR",
        "env-value",
        "gateway.listen_addr",
    ),
    (
        "ROCKSTREAM__CONTROL__LISTEN_ADDR",
        "env-value",
        "control.listen_addr",
    ),
    ("ROCKSTREAM__CONTROL__URL", "env-value", "control.url"),
    ("ROCKSTREAM__WORKER__WORKER_ID", "7", "worker.worker_id"),
    (
        "ROCKSTREAM__WORKER__MEMORY_BUDGET_BYTES",
        "7",
        "worker.memory_budget_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__FOREGROUND_RESERVATION_BYTES",
        "7",
        "worker.foreground_reservation_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_COMPACTION_CONCURRENCY",
        "7",
        "worker.max_compaction_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_BACKFILL_CONCURRENCY",
        "7",
        "worker.max_backfill_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_MIGRATION_CONCURRENCY",
        "7",
        "worker.max_migration_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__DISK_CACHE_DIR",
        "env-value",
        "worker.disk_cache_dir",
    ),
    (
        "ROCKSTREAM__WORKER__DISK_CACHE_BYTES",
        "7",
        "worker.disk_cache_bytes",
    ),
    ("ROCKSTREAM__STORAGE__URL", "env-value", "storage.url"),
    (
        "ROCKSTREAM__METRICS__LISTEN_ADDR",
        "env-value",
        "metrics.listen_addr",
    ),
    ("ROCKSTREAM__AUTH__MODE", "env-value", "auth.mode"),
    ("ROCKSTREAM__LOGGING__LEVEL", "env-value", "logging.level"),
    ("ROCKSTREAM__LOGGING__FORMAT", "env-value", "logging.format"),
    (
        "ROCKSTREAM__RUNTIME__SHUTDOWN_TIMEOUT_SECS",
        "7",
        "runtime.shutdown_timeout_secs",
    ),
    (
        "ROCKSTREAM__RUNTIME__MIN_EPOCH_MS",
        "7",
        "runtime.min_epoch_ms",
    ),
    (
        "ROCKSTREAM__RUNTIME__CHECKPOINT_RETENTION_COUNT",
        "7",
        "runtime.checkpoint_retention_count",
    ),
    (
        "ROCKSTREAM__RUNTIME__STATE_BUDGET_GB",
        "7",
        "runtime.state_budget_gb",
    ),
    (
        "ROCKSTREAM__CLUSTER__MIN_EPOCH_MS",
        "7",
        "cluster.min_epoch_ms",
    ),
    (
        "ROCKSTREAM__CLUSTER__CHECKPOINT_RETENTION_COUNT",
        "7",
        "cluster.checkpoint_retention_count",
    ),
    (
        "ROCKSTREAM__CLUSTER__STATE_BUDGET_GB",
        "7",
        "cluster.state_budget_gb",
    ),
    (
        "ROCKSTREAM__CLUSTER__SHUTDOWN_TIMEOUT_SECS",
        "7",
        "cluster.shutdown_timeout_secs",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_DIRECT_THRESHOLD_BYTES",
        "7",
        "exchange.exchange_direct_threshold_bytes",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_SPILL_THRESHOLD_MB",
        "7",
        "exchange.exchange_spill_threshold_mb",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_FORCE_DURABLE",
        "true",
        "exchange.exchange_force_durable",
    ),
    (
        "ROCKSTREAM__WORKER__SEGMENT_CACHE_BYTES",
        "7",
        "worker.segment_cache_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_ROWS_PER_QUANTUM",
        "7",
        "worker.max_rows_per_quantum",
    ),
    (
        "ROCKSTREAM__WORKER__EXECUTION_THREADS",
        "7",
        "worker.execution_threads",
    ),
    (
        "ROCKSTREAM__RECURSION_MAX_ITERATIONS",
        "7",
        "recursion_max_iterations",
    ),
    (
        "ROCKSTREAM__EXECUTION__JOIN_STRATEGY",
        "classic",
        "execution.join_strategy",
    ),
];

/// Variables whose value must parse; (variable, unparseable value, origin key that must stay untouched)
const UNPARSEABLE_ENV: &[(&str, &str, &str)] = &[
    (
        "ROCKSTREAM__WORKER__WORKER_ID",
        "not-a-number",
        "worker.worker_id",
    ),
    (
        "ROCKSTREAM__WORKER__MEMORY_BUDGET_BYTES",
        "not-a-number",
        "worker.memory_budget_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__FOREGROUND_RESERVATION_BYTES",
        "not-a-number",
        "worker.foreground_reservation_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_COMPACTION_CONCURRENCY",
        "not-a-number",
        "worker.max_compaction_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_BACKFILL_CONCURRENCY",
        "not-a-number",
        "worker.max_backfill_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_MIGRATION_CONCURRENCY",
        "not-a-number",
        "worker.max_migration_concurrency",
    ),
    (
        "ROCKSTREAM__WORKER__DISK_CACHE_BYTES",
        "not-a-number",
        "worker.disk_cache_bytes",
    ),
    (
        "ROCKSTREAM__RUNTIME__SHUTDOWN_TIMEOUT_SECS",
        "not-a-number",
        "runtime.shutdown_timeout_secs",
    ),
    (
        "ROCKSTREAM__RUNTIME__MIN_EPOCH_MS",
        "not-a-number",
        "runtime.min_epoch_ms",
    ),
    (
        "ROCKSTREAM__RUNTIME__CHECKPOINT_RETENTION_COUNT",
        "not-a-number",
        "runtime.checkpoint_retention_count",
    ),
    (
        "ROCKSTREAM__RUNTIME__STATE_BUDGET_GB",
        "not-a-number",
        "runtime.state_budget_gb",
    ),
    (
        "ROCKSTREAM__CLUSTER__MIN_EPOCH_MS",
        "not-a-number",
        "cluster.min_epoch_ms",
    ),
    (
        "ROCKSTREAM__CLUSTER__CHECKPOINT_RETENTION_COUNT",
        "not-a-number",
        "cluster.checkpoint_retention_count",
    ),
    (
        "ROCKSTREAM__CLUSTER__STATE_BUDGET_GB",
        "not-a-number",
        "cluster.state_budget_gb",
    ),
    (
        "ROCKSTREAM__CLUSTER__SHUTDOWN_TIMEOUT_SECS",
        "not-a-number",
        "cluster.shutdown_timeout_secs",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_DIRECT_THRESHOLD_BYTES",
        "not-a-number",
        "exchange.exchange_direct_threshold_bytes",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_SPILL_THRESHOLD_MB",
        "not-a-number",
        "exchange.exchange_spill_threshold_mb",
    ),
    (
        "ROCKSTREAM__EXCHANGE__EXCHANGE_FORCE_DURABLE",
        "maybe",
        "exchange.exchange_force_durable",
    ),
    (
        "ROCKSTREAM__WORKER__SEGMENT_CACHE_BYTES",
        "not-a-number",
        "worker.segment_cache_bytes",
    ),
    (
        "ROCKSTREAM__WORKER__MAX_ROWS_PER_QUANTUM",
        "not-a-number",
        "worker.max_rows_per_quantum",
    ),
    (
        "ROCKSTREAM__WORKER__EXECUTION_THREADS",
        "not-a-number",
        "worker.execution_threads",
    ),
    (
        "ROCKSTREAM__RECURSION_MAX_ITERATIONS",
        "not-a-number",
        "recursion_max_iterations",
    ),
    (
        "ROCKSTREAM__EXECUTION__JOIN_STRATEGY",
        "sideways",
        "execution.join_strategy",
    ),
];

#[test]
fn every_environment_variable_is_applied_and_recorded() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env = EnvGuard::set(VALID_ENV);
    let resolved = ConfigResolver::resolve(None, &CliConfigOverrides::default()).expect("resolves");
    for (name, _, origin_key) in VALID_ENV {
        // `runtime.*` and `cluster.*` are aliases of each other, so which variable is recorded for
        // the shared setting depends on the order the environment is iterated in.
        assert!(
            matches!(
                resolved.origins.get(*origin_key),
                Some(ConfigOrigin::Environment(_))
            ),
            "{name} must make {origin_key} an environment-sourced setting"
        );
    }
    assert_eq!(resolved.node_config.node.role, "env-value");
    assert_eq!(
        resolved.node_config.node.host_id.as_deref(),
        Some("env-value")
    );
    assert_eq!(resolved.node_config.worker.max_rows_per_quantum, 7);
    assert!(resolved.config.exchange.exchange_force_durable);
}

#[test]
fn unparseable_environment_values_are_ignored() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env = EnvGuard::set(UNPARSEABLE_ENV);
    let resolved = ConfigResolver::resolve(None, &CliConfigOverrides::default()).expect("resolves");
    for (name, _, origin_key) in UNPARSEABLE_ENV {
        assert!(
            !matches!(
                resolved.origins.get(*origin_key),
                Some(ConfigOrigin::Environment(_))
            ),
            "{name} does not parse, so it must not become the origin of {origin_key}"
        );
    }
}

#[test]
fn unknown_environment_variables_are_ignored() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let vars = [
        ("ROCKSTREAM__NO_SUCH_SECTION__NO_SUCH_KEY", "1", ""),
        ("ROCKSTREAM__NODE__NO_SUCH_KEY", "1", ""),
    ];
    let _env = EnvGuard::set(&vars);
    let baseline = ConfigResolver::resolve(None, &CliConfigOverrides::default()).expect("resolves");
    assert!(!baseline
        .origins
        .values()
        .any(|origin| matches!(origin, ConfigOrigin::Environment(_))));
}

fn full_cli_overrides() -> CliConfigOverrides {
    CliConfigOverrides {
        role: Some("worker".into()),
        host_id: Some("host-cli".into()),
        availability_zone: Some("az-cli".into()),
        listen_addr: Some("127.0.0.1:15432".into()),
        gateway_max_connections: Some(11),
        gateway_query_timeout_secs: Some(12),
        control_bind: Some("127.0.0.1:17000".into()),
        management_addr: Some("127.0.0.1:17001".into()),
        control_url: Some("http://127.0.0.1:17000".into()),
        control_shared_storage: Some("memory://shared".into()),
        worker_id: Some(3),
        worker_threads: Some(2),
        worker_cache_bytes: Some(1 << 20),
        worker_quantum: Some(64),
        worker_memory_budget_bytes: Some(1 << 30),
        worker_foreground_reservation_bytes: Some(1 << 20),
        worker_max_compaction_concurrency: Some(2),
        worker_max_backfill_concurrency: Some(2),
        worker_max_migration_concurrency: Some(2),
        worker_disk_cache_dir: Some(PathBuf::from("/tmp/rockstream-cli-cache")),
        worker_disk_cache_bytes: Some(1 << 24),
        storage_url: Some("memory://cli".into()),
        storage_temp_dir: Some(PathBuf::from("/tmp/rockstream-cli-temp")),
        storage_spill_dir: Some(PathBuf::from("/tmp/rockstream-cli-spill")),
        metrics_addr: Some("127.0.0.1:19090".into()),
        metrics_enabled: Some(true),
        auth_mode: Some("password".into()),
        auth_secret_path: Some(PathBuf::from("/etc/rockstream/auth-secret")),
        log_level: Some("debug".into()),
        log_format: Some("json".into()),
        min_epoch_ms: Some(20),
        checkpoint_retention_count: Some(3),
        state_budget_gb: Some(4),
        exchange_direct_threshold_bytes: Some(4096),
        exchange_spill_threshold_mb: Some(8),
        exchange_domain_size: Some(16),
        exchange_force_durable: Some(true),
        same_host_shm_segment_bytes: Some(1 << 20),
        same_host_shm_segments_per_peer: Some(4),
        max_exchange_compression_states: Some(8),
        webhook_listen_addr: Some("127.0.0.1:18080".into()),
        tls_cert_path: Some(PathBuf::from("/etc/rockstream/tls.crt")),
        tls_key_path: Some(PathBuf::from("/etc/rockstream/private.key")),
        tls_ca_cert_path: Some(PathBuf::from("/etc/rockstream/ca.crt")),
        internal_tls_cert_path: Some(PathBuf::from("/etc/rockstream/internal.crt")),
        internal_tls_key_path: Some(PathBuf::from("/etc/rockstream/internal-private.key")),
        internal_tls_ca_cert_path: Some(PathBuf::from("/etc/rockstream/internal-ca.crt")),
        shutdown_timeout_secs: Some(30),
    }
}

#[test]
fn every_cli_override_is_applied_and_recorded() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let resolved = ConfigResolver::resolve(None, &full_cli_overrides()).expect("resolves");
    let cli_origins = resolved
        .origins
        .values()
        .filter(|origin| matches!(origin, ConfigOrigin::Cli(_)))
        .count();
    assert!(
        cli_origins >= 40,
        "expected most overrides recorded as CLI origins, got {cli_origins}"
    );
    assert_eq!(resolved.node_config.node.role, "worker");
    assert_eq!(
        resolved.node_config.node.host_id.as_deref(),
        Some("host-cli")
    );
    assert_eq!(resolved.node_config.worker.execution_threads, 2);
    assert_eq!(resolved.node_config.worker.max_rows_per_quantum, 64);
    assert_eq!(resolved.config.cluster.min_epoch_ms, 20);
    assert!(matches!(
        resolved.origins.get("node.role"),
        Some(ConfigOrigin::Cli(_))
    ));
}

#[test]
fn cli_overrides_beat_environment_variables() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _env = EnvGuard::set(VALID_ENV);
    let resolved = ConfigResolver::resolve(None, &full_cli_overrides()).expect("resolves");
    assert_eq!(resolved.node_config.node.role, "worker");
    assert!(matches!(
        resolved.origins.get("node.role"),
        Some(ConfigOrigin::Cli(_))
    ));
}

#[test]
fn secrets_are_redacted_and_origins_render() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let resolved = ConfigResolver::resolve(None, &full_cli_overrides()).expect("resolves");

    let node = resolved.redacted_node_config();
    assert_eq!(node.auth.secret_path, Some(PathBuf::from("[REDACTED]")));
    assert_eq!(node.gateway.tls.key_path, Some(PathBuf::from("[REDACTED]")));
    let legacy = resolved.redacted_config();
    assert_eq!(
        legacy.gateway.tls_key_path,
        Some(PathBuf::from("[REDACTED]"))
    );
    assert_eq!(
        legacy.internal_tls.key_path,
        Some(PathBuf::from("[REDACTED]"))
    );

    let plain = resolved.to_toml_text(false);
    let annotated = resolved.to_toml_text(true);
    assert!(!plain.contains("# origin:"));
    assert!(
        annotated.contains("# origin: cli("),
        "annotated output: {annotated}"
    );
    assert!(!annotated.contains("private.key"));
}
