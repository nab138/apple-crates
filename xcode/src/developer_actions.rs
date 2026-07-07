use plist::Dictionary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct EmptyResponse {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamID(String);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperTeam {
    pub name: String,
    pub team_id: TeamID,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Developer {
    pub developer_id: String,
    pub person_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub ds_first_name: String,
    pub ds_last_name: String,
    pub email: String,
    pub developer_status: String, // TODO: DeveloperStatus enum
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperView {
    pub teams: Vec<DeveloperTeam>,
    pub developer: Developer,
}

#[derive(Debug, Serialize)]
pub struct ViewDeveloperAction {}
impl_developer_action!(ViewDeveloperAction, "viewDeveloper.action", DeveloperView);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTeamsResponse {
    pub teams: Vec<DeveloperTeam>,
}

#[derive(Debug, Serialize)]
pub struct ListTeamsAction {}
impl_developer_action!(ListTeamsAction, "listTeams.action", ListTeamsResponse);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperDevice {
    pub device_id: String,
    pub name: String,
    pub device_number: String,
}

#[derive(Debug, Deserialize)]
pub struct ListDevicesResponse {
    pub devices: Vec<DeveloperDevice>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDevicesAction {
    pub team_id: TeamID,
}

impl_developer_action!(
    ListDevicesAction,
    "listDevices.action",
    ListDevicesResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
pub struct AddDeviceResponse {
    pub device: DeveloperDevice,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDeviceAction {
    pub team_id: TeamID,
    pub name: String,
    pub device_number: String,
}

impl_developer_action!(
    AddDeviceAction,
    "addDevice.action",
    AddDeviceResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDeviceAction {
    pub team_id: TeamID,
    pub device_id: String,
}

impl_developer_action!(
    DeleteDeviceAction,
    "deleteDevice.action",
    EmptyResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentCertificate {
    pub name: String,
    pub certificate_id: String,
    pub serial_number: String,
    pub cert_content: Vec<u8>,
    #[serde(default)]
    pub machine_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListAllDevelopmentCertsResponse {
    pub certificates: Vec<DevelopmentCertificate>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAllDevelopmentCertsAction {
    pub team_id: TeamID,
}

impl_developer_action!(
    ListAllDevelopmentCertsAction,
    "listAllDevelopmentCerts.action",
    ListAllDevelopmentCertsResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeDevelopmentCertAction {
    pub team_id: TeamID,
    pub serial_number: String,
}

impl_developer_action!(
    RevokeDevelopmentCertAction,
    "revokeDevelopmentCert.action",
    EmptyResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentCertRequest {
    pub cert_request_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitDevelopmentCsrResponse {
    pub cert_request: DevelopmentCertRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitDevelopmentCsrAction {
    pub team_id: TeamID,
    pub machine_id: String,
    pub machine_name: String,
    pub csr_content: String,
}

impl_developer_action!(
    SubmitDevelopmentCsrAction,
    "submitDevelopmentCSR.action",
    SubmitDevelopmentCsrResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppIdFeature {
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "iCloud")]
    ICloud,
    #[serde(rename = "inAppPurchase")]
    InAppPurchase,
    #[serde(rename = "gameCenter")]
    GameCenter,
    #[serde(rename = "pass")]
    Passbook,
    #[serde(rename = "IAD53UNK2F")]
    InterAppAudio,
    #[serde(rename = "V66P55NK2I")]
    VpnConfiguration,
    #[serde(rename = "dataProtection")]
    DataProtection,
    #[serde(rename = "SKC3T5S89Y")]
    AssociatedDomains,
    #[serde(rename = "APG3427HIY")]
    AppGroup,
    #[serde(rename = "HK421J6T7P")]
    HealthKit,
    #[serde(rename = "homeKit")]
    HomeKit,
    #[serde(rename = "WC421J6T7P")]
    WirelessAccessory,
    #[serde(rename = "cloudKitVersion")]
    CloudKitVersion,
}

impl AppIdFeature {
    pub fn as_str(self) -> &'static str {
        match self {
            AppIdFeature::Push => "push",
            AppIdFeature::ICloud => "iCloud",
            AppIdFeature::InAppPurchase => "inAppPurchase",
            AppIdFeature::GameCenter => "gameCenter",
            AppIdFeature::Passbook => "pass",
            AppIdFeature::InterAppAudio => "IAD53UNK2F",
            AppIdFeature::VpnConfiguration => "V66P55NK2I",
            AppIdFeature::DataProtection => "dataProtection",
            AppIdFeature::AssociatedDomains => "SKC3T5S89Y",
            AppIdFeature::AppGroup => "APG3427HIY",
            AppIdFeature::HealthKit => "HK421J6T7P",
            AppIdFeature::HomeKit => "homeKit",
            AppIdFeature::WirelessAccessory => "WC421J6T7P",
            AppIdFeature::CloudKitVersion => "cloudKitVersion",
        }
    }
}

impl AsRef<str> for AppIdFeature {
    fn as_ref(&self) -> &str {
        (*self).as_str()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppId {
    pub app_id_id: String,
    pub identifier: String,
    pub name: String,
    pub features: Dictionary,
    #[serde(default)]
    pub expiration_date: Option<plist::Date>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAppIdsResponse {
    pub app_ids: Vec<AppId>,
    #[serde(default = "max_u64")]
    pub max_quantity: u64,
    #[serde(default = "max_u64")]
    pub available_quantity: u64,
}

fn max_u64() -> u64 {
    u64::MAX
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAppIdsAction {
    pub team_id: TeamID,
}

impl_developer_action!(
    ListAppIdsAction,
    "listAppIds.action",
    ListAppIdsResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAppIdAction {
    pub identifier: String,
    pub name: String,
    pub team_id: TeamID,
}

impl_developer_action!(
    AddAppIdAction,
    "addAppId.action",
    EmptyResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
pub struct UpdatedAppId {
    pub features: Dictionary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppIdResponse {
    pub app_id: UpdatedAppId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppIdAction {
    pub app_id_id: String,
    pub team_id: TeamID,
    #[serde(flatten)]
    pub features: Dictionary,
}

impl_developer_action!(
    UpdateAppIdAction,
    "updateAppId.action",
    UpdateAppIdResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAppIdAction {
    pub app_id_id: String,
    pub team_id: TeamID,
}

impl_developer_action!(
    DeleteAppIdAction,
    "deleteAppId.action",
    EmptyResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationGroup {
    pub application_group: String,
    pub name: String,
    pub identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListApplicationGroupsResponse {
    pub application_group_list: Vec<ApplicationGroup>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListApplicationGroupsAction {
    pub team_id: TeamID,
}

impl_developer_action!(
    ListApplicationGroupsAction,
    "listApplicationGroups.action",
    ListApplicationGroupsResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddApplicationGroupResponse {
    pub application_group: ApplicationGroup,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddApplicationGroupAction {
    pub identifier: String,
    pub name: String,
    pub team_id: TeamID,
}

impl_developer_action!(
    AddApplicationGroupAction,
    "addApplicationGroup.action",
    AddApplicationGroupResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignApplicationGroupToAppIdAction {
    pub app_id_id: String,
    pub application_groups: String,
    pub team_id: TeamID,
}

impl_developer_action!(
    AssignApplicationGroupToAppIdAction,
    "assignApplicationGroupToAppId.action",
    EmptyResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningProfile {
    pub provisioning_profile_id: String,
    pub name: String,
    pub encoded_profile: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTeamProvisioningProfileResponse {
    pub provisioning_profile: ProvisioningProfile,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTeamProvisioningProfileAction {
    pub app_id_id: String,
    pub team_id: TeamID,
}

impl_developer_action!(
    DownloadTeamProvisioningProfileAction,
    "downloadTeamProvisioningProfile.action",
    DownloadTeamProvisioningProfileResponse,
    [crate::IOS, crate::TvOS, crate::WatchOS]
);
