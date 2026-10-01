//! Docker's registered execution preferences and compatibility migration.
use crate::config::registry::{Effect, Kind, Registry, SettingDefinition};
use anyhow::Result;
pub(super) fn default_apparmor() -> String {
    if std::fs::read_to_string("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
        .is_ok_and(|v| v.trim() == "1")
    {
        "oyzu-buildkit".into()
    } else {
        "unconfined".into()
    }
}
pub(super) fn register(registry: &mut Registry) -> Result<()> {
    registry.register(SettingDefinition {
        key: "docker.apparmorProfile".into(),
        kind: Kind::Name,
        default: Some(serde_json::json!(default_apparmor())),
        administrative: false,
        profile: true,
        effect: Effect::Execution,
        sensitive: false,
        set: false,
    })
}
