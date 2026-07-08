mod adi_view;
mod app_view;
mod developer_login_view;
mod developer_view;
use crate::adi_services::{self, CoreAdiInstallEvent};
use crate::constants::*;
use crate::developer_accounts::{
    mock_developer_login, mock_developer_login_with_code, MockDeveloperLoginOutcome,
};
use crate::edit_line::EditLine;
use crate::main_view::SideloaderView;
use crate::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AdiProvisioningState, AppEntitlement,
    AppIdOption, AppMetadataField, AppOption, EntitlementValue, MachineIdentity,
    SupportedDeviceFamily, TeamOption,
};
use crate::paths::app_data_dir;
use crate::widgets::{
    chevron, combo_button, combo_item_content, combo_with_popover, developer_account_title,
    dropdown_list, lucide_icon, progress_circle, properties_list,
};
use futures::{channel::mpsc, StreamExt};
use gpui::{
    anchored, deferred, div, img, point, prelude::*, px, rgb, size, AnchoredPositionMode, App,
    AppContext, Bounds, ClickEvent, Context, Entity, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, KeyDownEvent, ObjectFit, ParentElement, PathPromptOptions, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, WeakEntity, Window, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
};
use std::sync::Arc;
use std::{fs, process::Command};

#[derive(Clone)]
pub(crate) enum SettingsWindowRequest {
    DeveloperLogin {
        parent: WeakEntity<SideloaderView>,
    },
    Team {
        parent: WeakEntity<SideloaderView>,
        teams: Vec<TeamOption>,
        selected_team: usize,
        auto_app_id: bool,
        selected_app_id: usize,
    },
    AppSettings {
        parent: WeakEntity<SideloaderView>,
        app_index: usize,
        app: AppOption,
        enabled_patches: Vec<bool>,
        team_id: SharedString,
    },
    AdiSettings {
        parent: WeakEntity<SideloaderView>,
        backends: Vec<AdiBackendOption>,
        selected_backend: usize,
        machine_identity: MachineIdentity,
        android_device_identity: MachineIdentity,
        android_adi_identifier: String,
    },
}

