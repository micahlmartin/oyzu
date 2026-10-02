//! Language-owned runtime requirements for optional application packaging.
//! Acquisition and orchestration consume these facts without language dispatch.
pub(crate) struct ContainerProfile {
    pub id: &'static str,
    pub artifact: &'static str,
    pub base: &'static str,
    pub payload: &'static str,
    pub entrypoint: Vec<String>,
    /// An isolated probe of the provisioned runtime, not application execution.
    pub probe: Vec<String>,
    pub expected: String,
}
