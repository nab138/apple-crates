use crate::developer_accounts::load_account_options;
use crate::models::{AccountOption, MachineIdentity};

pub(crate) fn load_accounts() -> Vec<AccountOption> {
    load_account_options().unwrap_or_else(|error| {
        eprintln!("{error}");
        Vec::new()
    })
}

pub(crate) fn load_machine_identity() -> MachineIdentity {
    crate::system_identity::machine_identity()
}