pub(crate) struct SettingsWindow {
    focus_handle: FocusHandle,
    request: SettingsWindowRequest,
    scroll_handle: ScrollHandle,
    team_picker_open: bool,
    app_id_picker_open: bool,
    adi_backend_picker_open: bool,
    adi_operation: Option<AdiOperationState>,
    machine_identity_edit: Option<MachineIdentityEdit>,
    app_detail_edit: Option<AppDetailEdit>,
    selected_entitlement: Option<usize>,
    entitlement_edit: Option<EntitlementEdit>,
    entitlement_type_picker_open: bool,
    developer_login: DeveloperLoginState,
    spinner_turns: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeveloperLoginStep {
    Credentials,
    TwoFactor,
}

struct DeveloperLoginState {
    email: Entity<EditLine>,
    password: Entity<EditLine>,
    code: Entity<EditLine>,
    remember_account: bool,
    step: DeveloperLoginStep,
    two_factor_detail: SharedString,
    error: Option<SharedString>,
}

impl DeveloperLoginState {
    fn new(cx: &mut Context<SettingsWindow>) -> Self {
        Self {
            email: cx.new(|cx| EditLine::new("", cx)),
            password: cx.new(|cx| EditLine::new_password("", cx)),
            code: cx.new(|cx| EditLine::new("", cx)),
            remember_account: false,
            step: DeveloperLoginStep::Credentials,
            two_factor_detail: "".into(),
            error: None,
        }
    }
}

#[derive(Clone, Debug)]
enum AdiOperationState {
    DownloadingCoreAdi {
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    InstallingCoreAdi,
    Provisioning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MachineIdentityField {
    MachineModel,
    OsName,
    OsVersion,
    MachineId,
}

impl MachineIdentityField {
    fn label(self) -> &'static str {
        match self {
            Self::MachineModel => "Machine model",
            Self::OsName => "OS name",
            Self::OsVersion => "OS version",
            Self::MachineId => "Machine ID",
        }
    }

    fn value(self, identity: &MachineIdentity) -> &SharedString {
        match self {
            Self::MachineModel => &identity.machine_name,
            Self::OsName => &identity.os_name,
            Self::OsVersion => &identity.os_version,
            Self::MachineId => &identity.machine_id,
        }
    }

    fn set_value(self, identity: &mut MachineIdentity, value: String) {
        match self {
            Self::MachineModel => identity.machine_name = value.into(),
            Self::OsName => identity.os_name = value.into(),
            Self::OsVersion => identity.os_version = value.into(),
            Self::MachineId => identity.machine_id = value.into(),
        }
    }
}

#[derive(Clone)]
struct MachineIdentityEdit {
    field: MachineIdentityField,
    input: Entity<EditLine>,
}

#[derive(Clone)]
enum AppDetailEdit {
    Text {
        field: AppMetadataField,
        input: Entity<EditLine>,
    },
    SupportedDevices {
        selected: Vec<SupportedDeviceFamily>,
    },
}

impl AppDetailEdit {
    fn field(&self) -> AppMetadataField {
        match self {
            Self::Text { field, .. } => *field,
            Self::SupportedDevices { .. } => AppMetadataField::SupportedDevices,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntitlementField {
    Key,
    ValueType,
    Value,
}

#[derive(Clone)]
enum EntitlementEdit {
    Text {
        row: usize,
        field: EntitlementField,
        input: Entity<EditLine>,
    },
    Boolean {
        row: usize,
        value: bool,
    },
    Array {
        row: usize,
        items: Vec<Entity<EditLine>>,
    },
    Type {
        row: usize,
        selected: SharedString,
    },
}

impl EntitlementEdit {
    fn row(&self) -> usize {
        match self {
            Self::Text { row, .. }
            | Self::Boolean { row, .. }
            | Self::Array { row, .. }
            | Self::Type { row, .. } => *row,
        }
    }

    fn field(&self) -> EntitlementField {
        match self {
            Self::Text { field, .. } => *field,
            Self::Boolean { .. } | Self::Array { .. } => EntitlementField::Value,
            Self::Type { .. } => EntitlementField::ValueType,
        }
    }
}

impl AdiOperationState {
    fn label(&self) -> SharedString {
        match self {
            Self::DownloadingCoreAdi {
                downloaded_bytes,
                total_bytes: Some(total_bytes),
            } if *total_bytes > 0 => {
                let percent = ((*downloaded_bytes as f32 / *total_bytes as f32) * 100.)
                    .clamp(0., 99.)
                    .round() as u32;
                format!("Downloading {percent}%").into()
            }
            Self::DownloadingCoreAdi { .. } => "Downloading...".into(),
            Self::InstallingCoreAdi => "Installing...".into(),
            Self::Provisioning => "Provisioning...".into(),
        }
    }

    fn progress(&self) -> f32 {
        match self {
            Self::DownloadingCoreAdi {
                downloaded_bytes,
                total_bytes: Some(total_bytes),
            } if *total_bytes > 0 => (*downloaded_bytes as f32 / *total_bytes as f32).clamp(0., 1.),
            Self::DownloadingCoreAdi { .. } => 0.28,
            Self::InstallingCoreAdi => 0.72,
            Self::Provisioning => 0.45,
        }
    }

    fn is_indeterminate(&self) -> bool {
        matches!(
            self,
            Self::DownloadingCoreAdi {
                total_bytes: None,
                ..
            } | Self::InstallingCoreAdi
                | Self::Provisioning
        )
    }

    fn is_coreadi_install(&self) -> bool {
        matches!(
            self,
            Self::DownloadingCoreAdi { .. } | Self::InstallingCoreAdi
        )
    }

    fn is_provisioning(&self) -> bool {
        matches!(self, Self::Provisioning)
    }
}
impl SettingsWindowRequest {
    fn title(&self) -> &'static str {
        match self {
            SettingsWindowRequest::DeveloperLogin { .. } => "Add Apple Account",
            SettingsWindowRequest::Team { .. } => "Developer Settings",
            SettingsWindowRequest::AppSettings { .. } => "App Settings",
            SettingsWindowRequest::AdiSettings { .. } => "Settings",
        }
    }
}

impl SettingsWindow {
    fn new(request: SettingsWindowRequest, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());

        Self {
            focus_handle,
            request,
            scroll_handle: ScrollHandle::new(),
            team_picker_open: false,
            app_id_picker_open: false,
            adi_backend_picker_open: false,
            adi_operation: None,
            machine_identity_edit: None,
            app_detail_edit: None,
            selected_entitlement: None,
            entitlement_edit: None,
            entitlement_type_picker_open: false,
            developer_login: DeveloperLoginState::new(cx),
            spinner_turns: 0.,
        }
    }

    fn show_request(
        &mut self,
        request: SettingsWindowRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let reset_login = matches!(&request, SettingsWindowRequest::DeveloperLogin { .. });
        let title = request.title();
        self.request = request;
        self.team_picker_open = false;
        self.app_id_picker_open = false;
        self.adi_backend_picker_open = false;
        self.machine_identity_edit = None;
        self.app_detail_edit = None;
        self.selected_entitlement = None;
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        if reset_login {
            self.developer_login = DeveloperLoginState::new(cx);
        }
        window.set_window_title(title);
        window.activate_window();
        window.focus(&self.focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());
        cx.notify();
    }

    pub(crate) fn set_app_settings_team_id(
        &mut self,
        updated_team_id: SharedString,
        cx: &mut Context<Self>,
    ) {
        let SettingsWindowRequest::AppSettings { team_id, .. } = &mut self.request else {
            return;
        };
        if team_id.as_ref() != updated_team_id.as_ref() {
            *team_id = updated_team_id;
            cx.notify();
        }
    }

    fn toggle_remember_developer_account(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::DeveloperLogin { .. }) {
            self.developer_login.remember_account = !self.developer_login.remember_account;
            cx.notify();
        }
    }

    fn submit_developer_login(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.submit_developer_login_inner(window, cx);
    }

