pub(crate) mod adi;
pub(crate) mod adi_service;
pub(crate) mod device;
pub(crate) mod error;
mod icon;
pub(crate) mod ipa;
pub(crate) mod paths;
pub(crate) mod preferences;
pub(crate) mod runtime;
pub(crate) mod system_identity;

pub(crate) mod developer;

pub(crate) use adi_service as adi_services;
pub(crate) use developer::service as developer_services;
pub(crate) use device as device_discovery;
pub(crate) use error::{BackendError, BackendResult};
