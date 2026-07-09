#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MachineIdentity {
    pub(crate) machine_name: String,
    pub(crate) os_name: String,
    pub(crate) os_version: String,
    pub(crate) machine_id: String,
}