    fn submit_developer_login_inner(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let SettingsWindowRequest::DeveloperLogin { parent } = &self.request else {
            return;
        };
        let parent = parent.clone();
        let email = self.developer_login.email.read(cx).value();
        let password = self.developer_login.password.read(cx).value();

        match mock_developer_login(&email, &password) {
            Ok(MockDeveloperLoginOutcome::RequiresTwoFactor { detail }) => {
                self.developer_login.step = DeveloperLoginStep::TwoFactor;
                self.developer_login.two_factor_detail = detail;
                self.developer_login.error = None;
                self.developer_login.code.update(cx, |code, cx| {
                    *code = EditLine::new("", cx);
                });
                let code = self.developer_login.code.clone();
                code.update(cx, |code, cx| code.focus(window, cx));
                cx.notify();
            }
            Ok(MockDeveloperLoginOutcome::SignedIn(account)) => {
                self.finish_developer_login(account, parent, window, cx);
            }
            Err(error) => {
                self.developer_login.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn submit_developer_two_factor(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.submit_developer_two_factor_inner(window, cx);
    }

    fn submit_developer_two_factor_inner(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let SettingsWindowRequest::DeveloperLogin { parent } = &self.request else {
            return;
        };
        let parent = parent.clone();
        let email = self.developer_login.email.read(cx).value();
        let code = self.developer_login.code.read(cx).value();

        match mock_developer_login_with_code(&email, &code) {
            Ok(account) => self.finish_developer_login(account, parent, window, cx),
            Err(error) => {
                self.developer_login.error = Some(error.into());
                cx.notify();
            }
        }
    }

    fn back_to_developer_login(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::DeveloperLogin { .. }) {
            self.developer_login.step = DeveloperLoginStep::Credentials;
            self.developer_login.error = None;
            cx.notify();
        }
    }

    fn handle_developer_login_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(self.request, SettingsWindowRequest::DeveloperLogin { .. }) {
            return;
        }

        if event.keystroke.key.as_str() == "enter" {
            cx.stop_propagation();
            window.prevent_default();
            match self.developer_login.step {
                DeveloperLoginStep::Credentials => {
                    self.submit_developer_login_inner(window, cx);
                }
                DeveloperLoginStep::TwoFactor => {
                    self.submit_developer_two_factor_inner(window, cx);
                }
            }
        }
    }

    fn finish_developer_login(
        &mut self,
        mut account: AccountOption,
        parent: WeakEntity<SideloaderView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        account.detail = if self.developer_login.remember_account {
            "Mock login session, marked to remember".into()
        } else {
            "Mock login session".into()
        };

        match parent.update(cx, |view, cx| {
            view.add_developer_account_from_settings(account, cx)
        }) {
            Ok(Some(request)) => self.show_request(request, window, cx),
            Ok(None) => {
                self.developer_login.error =
                    Some("The mock account has no developer teams.".into());
                cx.notify();
            }
            Err(_) => {
                self.developer_login.error =
                    Some("The main sideloader window is not available.".into());
                cx.notify();
            }
        }
    }

    fn select_team(
        &mut self,
        index: usize,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SettingsWindowRequest::Team {
            parent,
            teams,
            selected_team,
            selected_app_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        if index >= teams.len() {
            return;
        }

        *selected_team = index;
        *selected_app_id = 0;
        self.team_picker_open = false;
        self.app_id_picker_open = false;
        let _ = parent.update(cx, |view, cx| {
            if let Some(team_id) = view.select_team_from_settings(index, cx) {
                if let Some(handle) = view.app_settings_window.clone() {
                    let _ = handle.update(cx, |settings, _, cx| {
                        settings.set_app_settings_team_id(team_id.clone(), cx);
                    });
                }
            }
        });
        cx.notify();
    }

    fn toggle_team_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::Team { .. }) {
            self.team_picker_open = !self.team_picker_open;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn toggle_auto_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team {
            parent,
            auto_app_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        *auto_app_id = !*auto_app_id;
        let auto_app_id = *auto_app_id;
        self.app_id_picker_open = false;
        let _ = parent.update(cx, |view, cx| {
            view.set_auto_app_id_from_settings(auto_app_id, cx);
        });
        cx.notify();
    }

    fn toggle_app_id_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team { auto_app_id, .. } = &self.request else {
            return;
        };

        if !*auto_app_id {
            self.app_id_picker_open = !self.app_id_picker_open;
            self.team_picker_open = false;
            cx.notify();
        }
    }

    fn select_app_id(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team {
            parent,
            teams,
            selected_team,
            selected_app_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        let Some(team) = teams.get(*selected_team) else {
            return;
        };

        if index < team.app_ids.len() {
            *selected_app_id = index;
            self.app_id_picker_open = false;
            let _ = parent.update(cx, |view, cx| {
                view.select_app_id_from_settings(index, cx);
            });
            cx.notify();
        }
    }

    fn add_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn remove_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn edit_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn refresh_team_details(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::Team { .. }) {
            self.team_picker_open = false;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn log_out_developer_account(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::Team { .. }) {
            self.team_picker_open = false;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn select_adi_backend(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let SettingsWindowRequest::AdiSettings {
            parent,
            backends,
            selected_backend,
            ..
        } = &mut self.request
        else {
            return;
        };

        if index >= backends.len() {
            return;
        }

        *selected_backend = index;
        self.adi_backend_picker_open = false;
        let _ = parent.update(cx, |view, cx| {
            view.select_adi_backend_from_settings(index, cx);
        });
        cx.notify();
    }

    fn toggle_adi_backend_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        if matches!(self.request, SettingsWindowRequest::AdiSettings { .. }) {
            self.adi_backend_picker_open = !self.adi_backend_picker_open;
            cx.notify();
        }
    }

    fn open_data_folder(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.adi_backend_picker_open = false;
        if let Err(error) = open_app_data_folder() {
            eprintln!("{error}");
        }
        cx.notify();
    }

    fn repair_adi_backend(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let Some((kind, _, _)) = self.selected_adi_backend_context() else {
            return;
        };

        if kind != AdiBackendKind::AndroidCoreAdi {
            cx.notify();
            return;
        }

        self.adi_backend_picker_open = false;
        self.adi_operation = Some(AdiOperationState::DownloadingCoreAdi {
            downloaded_bytes: 0,
            total_bytes: None,
        });
        let (progress_sender, mut progress_receiver) = mpsc::unbounded();
        cx.spawn(async move |settings, cx| {
            while let Some(event) = progress_receiver.next().await {
                let _ = settings.update(cx, |settings, cx| {
                    settings.set_coreadi_install_event(event, cx);
                });
            }
        })
        .detach();
        cx.spawn(async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    adi_services::download_and_install_coreadi(move |event| {
                        let _ = progress_sender.unbounded_send(event);
                    })
                })
                .await;

            let _ = settings.update(cx, |settings, cx| match result {
                Ok(_) => {
                    settings.adi_operation = None;
                    settings.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx)
                }
                Err(error) => {
                    settings.adi_operation = None;
                    settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.into()),
                        cx,
                    );
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn select_coreadi_apk(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select APK".into()),
        });

        cx.spawn(async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = settings.update(cx, |settings, cx| {
                        settings.set_selected_adi_provisioning_state(
                            AdiProvisioningState::Error(error.to_string().into()),
                            cx,
                        );
                    });
                    return;
                }
            };

            let Some(apk_path) = paths.into_iter().next() else {
                return;
            };

            let _ = settings.update(cx, |settings, cx| {
                settings.adi_operation = Some(AdiOperationState::InstallingCoreAdi);
                cx.notify();
            });
            let result = cx
                .background_spawn(async move { adi_services::install_coreadi_from_apk(&apk_path) })
                .await;
            let _ = settings.update(cx, |settings, cx| match result {
                Ok(_) => {
                    settings.adi_operation = None;
                    settings.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx)
                }
                Err(error) => {
                    settings.adi_operation = None;
                    settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.into()),
                        cx,
                    );
                }
            });
        })
        .detach();
    }

    fn erase_adi_backend_provisioning(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let Some((kind, _machine_identity, android_adi_identifier)) =
            self.selected_adi_backend_context()
        else {
            return;
        };

        match adi_services::erase_provisioning(kind, &android_adi_identifier) {
            Ok(()) => self.refresh_adi_backend_options(Some(kind), cx),
            Err(error) => self
                .set_selected_adi_provisioning_state(AdiProvisioningState::Error(error.into()), cx),
        }
    }

    fn provision_adi_backend(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let Some((kind, machine_identity, android_adi_identifier)) =
            self.selected_adi_backend_context()
        else {
            return;
        };

        self.adi_backend_picker_open = false;
        self.adi_operation = Some(AdiOperationState::Provisioning);
        cx.spawn(async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    adi_services::provision(kind, &machine_identity, &android_adi_identifier)
                })
                .await;
            let _ = settings.update(cx, |settings, cx| {
                settings.adi_operation = None;
                match result {
                    Ok(()) => settings.refresh_adi_backend_options(Some(kind), cx),
                    Err(error) => settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.into()),
                        cx,
                    ),
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn selected_adi_backend_context(&self) -> Option<(AdiBackendKind, MachineIdentity, String)> {
        let SettingsWindowRequest::AdiSettings {
            backends,
            selected_backend,
            machine_identity,
            android_device_identity,
            android_adi_identifier,
            ..
        } = &self.request
        else {
            return None;
        };

        backends.get(*selected_backend).map(|backend| {
            let machine_identity = match backend.kind {
                AdiBackendKind::AndroidCoreAdi => android_device_identity.clone(),
                AdiBackendKind::SystemAdid | AdiBackendKind::WindowsCoreAdi => {
                    machine_identity.clone()
                }
            };
            (
                backend.kind,
                machine_identity,
                android_adi_identifier.clone(),
            )
        })
    }

    fn refresh_adi_backend_options(
        &mut self,
        preferred_kind: Option<AdiBackendKind>,
        cx: &mut Context<Self>,
    ) {
        let SettingsWindowRequest::AdiSettings {
            parent,
            backends,
            selected_backend,
            android_adi_identifier,
            ..
        } = &mut self.request
        else {
            return;
        };

        let selected_kind =
            preferred_kind.or_else(|| backends.get(*selected_backend).map(|backend| backend.kind));
        let refreshed = adi_services::available_backends(android_adi_identifier);
        let selected = selected_kind
            .and_then(|kind| refreshed.iter().position(|backend| backend.kind == kind))
            .unwrap_or_else(|| adi_services::default_backend(&refreshed));

        *backends = refreshed.clone();
        *selected_backend = selected;

        let _ = parent.update(cx, |view, cx| {
            view.replace_adi_backends_from_settings(refreshed, selected, true, cx);
        });
        cx.notify();
    }

    fn set_selected_adi_provisioning_state(
        &mut self,
        state: AdiProvisioningState,
        cx: &mut Context<Self>,
    ) {
        let SettingsWindowRequest::AdiSettings {
            parent,
            backends,
            selected_backend,
            ..
        } = &mut self.request
        else {
            return;
        };

        if let Some(backend) = backends.get_mut(*selected_backend) {
            backend.provisioning_state = state.clone();
        }

        let backends = backends.clone();
        let selected = *selected_backend;
        let _ = parent.update(cx, |view, cx| {
            view.replace_adi_backends_from_settings(backends, selected, false, cx);
        });
        cx.notify();
    }

    fn set_coreadi_install_event(&mut self, event: CoreAdiInstallEvent, cx: &mut Context<Self>) {
        self.adi_operation = Some(match event {
            CoreAdiInstallEvent::Downloading(progress) => AdiOperationState::DownloadingCoreAdi {
                downloaded_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
            },
            CoreAdiInstallEvent::Installing => AdiOperationState::InstallingCoreAdi,
        });
        cx.notify();
    }

    fn begin_machine_identity_edit(
        &mut self,
        field: MachineIdentityField,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let Some(value) = self.editable_machine_identity_value(field).cloned() else {
            return;
        };
        let input = cx.new(|cx| EditLine::new(value, cx));
        self.machine_identity_edit = Some(MachineIdentityEdit {
            field,
            input: input.clone(),
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn save_machine_identity_edit_from_button(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.save_machine_identity_edit(cx);
    }

    fn cancel_machine_identity_edit(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.machine_identity_edit = None;
        cx.notify();
    }

    fn handle_machine_identity_editor_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.machine_identity_edit.is_none() {
            return;
        }

        match event.keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                window.prevent_default();
                self.save_machine_identity_edit(cx);
            }
            "escape" => {
                cx.stop_propagation();
                window.prevent_default();
                self.machine_identity_edit = None;
                cx.notify();
            }
            _ => {}
        }
    }

    fn editable_machine_identity_value(
        &self,
        field: MachineIdentityField,
    ) -> Option<&SharedString> {
        let SettingsWindowRequest::AdiSettings {
            backends,
            selected_backend,
            android_device_identity,
            ..
        } = &self.request
        else {
            return None;
        };

        let backend = backends.get(*selected_backend)?;
        if backend.kind != AdiBackendKind::AndroidCoreAdi || !backend.editable_identity {
            return None;
        }

        Some(field.value(android_device_identity))
    }

    fn save_machine_identity_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.machine_identity_edit.take() else {
            return;
        };

        let mut changed_identity = None;
        let mut should_erase_provisioning = false;
        let mut android_adi_identifier = String::new();
        let mut error = None;

        if let SettingsWindowRequest::AdiSettings {
            parent,
            backends,
            selected_backend,
            android_device_identity,
            android_adi_identifier: request_android_adi_identifier,
            ..
        } = &mut self.request
        {
            let Some(backend) = backends.get(*selected_backend) else {
                cx.notify();
                return;
            };
            if backend.kind != AdiBackendKind::AndroidCoreAdi || !backend.editable_identity {
                cx.notify();
                return;
            }

            let new_value = edit.input.read(cx).value().to_string();
            let old_value = edit.field.value(android_device_identity).to_string();
            if old_value != new_value {
                edit.field.set_value(android_device_identity, new_value);
                changed_identity = Some((parent.clone(), android_device_identity.clone()));
                should_erase_provisioning = matches!(
                    backend.provisioning_state,
                    AdiProvisioningState::Provisioned
                );
                android_adi_identifier = request_android_adi_identifier.clone();
            }
        }

        if let Some((parent, identity)) = changed_identity {
            let _ = parent.update(cx, |view, cx| {
                view.replace_android_device_identity_from_settings(identity, cx);
            });

            if should_erase_provisioning {
                error = adi_services::erase_provisioning(
                    AdiBackendKind::AndroidCoreAdi,
                    &android_adi_identifier,
                )
                .err();
            }

            self.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx);
            if let Some(error) = error {
                self.set_selected_adi_provisioning_state(
                    AdiProvisioningState::Error(error.into()),
                    cx,
                );
            }
        } else {
            cx.notify();
        }
    }

    fn edit_app_icon(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select App Icon".into()),
        });

        cx.spawn(async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    eprintln!("Failed to select app icon: {error}");
                    return;
                }
            };

            let Some(path) = paths.into_iter().next() else {
                return;
            };

            let _ = settings.update(cx, |settings, cx| {
                if let SettingsWindowRequest::AppSettings {
                    parent,
                    app_index,
                    app,
                    ..
                } = &mut settings.request
                {
                    app.icon_override_path = Some(path.to_string_lossy().to_string().into());
                    let app_index = *app_index;
                    let updated = app.clone();
                    let _ = parent.update(cx, |view, cx| {
                        view.replace_app_from_settings(app_index, updated, cx);
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn begin_app_detail_edit(
        &mut self,
        field: AppMetadataField,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings { app, .. } = &self.request else {
            return;
        };
        if field == AppMetadataField::SupportedDevices {
            self.app_detail_edit = Some(AppDetailEdit::SupportedDevices {
                selected: app.supported_devices().to_vec(),
            });
        } else {
            let input = cx.new(|cx| EditLine::new(app.field(field).value().clone(), cx));
            self.app_detail_edit = Some(AppDetailEdit::Text {
                field,
                input: input.clone(),
            });
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    fn toggle_supported_device_family(
        &mut self,
        family: SupportedDeviceFamily,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some(AppDetailEdit::SupportedDevices { selected }) = &mut self.app_detail_edit else {
            return;
        };

        if let Some(index) = selected.iter().position(|candidate| *candidate == family) {
            selected.remove(index);
        } else {
            selected.push(family);
            selected.sort();
            selected.dedup();
        }
        cx.notify();
    }

    fn save_app_detail_edit_from_button(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.save_app_detail_edit(cx);
    }

    fn cancel_app_detail_edit(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.app_detail_edit = None;
        cx.notify();
    }

    fn handle_app_detail_editor_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.app_detail_edit.is_none() {
            return;
        }

        match event.keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                window.prevent_default();
                self.save_app_detail_edit(cx);
            }
            "escape" => {
                cx.stop_propagation();
                window.prevent_default();
                self.app_detail_edit = None;
                cx.notify();
            }
            _ => {}
        }
    }

    fn handle_app_settings_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.entitlement_edit.is_some() {
            self.handle_entitlement_editor_key(event, window, cx);
        } else {
            self.handle_app_detail_editor_key(event, window, cx);
        }
    }

    fn save_app_detail_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.app_detail_edit.take() else {
            return;
        };

        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            ..
        } = &mut self.request
        else {
            cx.notify();
            return;
        };

        match edit {
            AppDetailEdit::Text { field, input } => {
                let value = input.read(cx).value().to_string();
                app.set_field_override(field, value);
            }
            AppDetailEdit::SupportedDevices { selected } => {
                app.set_supported_devices_override(selected);
            }
        }
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }

    fn revert_app_detail_field(
        &mut self,
        field: AppMetadataField,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            ..
        } = &mut self.request
        else {
            return;
        };

        app.clear_field_override(field);
        if self
            .app_detail_edit
            .as_ref()
            .is_some_and(|edit| edit.field() == field)
        {
            self.app_detail_edit = None;
        }
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }

    fn add_entitlement(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            team_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        let mut entitlements = app.effective_entitlements(team_id);
        entitlements.push(AppEntitlement {
            key: "new.entitlement".into(),
            value: EntitlementValue::String("".into()),
        });
        let selected = entitlements.len().saturating_sub(1);
        app.entitlement_overrides = Some(entitlements);
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        let input = cx.new(|cx| EditLine::new("new.entitlement", cx));
        self.selected_entitlement = Some(selected);
        self.entitlement_type_picker_open = false;
        self.entitlement_edit = Some(EntitlementEdit::Text {
            row: selected,
            field: EntitlementField::Key,
            input: input.clone(),
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn remove_entitlement(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            team_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        let mut entitlements = app.effective_entitlements(team_id);
        if entitlements.is_empty() {
            return;
        }
        let index = self
            .selected_entitlement
            .filter(|index| *index < entitlements.len())
            .unwrap_or_else(|| entitlements.len() - 1);
        entitlements.remove(index);
        let entitlements_len = entitlements.len();
        app.entitlement_overrides = Some(entitlements);
        self.selected_entitlement = if entitlements_len == 0 {
            None
        } else {
            Some(index.min(entitlements_len - 1))
        };
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }

    fn revert_entitlements(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            ..
        } = &mut self.request
        else {
            return;
        };

        app.entitlement_overrides = None;
        self.selected_entitlement = None;
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }

    fn revert_entitlement(
        &mut self,
        row: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            team_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        let mut entitlements = app.effective_entitlements(team_id);
        if row >= entitlements.len() {
            return;
        }

        if let Some(default) = entitlement_revert_target(
            &entitlements[row],
            row,
            &app.default_effective_entitlements(team_id),
        ) {
            entitlements[row] = default;
        } else {
            entitlements.remove(row);
        }

        let defaults = app.default_effective_entitlements(team_id);
        app.entitlement_overrides = (entitlements != defaults).then_some(entitlements);
        self.selected_entitlement = app.entitlement_overrides.as_ref().and_then(|entitlements| {
            (!entitlements.is_empty()).then_some(row.min(entitlements.len() - 1))
        });
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }

    fn begin_entitlement_edit(
        &mut self,
        row: usize,
        field: EntitlementField,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AppSettings { app, team_id, .. } = &self.request else {
            return;
        };
        let entitlements = app.effective_entitlements(team_id);
        let Some(entitlement) = entitlements.get(row) else {
            return;
        };
        self.selected_entitlement = Some(row);
        self.entitlement_type_picker_open = false;
        if field == EntitlementField::ValueType {
            self.entitlement_type_picker_open = true;
            self.entitlement_edit = Some(EntitlementEdit::Type {
                row,
                selected: entitlement.value.type_label().into(),
            });
            cx.notify();
            return;
        }

        if field == EntitlementField::Value {
            match &entitlement.value {
                EntitlementValue::Boolean(value) => {
                    self.entitlement_edit = Some(EntitlementEdit::Boolean { row, value: *value });
                    cx.notify();
                    return;
                }
                EntitlementValue::Array(_) => {
                    let items = entitlement
                        .value
                        .array_edit_values()
                        .into_iter()
                        .map(|value| cx.new(|cx| EditLine::new(value, cx)))
                        .collect::<Vec<_>>();
                    self.entitlement_edit = Some(EntitlementEdit::Array {
                        row,
                        items: items.clone(),
                    });
                    if let Some(input) = items.first() {
                        input.update(cx, |input, cx| input.focus(window, cx));
                    }
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        let value = entitlement_field_value(entitlement, field).clone();
        let input = cx.new(|cx| EditLine::new(value, cx));
        self.entitlement_edit = Some(EntitlementEdit::Text {
            row,
            field,
            input: input.clone(),
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn toggle_boolean_entitlement_value(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let Some(EntitlementEdit::Boolean { value: edited, .. }) = &mut self.entitlement_edit
        else {
            return;
        };
        *edited = !*edited;
        cx.notify();
    }

    fn add_entitlement_array_item(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let Some(EntitlementEdit::Array { items, .. }) = &mut self.entitlement_edit else {
            return;
        };
        let input = cx.new(|cx| EditLine::new("", cx));
        items.push(input.clone());
        input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn remove_entitlement_array_item(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let Some(EntitlementEdit::Array { items, .. }) = &mut self.entitlement_edit else {
            return;
        };
        if index < items.len() {
            items.remove(index);
        }
        cx.notify();
    }

    fn toggle_entitlement_type_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        if matches!(self.entitlement_edit, Some(EntitlementEdit::Type { .. })) {
            self.entitlement_type_picker_open = !self.entitlement_type_picker_open;
        }
        cx.notify();
    }

    fn select_entitlement_type(
        &mut self,
        label: &'static str,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        let Some(EntitlementEdit::Type { selected, .. }) = &mut self.entitlement_edit else {
            return;
        };
        *selected = label.into();
        self.entitlement_type_picker_open = false;
        cx.notify();
    }

    fn save_entitlement_edit_from_button(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        self.save_entitlement_edit(cx);
    }

    fn cancel_entitlement_edit(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        self.entitlement_type_picker_open = false;
        self.entitlement_edit = None;
        cx.notify();
    }

    fn handle_entitlement_editor_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                window.prevent_default();
                self.save_entitlement_edit(cx);
            }
            "escape" => {
                cx.stop_propagation();
                window.prevent_default();
                self.entitlement_type_picker_open = false;
                self.entitlement_edit = None;
                cx.notify();
            }
            _ => {}
        }
    }

    fn save_entitlement_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.entitlement_edit.take() else {
            return;
        };
        self.entitlement_type_picker_open = false;
        let SettingsWindowRequest::AppSettings {
            parent,
            app_index,
            app,
            team_id,
            ..
        } = &mut self.request
        else {
            cx.notify();
            return;
        };

        let row = edit.row();
        let mut entitlements = app.effective_entitlements(team_id);
        let Some(entitlement) = entitlements.get_mut(row) else {
            cx.notify();
            return;
        };

        match edit {
            EntitlementEdit::Text { field, input, .. } => {
                let value = input.read(cx).value();
                match field {
                    EntitlementField::Key => entitlement.key = value.into(),
                    EntitlementField::ValueType => {
                        entitlement.value = entitlement.value.with_type_label(value.as_ref())
                    }
                    EntitlementField::Value => {
                        entitlement.value = entitlement.value.with_edit_text(value.as_ref())
                    }
                }
            }
            EntitlementEdit::Boolean { value, .. } => {
                entitlement.value = EntitlementValue::Boolean(value);
            }
            EntitlementEdit::Array { items, .. } => {
                entitlement.value = EntitlementValue::string_array(
                    items
                        .iter()
                        .map(|input| input.read(cx).value().to_string())
                        .collect(),
                );
            }
            EntitlementEdit::Type { selected, .. } => {
                entitlement.value = entitlement.value.with_type_label(selected.as_ref());
            }
        }

        app.entitlement_overrides = Some(entitlements);
        self.selected_entitlement = Some(row);
        let app_index = *app_index;
        let updated = app.clone();
        let _ = parent.update(cx, |view, cx| {
            view.replace_app_from_settings(app_index, updated, cx);
        });
        cx.notify();
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.adi_operation.is_some() {
            self.spinner_turns = (self.spinner_turns + 0.035) % 1.;
            window.request_animation_frame();
        } else {
            self.spinner_turns = 0.;
        }

        let team_picker_open = self.team_picker_open;
        let app_id_picker_open = self.app_id_picker_open;
        let adi_operation = self.adi_operation.clone();
        let adi_busy = adi_operation.is_some();
        let adi_backend_picker_open = self.adi_backend_picker_open && !adi_busy;
        let spinner_turns = self.spinner_turns;
        let machine_identity_edit = self.machine_identity_edit.clone();
        let app_detail_edit = self.app_detail_edit.clone();
        let selected_entitlement = self.selected_entitlement;
        let entitlement_edit = self.entitlement_edit.clone();
        let entitlement_type_picker_open = self.entitlement_type_picker_open;

        match &mut self.request {
            SettingsWindowRequest::DeveloperLogin { .. } => developer_login_view::render(
                &self.focus_handle,
                &self.scroll_handle,
                &self.developer_login,
                cx,
            ),
            SettingsWindowRequest::Team {
                teams,
                selected_team,
                auto_app_id,
                selected_app_id,
                ..
            } => developer_view::render(
                &self.focus_handle,
                &self.scroll_handle,
                teams,
                *selected_team,
                *auto_app_id,
                *selected_app_id,
                team_picker_open,
                app_id_picker_open,
                cx,
            ),
            SettingsWindowRequest::AppSettings {
                app,
                enabled_patches,
                team_id,
                ..
            } => app_view::render(
                &self.focus_handle,
                &self.scroll_handle,
                app,
                enabled_patches,
                team_id,
                app_detail_edit.as_ref(),
                selected_entitlement,
                entitlement_edit.as_ref(),
                entitlement_type_picker_open,
                cx,
            ),
            SettingsWindowRequest::AdiSettings {
                backends,
                selected_backend,
                machine_identity,
                android_device_identity,
                android_adi_identifier: _,
                ..
            } => adi_view::render(
                &self.focus_handle,
                &self.scroll_handle,
                backends,
                *selected_backend,
                machine_identity,
                android_device_identity,
                adi_backend_picker_open,
                adi_operation.as_ref(),
                spinner_turns,
                machine_identity_edit.as_ref(),
                cx,
            ),
        }
    }
}

pub(crate) fn show_or_open_settings_window(
    handle: Option<WindowHandle<SettingsWindow>>,
    request: SettingsWindowRequest,
    width: f32,
    height: f32,
    cx: &mut App,
) -> WindowHandle<SettingsWindow> {
    if let Some(handle) = handle {
        if handle
            .update(cx, |settings, window, cx| {
                settings.show_request(request.clone(), window, cx);
            })
            .is_ok()
        {
            return handle;
        }
    }

    open_settings_window(request, width, height, cx)
}

fn open_settings_window(
    request: SettingsWindowRequest,
    width: f32,
    height: f32,
    cx: &mut App,
) -> WindowHandle<SettingsWindow> {
    let window_size = size(px(width), px(height));
    let bounds = Bounds::centered(None, window_size, cx);
    let title = request.title();

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(window_size),
            is_resizable: true,
            is_minimizable: false,
            kind: WindowKind::Floating,
            ..Default::default()
        },
        |window, cx| {
            window.set_window_title(title);
            cx.new(|cx| SettingsWindow::new(request, window, cx))
        },
    )
    .expect("failed to open settings window")
}

