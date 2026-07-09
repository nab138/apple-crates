#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeveloperAccount {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) profile_name: Option<String>,
    pub(crate) token_expires_at: Option<String>,
    pub(crate) token_expires_at_epoch_millis: Option<u64>,
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) teams: Vec<DeveloperTeam>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeveloperTeam {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) app_id_available_quantity: Option<u64>,
    pub(crate) app_id_max_quantity: Option<u64>,
    pub(crate) app_ids: Vec<DeveloperAppId>,
    pub(crate) certificates: Vec<DeveloperCertificate>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeveloperAppId {
    pub(crate) id: String,
    pub(crate) developer_id: String,
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) capabilities: Vec<DeveloperAppIdCapability>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeveloperAppIdCapability {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeveloperCertificate {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) serial_number: String,
    pub(crate) machine_name: String,
    pub(crate) private_key_available: bool,
    pub(crate) certificate_fingerprint: Option<String>,
    pub(crate) public_key_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeveloperDevice {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) udid: String,
}