fn open_app_data_folder() -> Result<(), String> {
    let data_dir = app_data_dir()
        .ok_or_else(|| "The application data folder is not available.".to_string())?;
    fs::create_dir_all(&data_dir).map_err(|error| {
        format!(
            "Failed to create application data folder at {}: {error}",
            data_dir.display()
        )
    })?;

    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(target_os = "windows") {
        Command::new("explorer")
    } else {
        Command::new("xdg-open")
    };

    command.arg(&data_dir).spawn().map_err(|error| {
        format!(
            "Failed to open application data folder at {}: {error}",
            data_dir.display()
        )
    })?;
    Ok(())
}

fn scroll_panel(
    scroll_id: &'static str,
    scroll_handle: &ScrollHandle,
    content: impl IntoElement,
) -> gpui::Div {
    let viewport_height = scroll_handle.bounds().size.height.as_f32();
    let max_scroll = scroll_handle.max_offset().y.as_f32();
    let is_scrollable = viewport_height > 0. && max_scroll > 0.5;
    let scroll_offset = (-scroll_handle.offset().y.as_f32()).clamp(0., max_scroll);
    let content_height = viewport_height + max_scroll;
    let thumb_height = if is_scrollable {
        ((viewport_height / content_height) * viewport_height).clamp(36., viewport_height)
    } else {
        0.
    };
    let thumb_top = if is_scrollable && max_scroll > 0. {
        (scroll_offset / max_scroll) * (viewport_height - thumb_height)
    } else {
        0.
    };

    div()
        .min_h_0()
        .flex_1()
        .flex()
        .gap_2()
        .child(
            div()
                .id(scroll_id)
                .min_w_0()
                .h_full()
                .flex_1()
                .overflow_y_scroll()
                .scrollbar_width(px(1.))
                .track_scroll(scroll_handle)
                .child(content),
        )
        .when(is_scrollable, |this| {
            this.child(
                div()
                    .flex_none()
                    .w(px(6.))
                    .h_full()
                    .rounded_full()
                    .bg(rgb(0xdde5e4))
                    .flex()
                    .flex_col()
                    .child(div().flex_none().h(px(thumb_top)))
                    .child(
                        div()
                            .flex_none()
                            .w_full()
                            .h(px(thumb_height))
                            .rounded_full()
                            .bg(rgb(0x789094)),
                    )
                    .child(div().flex_1()),
            )
        })
}

fn settings_window_shell() -> gpui::Div {
    div()
        .size_full()
        .p_5()
        .flex()
        .flex_col()
        .bg(rgb(0xf4f6f4))
        .text_color(rgb(0x263238))
        .font_family(".SystemUIFont")
}

fn settings_window_header(title: &'static str, detail: &'static str) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0x24333a))
                .child(title),
        )
        .child(div().text_xs().text_color(rgb(0x6a7a81)).child(detail))
}

fn settings_window_header_with_action(
    header: impl IntoElement,
    action: impl IntoElement,
) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
        .child(header)
        .child(action)
}

fn open_data_folder_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id("open-data-folder")
        .flex_none()
        .h_8()
        .px_3()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xdfe8e6)))
        .on_click(cx.listener(SettingsWindow::open_data_folder))
        .child(lucide_icon("icons/folder-open.svg"))
        .child("Open Data Folder")
}

fn settings_label(label: &'static str) -> gpui::Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x66767c))
        .child(label)
}

fn machine_identity_editor(input: &Entity<EditLine>) -> gpui::Div {
    div()
        .min_w_0()
        .flex_1()
        .h_7()
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x168291))
        .bg(rgb(0xf2fbfb))
        .text_sm()
        .text_color(rgb(0x24333a))
        .flex()
        .items_center()
        .overflow_hidden()
        .child(input.clone())
}

fn entitlement_revert_target(
    entitlement: &AppEntitlement,
    row: usize,
    defaults: &[AppEntitlement],
) -> Option<AppEntitlement> {
    defaults
        .iter()
        .find(|default| default.key == entitlement.key)
        .cloned()
        .or_else(|| defaults.get(row).cloned())
}

fn entitlement_field_value(entitlement: &AppEntitlement, field: EntitlementField) -> SharedString {
    match field {
        EntitlementField::Key => entitlement.key.clone(),
        EntitlementField::ValueType => entitlement.value.type_label().into(),
        EntitlementField::Value => entitlement.value.display_text(),
    }
}
