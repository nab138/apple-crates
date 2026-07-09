mod adi_view;
mod app_view;
mod developer_login_view;
mod developer_view;
use crate::app::effects::{
    self as app_effects, CoreAdiInstallEvent, DeveloperAppIdCapabilityUpdate,
    DeveloperLoginOutcome, DeveloperLoginRequest, DeveloperSessionContext,
};
use crate::app::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AdiProvisioningState, AppEntitlement,
    AppIdOption, AppMetadataField, AppOption, DevelopmentCertificateOption, EntitlementValue,
    MachineIdentity, SupportedDeviceFamily, TeamOption,
};
use crate::app::preferences::ThemePreference;
use crate::app::state::SideloaderState;
use crate::app::AppError;
use crate::constants::*;
use crate::ui::main_view::SideloaderView;
use crate::ui::theme::{fixed_rgb, rgb, sync_window_theme};
use crate::ui::widgets::{
    action_button_surface, chevron, developer_account_title, dropdown_list,
    floating_select_popover, icon_button_surface, lucide_icon, lucide_icon_tinted,
    primary_action_button_surface, progress_circle, properties_list, select_button,
    select_item_content, select_option_button, select_with_popover, surface_button,
    FloatingPopoverLayout,
};
use futures::{channel::mpsc, StreamExt};
use gpui::{
    div, img, prelude::*, px, size, AnyElement, App, AppContext, Bounds, ClickEvent, Context,
    Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ObjectFit,
    ParentElement, PathPromptOptions, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, WeakEntity, Window, WindowBounds, WindowHandle, WindowKind,
    WindowOptions,
};
use gpui_component::{
    button::Button,
    input::{Input, InputState, OtpState},
    scroll::ScrollableElement as _,
    Root, Sizable as _,
};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SettingsMode {
    DeveloperLogin,
    Team,
    AppSettings { app_index: usize },
    AdiSettings,
}

#[derive(Clone)]
struct DeveloperLoginSnapshot {
    adi_backend: Option<AdiBackendKind>,
    machine_identity: MachineIdentity,
    android_adi_identifier: String,
}

#[derive(Clone)]
struct TeamSettingsSnapshot {
    teams: Vec<TeamOption>,
    selected_team: usize,
    selected_certificate: usize,
    auto_app_id: bool,
    selected_app_id: usize,
}

#[derive(Clone)]
struct AppSettingsSnapshot {
    app_index: usize,
    app: AppOption,
    enabled_patches: Vec<bool>,
    team_id: String,
}

#[derive(Clone)]
struct AdiSettingsSnapshot {
    theme_preference: ThemePreference,
    backends: Vec<AdiBackendOption>,
    selected_backend: usize,
    machine_identity: MachineIdentity,
    android_device_identity: MachineIdentity,
    android_adi_identifier: String,
}

#[derive(Clone)]
enum SettingsSnapshot {
    DeveloperLogin(DeveloperLoginSnapshot),
    Team(TeamSettingsSnapshot),
    AppSettings(Box<AppSettingsSnapshot>),
    AdiSettings(AdiSettingsSnapshot),
}

pub(crate) struct SettingsWindow {
    parent: WeakEntity<SideloaderView>,
    focus_handle: FocusHandle,
    mode: SettingsMode,
    theme_preference: ThemePreference,
    render_snapshot: Option<SettingsSnapshot>,
    request_generation: u64,
    scroll_handle: ScrollHandle,
    team_picker_open: bool,
    certificate_picker_open: bool,
    app_id_picker_open: bool,
    theme_picker_open: bool,
    adi_backend_picker_open: bool,
    adi_operation: Option<AdiOperationState>,
    machine_identity_edit: Option<MachineIdentityEdit>,
    app_detail_edit: Option<AppDetailEdit>,
    app_id_add_form: Option<AppIdAddForm>,
    app_id_edit_form: Option<AppIdEditForm>,
    selected_entitlement: Option<usize>,
    entitlement_edit: Option<EntitlementEdit>,
    entitlement_type_picker_open: bool,
    app_settings_error: Option<SharedString>,
    developer_login: DeveloperLoginState,
    spinner_turns: f32,
    team_refreshing: bool,
    team_refresh_error: Option<SharedString>,
    certificate_error: Option<SharedString>,
}

#[derive(Clone)]
pub(crate) struct SettingsWindowHandle {
    window: WindowHandle<Root>,
    settings: Entity<SettingsWindow>,
}

impl SettingsWindowHandle {
    pub(crate) fn close(self, cx: &mut App) {
        let _ = self
            .window
            .update(cx, |_, window, _| window.remove_window());
    }

    fn show_request(
        &self,
        parent: WeakEntity<SideloaderView>,
        mode: SettingsMode,
        theme_preference: ThemePreference,
        render_snapshot: Option<SettingsSnapshot>,
        cx: &mut App,
    ) -> bool {
        let settings = self.settings.clone();
        self.window
            .update(cx, |_, window, cx| {
                settings.update(cx, |settings, cx| {
                    settings.show_request(
                        parent,
                        mode,
                        theme_preference,
                        render_snapshot,
                        window,
                        cx,
                    );
                });
            })
            .is_ok()
    }

    pub(crate) fn sync_from_state(&self, state: &SideloaderState, cx: &mut App) {
        let settings = self.settings.clone();
        let _ = self.window.update(cx, |_, _, cx| {
            settings.update(cx, |settings, cx| {
                settings.refresh_from_state(state, cx);
            });
        });
    }
}

enum SettingsParentAction {
    AddDeveloperAccount(Box<AccountOption>),
    SelectTeam(usize),
    SelectCertificate(usize),
    SelectAppId(usize),
    SetAutoAppId(bool),
    ReplaceDeveloperAccount(Box<AccountOption>),
    LogOutSelectedDeveloperAccount,
    SetThemePreference(ThemePreference),
    SelectAdiBackend(usize),
    ReplaceAdiBackends {
        backends: Box<[AdiBackendOption]>,
        selected: usize,
        persist: bool,
    },
    ReplaceAndroidDeviceIdentity(MachineIdentity),
    ReplaceApp {
        app_index: usize,
        app: Box<AppOption>,
    },
}

enum SettingsParentActionResult {
    None,
    Mode(Option<SettingsMode>),
}

#[derive(Clone, Copy)]
struct ParentWindowUnavailable;

impl ParentWindowUnavailable {
    fn user_message(self) -> &'static str {
        "The main sideloader window is not available."
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeveloperLoginStep {
    Credentials,
    SecondaryAction,
}

struct DeveloperLoginState {
    email: Entity<InputState>,
    password: Entity<InputState>,
    code: Entity<OtpState>,
    remember_account: bool,
    step: DeveloperLoginStep,
    secondary_action_detail: SharedString,
    error: Option<SharedString>,
    busy: bool,
}

impl DeveloperLoginState {
    fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        Self {
            email: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("appleid@example.com")
                    .clean_on_escape()
            }),
            password: cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("Password")
                    .clean_on_escape()
            }),
            code: cx.new(|cx| OtpState::new(6, window, cx)),
            remember_account: true,
            step: DeveloperLoginStep::Credentials,
            secondary_action_detail: "".into(),
            error: None,
            busy: false,
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

    fn value(self, identity: &MachineIdentity) -> &String {
        match self {
            Self::MachineModel => &identity.machine_name,
            Self::OsName => &identity.os_name,
            Self::OsVersion => &identity.os_version,
            Self::MachineId => &identity.machine_id,
        }
    }

    fn set_value(self, identity: &mut MachineIdentity, value: String) {
        match self {
            Self::MachineModel => identity.machine_name = value,
            Self::OsName => identity.os_name = value,
            Self::OsVersion => identity.os_version = value,
            Self::MachineId => identity.machine_id = value,
        }
    }
}

#[derive(Clone)]
struct MachineIdentityEdit {
    field: MachineIdentityField,
    input: Entity<InputState>,
}

#[derive(Clone)]
struct AppIdAddForm {
    team_id: String,
    identifier: Entity<InputState>,
    name: Entity<InputState>,
}

#[derive(Clone)]
struct AppIdEditForm {
    team_id: String,
    app_id_id: String,
    original_name: String,
    name: Entity<InputState>,
    capabilities: Vec<AppIdCapabilityEdit>,
}

#[derive(Clone)]
struct AppIdCapabilityEdit {
    key: String,
    label: String,
    detail: String,
    enabled: bool,
}

#[derive(Clone)]
enum AppDetailEdit {
    Text {
        field: AppMetadataField,
        input: Entity<InputState>,
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
        input: Entity<InputState>,
    },
    Boolean {
        row: usize,
        value: bool,
    },
    Array {
        row: usize,
        items: Vec<Entity<InputState>>,
    },
    Type {
        row: usize,
        selected: String,
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
impl SettingsMode {
    fn title(&self) -> &'static str {
        match self {
            SettingsMode::DeveloperLogin => "Add Apple Account",
            SettingsMode::Team => "Developer Settings",
            SettingsMode::AppSettings { .. } => "App Settings",
            SettingsMode::AdiSettings => "Settings",
        }
    }
}

fn settings_task_is_current(
    current_generation: u64,
    current_mode: &SettingsMode,
    task_generation: u64,
    expected_mode: &SettingsMode,
) -> bool {
    current_generation == task_generation && current_mode == expected_mode
}

impl SettingsSnapshot {
    fn from_state(mode: &SettingsMode, state: &SideloaderState) -> Option<Self> {
        match mode {
            SettingsMode::DeveloperLogin => {
                let adi_backend = state
                    .adi_backends
                    .get(state.selected_adi_backend)
                    .and_then(|backend| backend.availability.is_ready().then_some(backend.kind));

                Some(Self::DeveloperLogin(DeveloperLoginSnapshot {
                    adi_backend,
                    machine_identity: state.machine_identity.clone(),
                    android_adi_identifier: state.android_adi_identifier.clone(),
                }))
            }
            SettingsMode::Team => {
                let account = state.selected_account()?;
                Some(Self::Team(TeamSettingsSnapshot {
                    teams: account.teams.clone(),
                    selected_team: state.selected_team,
                    selected_certificate: state.selected_certificate,
                    auto_app_id: state.auto_app_id,
                    selected_app_id: state.selected_app_id,
                }))
            }
            SettingsMode::AppSettings { app_index } => {
                let app = state.app_selection.app(*app_index)?;
                Some(Self::AppSettings(Box::new(AppSettingsSnapshot {
                    app_index: *app_index,
                    app: app.clone(),
                    enabled_patches: state.enabled_patches.clone(),
                    team_id: state
                        .selected_team()
                        .map(|team| team.identifier.clone())
                        .unwrap_or_default(),
                })))
            }
            SettingsMode::AdiSettings => Some(Self::AdiSettings(AdiSettingsSnapshot {
                theme_preference: state.theme_preference,
                backends: state.adi_backends.clone(),
                selected_backend: state.selected_adi_backend,
                machine_identity: state.machine_identity.clone(),
                android_device_identity: state.android_device_identity.clone(),
                android_adi_identifier: state.android_adi_identifier.clone(),
            })),
        }
    }
}

impl SettingsWindow {
    fn new(
        parent: WeakEntity<SideloaderView>,
        mode: SettingsMode,
        theme_preference: ThemePreference,
        render_snapshot: Option<SettingsSnapshot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());
        cx.observe_window_appearance(window, |settings, window, cx| {
            sync_window_theme(window, cx, settings.theme_preference);
            cx.notify();
        })
        .detach();

        Self {
            parent,
            focus_handle,
            mode,
            theme_preference,
            render_snapshot,
            request_generation: 0,
            scroll_handle: ScrollHandle::new(),
            team_picker_open: false,
            certificate_picker_open: false,
            app_id_picker_open: false,
            theme_picker_open: false,
            adi_backend_picker_open: false,
            adi_operation: None,
            machine_identity_edit: None,
            app_detail_edit: None,
            app_id_add_form: None,
            app_id_edit_form: None,
            selected_entitlement: None,
            entitlement_edit: None,
            entitlement_type_picker_open: false,
            app_settings_error: None,
            developer_login: DeveloperLoginState::new(window, cx),
            spinner_turns: 0.,
            team_refreshing: false,
            team_refresh_error: None,
            certificate_error: None,
        }
    }

    fn snapshot(&self, _cx: &mut Context<Self>) -> Option<SettingsSnapshot> {
        self.render_snapshot.clone()
    }

    fn developer_login_snapshot(&self, cx: &mut Context<Self>) -> Option<DeveloperLoginSnapshot> {
        match self.snapshot(cx)? {
            SettingsSnapshot::DeveloperLogin(snapshot) => Some(snapshot),
            SettingsSnapshot::Team(_)
            | SettingsSnapshot::AppSettings(_)
            | SettingsSnapshot::AdiSettings(_) => None,
        }
    }

    fn team_snapshot(&self, cx: &mut Context<Self>) -> Option<TeamSettingsSnapshot> {
        match self.snapshot(cx)? {
            SettingsSnapshot::Team(snapshot) => Some(snapshot),
            SettingsSnapshot::DeveloperLogin(_)
            | SettingsSnapshot::AppSettings(_)
            | SettingsSnapshot::AdiSettings(_) => None,
        }
    }

    fn app_settings_snapshot(&self, cx: &mut Context<Self>) -> Option<AppSettingsSnapshot> {
        match self.snapshot(cx)? {
            SettingsSnapshot::AppSettings(snapshot) => Some(*snapshot),
            SettingsSnapshot::DeveloperLogin(_)
            | SettingsSnapshot::Team(_)
            | SettingsSnapshot::AdiSettings(_) => None,
        }
    }

    fn adi_settings_snapshot(&self, cx: &mut Context<Self>) -> Option<AdiSettingsSnapshot> {
        match self.snapshot(cx)? {
            SettingsSnapshot::AdiSettings(snapshot) => Some(snapshot),
            SettingsSnapshot::DeveloperLogin(_)
            | SettingsSnapshot::Team(_)
            | SettingsSnapshot::AppSettings(_) => None,
        }
    }

    fn theme_preference(&self, _cx: &mut Context<Self>) -> ThemePreference {
        self.theme_preference
    }

    fn refresh_from_state(&mut self, state: &SideloaderState, cx: &mut Context<Self>) {
        self.theme_preference = state.theme_preference;
        self.render_snapshot = SettingsSnapshot::from_state(&self.mode, state);
        cx.notify();
    }

    fn task_mode(&self) -> SettingsMode {
        self.mode.clone()
    }

    fn task_generation(&self) -> u64 {
        self.request_generation
    }

    fn accepts_task_result(&self, generation: u64, expected_mode: &SettingsMode) -> bool {
        settings_task_is_current(
            self.request_generation,
            &self.mode,
            generation,
            expected_mode,
        )
    }

    fn show_request_from_parent(
        &mut self,
        mode: SettingsMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (theme_preference, render_snapshot) = self
            .parent
            .update(cx, |view, _| {
                (
                    view.theme_preference,
                    SettingsSnapshot::from_state(&mode, &view.state),
                )
            })
            .unwrap_or((ThemePreference::default(), None));
        self.show_request(
            self.parent.clone(),
            mode,
            theme_preference,
            render_snapshot,
            window,
            cx,
        );
    }

    fn dispatch_parent_action(
        &mut self,
        action: SettingsParentAction,
        cx: &mut Context<Self>,
    ) -> Result<SettingsParentActionResult, ParentWindowUnavailable> {
        let current_mode = self.mode.clone();
        let current_settings = cx.entity();
        self.parent
            .update(cx, |view, cx| match action {
                SettingsParentAction::AddDeveloperAccount(account) => {
                    let result = SettingsParentActionResult::Mode(
                        view.add_developer_account_from_settings(*account, cx),
                    );
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (result, theme_preference, render_snapshot)
                }
                SettingsParentAction::SelectTeam(index) => {
                    view.select_team_from_settings(index, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::SelectCertificate(index) => {
                    view.select_certificate_from_settings(index, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::SelectAppId(index) => {
                    view.select_app_id_from_settings(index, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::SetAutoAppId(checked) => {
                    view.set_auto_app_id_from_settings(checked, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::ReplaceDeveloperAccount(account) => {
                    let result = SettingsParentActionResult::Mode(
                        view.replace_developer_account_from_settings(*account, cx),
                    );
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (result, theme_preference, render_snapshot)
                }
                SettingsParentAction::LogOutSelectedDeveloperAccount => {
                    let result = SettingsParentActionResult::Mode(
                        view.log_out_selected_developer_account_from_settings(cx),
                    );
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (result, theme_preference, render_snapshot)
                }
                SettingsParentAction::SetThemePreference(preference) => {
                    view.theme_preference = preference;
                    view.save_preferences();
                    for handle in [
                        view.team_settings_window.clone(),
                        view.app_settings_window.clone(),
                        view.adi_settings_window.clone(),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        if handle.settings != current_settings {
                            handle.sync_from_state(&view.state, cx);
                        }
                    }
                    cx.notify();
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::SelectAdiBackend(index) => {
                    view.select_adi_backend_from_settings(index, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::ReplaceAdiBackends {
                    backends,
                    selected,
                    persist,
                } => {
                    view.replace_adi_backends_from_settings(
                        backends.into_vec(),
                        selected,
                        persist,
                        cx,
                    );
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::ReplaceAndroidDeviceIdentity(identity) => {
                    view.replace_android_device_identity_from_settings(identity, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
                SettingsParentAction::ReplaceApp { app_index, app } => {
                    view.replace_app_from_settings(app_index, *app, cx);
                    let theme_preference = view.theme_preference;
                    let render_snapshot = SettingsSnapshot::from_state(&current_mode, &view.state);
                    (
                        SettingsParentActionResult::None,
                        theme_preference,
                        render_snapshot,
                    )
                }
            })
            .map(|(result, theme_preference, render_snapshot)| {
                self.theme_preference = theme_preference;
                self.render_snapshot = render_snapshot;
                result
            })
            .map_err(|_| ParentWindowUnavailable)
    }

    fn selected_developer_context(
        &self,
        cx: &mut Context<Self>,
    ) -> Result<DeveloperSessionContext, String> {
        self.parent
            .update(cx, |view, _| view.selected_developer_context())
            .map_err(|_| ParentWindowUnavailable.user_message().to_string())?
    }

    fn default_developer_app_id_fields(
        &self,
        team_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<(String, String), String> {
        self.parent
            .update(cx, |view, _| view.default_developer_app_id_fields(team_id))
            .map_err(|_| ParentWindowUnavailable.user_message().to_string())?
    }

    fn replace_app_in_parent(&mut self, app_index: usize, app: AppOption, cx: &mut Context<Self>) {
        let _ = self.dispatch_parent_action(
            SettingsParentAction::ReplaceApp {
                app_index,
                app: Box::new(app),
            },
            cx,
        );
    }

    fn show_request(
        &mut self,
        parent: WeakEntity<SideloaderView>,
        mode: SettingsMode,
        theme_preference: ThemePreference,
        render_snapshot: Option<SettingsSnapshot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let reset_login = matches!(&mode, SettingsMode::DeveloperLogin);
        let title = mode.title();
        self.parent = parent;
        self.request_generation = self.request_generation.wrapping_add(1);
        self.mode = mode;
        self.theme_preference = theme_preference;
        self.render_snapshot = render_snapshot;
        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.theme_picker_open = false;
        self.adi_backend_picker_open = false;
        self.machine_identity_edit = None;
        self.app_detail_edit = None;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.selected_entitlement = None;
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        self.app_settings_error = None;
        self.team_refreshing = false;
        self.team_refresh_error = None;
        self.certificate_error = None;
        if reset_login {
            self.developer_login = DeveloperLoginState::new(window, cx);
        }
        window.set_window_title(title);
        window.activate_window();
        window.focus(&self.focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());
        cx.notify();
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
        if !matches!(self.mode, SettingsMode::DeveloperLogin) {
            return;
        };
        if self.developer_login.busy {
            return;
        }
        let Some(snapshot) = self.developer_login_snapshot(cx) else {
            self.developer_login.error = Some(ParentWindowUnavailable.user_message().into());
            cx.notify();
            return;
        };
        let Some(adi_backend) = snapshot.adi_backend else {
            self.developer_login.error = Some(
                "No ADI backend is available. Configure an ADI backend in Settings first.".into(),
            );
            cx.notify();
            return;
        };
        let email = self.developer_login.email.read(cx).value();
        let password = self.developer_login.password.read(cx).value();
        let login_request = DeveloperLoginRequest {
            email: email.to_string(),
            password: password.to_string(),
            remember_account: self.developer_login.remember_account,
            adi_backend,
            machine_identity: snapshot.machine_identity,
            android_adi_identifier: snapshot.android_adi_identifier,
        };

        self.developer_login.busy = true;
        self.developer_login.error = None;
        self.developer_login.secondary_action_detail = "".into();
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.notify();

        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move { app_effects::login(login_request).await })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.developer_login.busy = false;
                match result {
                    Ok(DeveloperLoginOutcome::RequiresSecondaryAction { detail }) => {
                        settings.developer_login.step = DeveloperLoginStep::SecondaryAction;
                        settings.developer_login.secondary_action_detail = detail.into();
                        settings.developer_login.error = None;
                        settings.developer_login.code.update(cx, |code, cx| {
                            code.set_value("", window, cx);
                        });
                        let code = settings.developer_login.code.clone();
                        code.update(cx, |code, cx| code.focus(window, cx));
                        cx.notify();
                    }
                    Ok(DeveloperLoginOutcome::SignedIn(account)) => {
                        settings.finish_developer_login(account, window, cx);
                    }
                    Err(error) => {
                        settings.developer_login.error = Some(error.user_message().into());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn submit_developer_secondary_action(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.submit_developer_secondary_action_inner(window, cx);
    }

    fn submit_developer_secondary_action_inner(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SettingsMode::DeveloperLogin = &self.mode else {
            return;
        };
        window.focus(&self.focus_handle, cx);
        self.developer_login.error = Some(app_effects::secondary_action_not_supported().into());
        cx.notify();
    }

    fn back_to_developer_login(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.mode, SettingsMode::DeveloperLogin) {
            if self.developer_login.busy {
                return;
            }
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
        if !matches!(self.mode, SettingsMode::DeveloperLogin) {
            return;
        }

        if event.keystroke.key.as_str() == "enter" {
            if self.developer_login.busy {
                return;
            }
            cx.stop_propagation();
            window.prevent_default();
            match self.developer_login.step {
                DeveloperLoginStep::Credentials => {
                    self.submit_developer_login_inner(window, cx);
                }
                DeveloperLoginStep::SecondaryAction => {
                    self.submit_developer_secondary_action_inner(window, cx);
                }
            }
        }
    }

    fn finish_developer_login(
        &mut self,
        account: AccountOption,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.dispatch_parent_action(
            SettingsParentAction::AddDeveloperAccount(Box::new(account)),
            cx,
        ) {
            Ok(SettingsParentActionResult::Mode(Some(mode))) => {
                self.show_request_from_parent(mode, window, cx)
            }
            Ok(SettingsParentActionResult::Mode(None)) => {
                self.developer_login.error =
                    Some("The Apple Account has no developer teams.".into());
                cx.notify();
            }
            Ok(_) => {
                self.developer_login.error = Some(
                    "The Apple Account was added, but the settings view did not refresh.".into(),
                );
                cx.notify();
            }
            Err(_) => {
                self.developer_login.error = Some(ParentWindowUnavailable.user_message().into());
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
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };

        if index >= snapshot.teams.len() {
            return;
        }

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        let _ = self.dispatch_parent_action(SettingsParentAction::SelectTeam(index), cx);
        cx.notify();
    }

    fn select_certificate(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };

        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            return;
        };

        if index < team.certificates.len() {
            self.certificate_picker_open = false;
            let _ = self.dispatch_parent_action(SettingsParentAction::SelectCertificate(index), cx);
            cx.notify();
        }
    }

    fn import_certificate_private_key(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.certificate_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let Some(certificate) = team.certificates.get(snapshot.selected_certificate) else {
            self.certificate_error = Some("No development certificate is selected.".into());
            cx.notify();
            return;
        };
        let Some(public_key_fingerprint) = certificate.public_key_fingerprint.clone() else {
            self.certificate_error =
                Some("Refresh developer settings before importing this private key.".into());
            cx.notify();
            return;
        };

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.certificate_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = team.identifier.clone();
        let certificate_id = certificate.id.to_string();
        let public_key_fingerprint = public_key_fingerprint.to_string();

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.certificate_error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import PEM Private Key".into()),
        });
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = settings.update_in(cx, |settings, _, cx| {
                        if !settings.accepts_task_result(task_generation, &task_mode) {
                            return;
                        }
                        settings.certificate_error = Some(error.to_string().into());
                        cx.notify();
                    });
                    return;
                }
            };

            let Some(private_key_path) = paths.into_iter().next() else {
                return;
            };

            let _ = settings.update_in(cx, |settings, _, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.team_refreshing = true;
                settings.certificate_error = None;
                cx.notify();
            });

            let result = cx
                .background_spawn(async move {
                    app_effects::import_certificate_private_key(
                        developer_context,
                        team_id,
                        certificate_id,
                        public_key_fingerprint,
                        private_key_path,
                    )
                    .await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account_for_certificate(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn create_certificate(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.certificate_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.certificate_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = team.identifier.to_string();

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        self.certificate_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::create_certificate(developer_context, team_id).await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account_for_certificate(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn revoke_certificate(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.certificate_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let Some(certificate) = team.certificates.get(snapshot.selected_certificate) else {
            self.certificate_error = Some("No development certificate is selected.".into());
            cx.notify();
            return;
        };

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.certificate_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = team.identifier.to_string();
        let serial_number = certificate.serial_number.to_string();

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        self.certificate_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::revoke_certificate(developer_context, team_id, serial_number).await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account_for_certificate(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn select_app_id(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };

        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            return;
        };

        if index < team.app_ids.len() {
            self.certificate_picker_open = false;
            self.app_id_picker_open = false;
            self.app_id_add_form = None;
            self.app_id_edit_form = None;
            let _ = self.dispatch_parent_action(SettingsParentAction::SelectAppId(index), cx);
            cx.notify();
        }
    }

    fn set_auto_app_id(&mut self, checked: &bool, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if !matches!(self.mode, SettingsMode::Team) {
            return;
        }

        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        let _ = self.dispatch_parent_action(SettingsParentAction::SetAutoAppId(*checked), cx);
        cx.notify();
    }

    fn add_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.submit_app_id_add(window, cx);
    }

    fn set_app_id_add_popover(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        if !open {
            self.app_id_add_form = None;
            cx.notify();
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.team_refresh_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let team_id = team.identifier.to_string();
        let (identifier, name) = match self.default_developer_app_id_fields(&team_id, cx) {
            Ok(defaults) => defaults,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };

        let identifier = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(identifier)
                .clean_on_escape()
        });
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(name)
                .clean_on_escape()
        });
        self.app_id_add_form = Some(AppIdAddForm {
            team_id: team.identifier.clone(),
            identifier,
            name: name.clone(),
        });
        self.app_id_edit_form = None;
        name.update(cx, |name, cx| name.focus(window, cx));
        self.team_refresh_error = None;
        cx.notify();
    }

    fn cancel_app_id_add(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.app_id_add_form = None;
        cx.notify();
    }

    fn set_app_id_edit_popover(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        if !open {
            self.app_id_edit_form = None;
            cx.notify();
            return;
        }
        if snapshot.auto_app_id {
            self.team_refresh_error =
                Some("Turn off automatic App ID selection before editing an App ID.".into());
            cx.notify();
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.team_refresh_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let Some(app_id) = team.app_ids.get(snapshot.selected_app_id) else {
            self.team_refresh_error = Some("No App ID is selected.".into());
            cx.notify();
            return;
        };
        if app_id.developer_id.is_empty() {
            self.team_refresh_error =
                Some("The selected App ID is missing its Xcode identifier.".into());
            cx.notify();
            return;
        }

        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(app_id.name.to_string())
                .clean_on_escape()
        });
        self.app_id_add_form = None;
        self.app_id_edit_form = Some(AppIdEditForm {
            team_id: team.identifier.clone(),
            app_id_id: app_id.developer_id.clone(),
            original_name: app_id.name.clone(),
            name,
            capabilities: app_id
                .capabilities
                .iter()
                .map(|capability| AppIdCapabilityEdit {
                    key: capability.key.clone(),
                    label: capability.label.clone(),
                    detail: capability.detail.clone(),
                    enabled: capability.enabled,
                })
                .collect(),
        });
        self.team_refresh_error = None;
        cx.notify();
    }

    fn cancel_app_id_edit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.app_id_edit_form = None;
        cx.notify();
    }

    fn toggle_app_id_capability(
        &mut self,
        index: usize,
        checked: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(form) = &mut self.app_id_edit_form {
            if let Some(capability) = form.capabilities.get_mut(index) {
                capability.enabled = checked;
                cx.notify();
            }
        }
    }

    fn submit_app_id_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.mode, SettingsMode::Team) {
            return;
        }
        if self.team_refreshing {
            return;
        }
        let Some(form) = self.app_id_add_form.clone() else {
            return;
        };

        let identifier = form.identifier.read(cx).value().trim().to_string();
        let name = form.name.read(cx).value().trim().to_string();
        if identifier.is_empty() {
            self.team_refresh_error = Some("Enter an App ID identifier.".into());
            cx.notify();
            return;
        }
        if name.is_empty() {
            self.team_refresh_error = Some("Enter an App ID name.".into());
            cx.notify();
            return;
        }

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = form.team_id.to_string();

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::add_app_id(developer_context, team_id, identifier, name).await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn submit_app_id_update(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if !matches!(self.mode, SettingsMode::Team) {
            return;
        }
        if self.team_refreshing {
            return;
        }
        let Some(form) = self.app_id_edit_form.clone() else {
            return;
        };

        let name = form.name.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.team_refresh_error = Some("Enter an App ID name.".into());
            cx.notify();
            return;
        }
        let renamed = name != form.original_name.as_str();
        let capabilities: Vec<DeveloperAppIdCapabilityUpdate> = form
            .capabilities
            .iter()
            .map(|capability| DeveloperAppIdCapabilityUpdate {
                key: capability.key.to_string(),
                enabled: capability.enabled,
            })
            .collect();

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = form.team_id.to_string();
        let app_id_id = form.app_id_id.to_string();
        let name = renamed.then_some(name);

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::update_app_id(
                        developer_context,
                        team_id,
                        app_id_id,
                        name,
                        capabilities,
                    )
                    .await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn remove_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.team_refresh_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let Some(app_id) = team.app_ids.get(snapshot.selected_app_id) else {
            self.team_refresh_error = Some("No App ID is selected.".into());
            cx.notify();
            return;
        };
        let team_id = team.identifier.to_string();
        let app_id_id = app_id.developer_id.to_string();
        if app_id_id.is_empty() {
            self.team_refresh_error =
                Some("The selected App ID is missing its Xcode identifier.".into());
            cx.notify();
            return;
        }
        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::delete_app_id(developer_context, team_id, app_id_id).await
                })
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn save_mobileprovision(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some(snapshot) = self.team_snapshot(cx) else {
            return;
        };
        if self.team_refreshing {
            return;
        }
        if snapshot.auto_app_id {
            self.team_refresh_error =
                Some("Turn off automatic App ID selection before saving a profile.".into());
            cx.notify();
            return;
        }
        let Some(team) = snapshot.teams.get(snapshot.selected_team) else {
            self.team_refresh_error = Some("No developer team is selected.".into());
            cx.notify();
            return;
        };
        let Some(app_id) = team.app_ids.get(snapshot.selected_app_id) else {
            self.team_refresh_error = Some("No App ID is selected.".into());
            cx.notify();
            return;
        };
        if app_id.developer_id.is_empty() {
            self.team_refresh_error =
                Some("The selected App ID is missing its Xcode identifier.".into());
            cx.notify();
            return;
        }

        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        let team_id = team.identifier.to_string();
        let app_id_id = app_id.developer_id.to_string();

        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refresh_error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Save Mobile Provision".into()),
        });
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn(async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = settings.update(cx, |settings, cx| {
                        if !settings.accepts_task_result(task_generation, &task_mode) {
                            return;
                        }
                        settings.team_refresh_error = Some(error.to_string().into());
                        cx.notify();
                    });
                    return;
                }
            };

            let Some(folder) = paths.into_iter().next() else {
                return;
            };

            let _ = settings.update(cx, |settings, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.team_refreshing = true;
                settings.team_refresh_error = None;
                cx.notify();
            });

            let result = cx
                .background_spawn(async move {
                    let profile = app_effects::download_provisioning_profile(
                        developer_context,
                        team_id,
                        app_id_id,
                    )
                    .await
                    .map_err(|error| error.user_message())?;
                    app_effects::save_provisioning_profile(folder, profile)
                        .map_err(|error| error.user_message())
                })
                .await;

            let _ = settings.update(cx, |settings, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.team_refreshing = false;
                if let Err(error) = result {
                    settings.team_refresh_error = Some(error.into());
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn refresh_team_details(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if !matches!(self.mode, SettingsMode::Team) {
            return;
        }
        if self.team_refreshing {
            return;
        }
        let developer_context = match self.selected_developer_context(cx) {
            Ok(context) => context,
            Err(error) => {
                self.team_refresh_error = Some(error.into());
                cx.notify();
                return;
            }
        };
        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        self.app_id_add_form = None;
        self.app_id_edit_form = None;
        self.team_refreshing = true;
        self.team_refresh_error = None;
        self.certificate_error = None;
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn_in(window, async move |settings, cx| {
            let result = cx
                .background_spawn(
                    async move { app_effects::refresh_account(developer_context).await },
                )
                .await;
            let _ = settings.update_in(cx, |settings, window, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.apply_refreshed_developer_account(result, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_refreshed_developer_account(
        &mut self,
        result: Result<AccountOption, AppError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.team_refreshing = false;
        match result {
            Ok(account) => match self.dispatch_parent_action(
                SettingsParentAction::ReplaceDeveloperAccount(Box::new(account)),
                cx,
            ) {
                Ok(SettingsParentActionResult::Mode(Some(mode))) => {
                    self.show_request_from_parent(mode, window, cx)
                }
                Ok(SettingsParentActionResult::Mode(None)) => {
                    self.team_refresh_error =
                        Some("The refreshed account is no longer selected.".into());
                    cx.notify();
                }
                Ok(_) => {
                    self.team_refresh_error = Some(
                        "The refreshed account did not return updated developer settings.".into(),
                    );
                    cx.notify();
                }
                Err(_) => {
                    self.team_refresh_error = Some(ParentWindowUnavailable.user_message().into());
                    cx.notify();
                }
            },
            Err(error) => {
                self.team_refresh_error = Some(error.user_message().into());
                cx.notify();
            }
        }
    }

    fn apply_refreshed_developer_account_for_certificate(
        &mut self,
        result: Result<AccountOption, AppError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.team_refreshing = false;
        match result {
            Ok(account) => match self.dispatch_parent_action(
                SettingsParentAction::ReplaceDeveloperAccount(Box::new(account)),
                cx,
            ) {
                Ok(SettingsParentActionResult::Mode(Some(mode))) => {
                    self.show_request_from_parent(mode, window, cx)
                }
                Ok(SettingsParentActionResult::Mode(None)) => {
                    self.certificate_error =
                        Some("The refreshed account is no longer selected.".into());
                    cx.notify();
                }
                Ok(_) => {
                    self.certificate_error = Some(
                        "The refreshed account did not return updated developer settings.".into(),
                    );
                    cx.notify();
                }
                Err(_) => {
                    self.certificate_error = Some(ParentWindowUnavailable.user_message().into());
                    cx.notify();
                }
            },
            Err(error) => {
                self.certificate_error = Some(error.user_message().into());
                cx.notify();
            }
        }
    }

    fn log_out_developer_account(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if !matches!(self.mode, SettingsMode::Team) {
            return;
        }
        self.team_picker_open = false;
        self.certificate_picker_open = false;
        self.app_id_picker_open = false;
        match self.dispatch_parent_action(SettingsParentAction::LogOutSelectedDeveloperAccount, cx)
        {
            Ok(SettingsParentActionResult::Mode(Some(mode))) => {
                self.show_request_from_parent(mode, window, cx)
            }
            Ok(SettingsParentActionResult::Mode(None)) => window.remove_window(),
            Ok(_) => {
                self.developer_login.error =
                    Some("The logout action did not return updated developer settings.".into());
                cx.notify();
            }
            Err(_) => {
                self.developer_login.error = Some(ParentWindowUnavailable.user_message().into());
                cx.notify();
            }
        }
    }

    fn select_theme_preference(
        &mut self,
        preference: ThemePreference,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.theme_picker_open = false;
        if !matches!(self.mode, SettingsMode::AdiSettings) {
            return;
        }

        sync_window_theme(window, cx, preference);
        let _ =
            self.dispatch_parent_action(SettingsParentAction::SetThemePreference(preference), cx);
        cx.notify();
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
        let Some(snapshot) = self.adi_settings_snapshot(cx) else {
            return;
        };

        if index >= snapshot.backends.len() {
            return;
        }

        self.adi_backend_picker_open = false;
        let _ = self.dispatch_parent_action(SettingsParentAction::SelectAdiBackend(index), cx);
        cx.notify();
    }

    fn open_data_folder(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.adi_backend_picker_open = false;
        if let Err(error) = app_effects::open_app_data_folder() {
            log::warn!("{}", error.user_message());
        }
        cx.notify();
    }

    fn repair_adi_backend(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.adi_operation.is_some() {
            return;
        }
        let Some((kind, _, _)) = self.selected_adi_backend_context(cx) else {
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
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        let progress_task_mode = task_mode.clone();
        cx.spawn(async move |settings, cx| {
            while let Some(event) = progress_receiver.next().await {
                let _ = settings.update(cx, |settings, cx| {
                    if !settings.accepts_task_result(task_generation, &progress_task_mode) {
                        return;
                    }
                    settings.set_coreadi_install_event(event, cx);
                });
            }
        })
        .detach();
        cx.spawn(async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::download_and_install_coreadi(move |event| {
                        let _ = progress_sender.unbounded_send(event);
                    })
                    .await
                })
                .await;

            let _ = settings.update(cx, |settings, cx| match result {
                Ok(_) if settings.accepts_task_result(task_generation, &task_mode) => {
                    settings.adi_operation = None;
                    settings.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx)
                }
                Err(error) if settings.accepts_task_result(task_generation, &task_mode) => {
                    settings.adi_operation = None;
                    settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.user_message()),
                        cx,
                    );
                }
                _ => {}
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

        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn(async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = settings.update(cx, |settings, cx| {
                        if !settings.accepts_task_result(task_generation, &task_mode) {
                            return;
                        }
                        settings.set_selected_adi_provisioning_state(
                            AdiProvisioningState::Error(error.to_string()),
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
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.adi_operation = Some(AdiOperationState::InstallingCoreAdi);
                cx.notify();
            });
            let result = cx
                .background_spawn(
                    async move { app_effects::install_coreadi_from_apk(apk_path).await },
                )
                .await;
            let _ = settings.update(cx, |settings, cx| match result {
                Ok(_) if settings.accepts_task_result(task_generation, &task_mode) => {
                    settings.adi_operation = None;
                    settings.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx)
                }
                Err(error) if settings.accepts_task_result(task_generation, &task_mode) => {
                    settings.adi_operation = None;
                    settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.user_message()),
                        cx,
                    );
                }
                _ => {}
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
            self.selected_adi_backend_context(cx)
        else {
            return;
        };

        match app_effects::erase_adi_provisioning(kind, &android_adi_identifier) {
            Ok(()) => self.refresh_adi_backend_options(Some(kind), cx),
            Err(error) => self.set_selected_adi_provisioning_state(
                AdiProvisioningState::Error(error.user_message()),
                cx,
            ),
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
            self.selected_adi_backend_context(cx)
        else {
            return;
        };

        self.adi_backend_picker_open = false;
        self.adi_operation = Some(AdiOperationState::Provisioning);
        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn(async move |settings, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::provision_adi(kind, &machine_identity, &android_adi_identifier)
                        .await
                })
                .await;
            let _ = settings.update(cx, |settings, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                settings.adi_operation = None;
                match result {
                    Ok(()) => settings.refresh_adi_backend_options(Some(kind), cx),
                    Err(error) => settings.set_selected_adi_provisioning_state(
                        AdiProvisioningState::Error(error.user_message()),
                        cx,
                    ),
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn selected_adi_backend_context(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<(AdiBackendKind, MachineIdentity, String)> {
        let snapshot = self.adi_settings_snapshot(cx)?;

        snapshot
            .backends
            .get(snapshot.selected_backend)
            .map(|backend| {
                let machine_identity = match backend.kind {
                    AdiBackendKind::AndroidCoreAdi => snapshot.android_device_identity.clone(),
                    AdiBackendKind::SystemAdid | AdiBackendKind::WindowsCoreAdi => {
                        snapshot.machine_identity.clone()
                    }
                };
                (
                    backend.kind,
                    machine_identity,
                    snapshot.android_adi_identifier.clone(),
                )
            })
    }

    fn refresh_adi_backend_options(
        &mut self,
        preferred_kind: Option<AdiBackendKind>,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.adi_settings_snapshot(cx) else {
            return;
        };

        let selected_kind = preferred_kind.or_else(|| {
            snapshot
                .backends
                .get(snapshot.selected_backend)
                .map(|backend| backend.kind)
        });
        let refreshed =
            app_effects::available_adi_backends_with_provisioning(&snapshot.android_adi_identifier);
        let selected = selected_kind
            .and_then(|kind| refreshed.iter().position(|backend| backend.kind == kind))
            .unwrap_or_else(|| app_effects::default_adi_backend(&refreshed));

        let _ = self.dispatch_parent_action(
            SettingsParentAction::ReplaceAdiBackends {
                backends: refreshed.into_boxed_slice(),
                selected,
                persist: true,
            },
            cx,
        );
        cx.notify();
    }

    fn set_selected_adi_provisioning_state(
        &mut self,
        state: AdiProvisioningState,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.adi_settings_snapshot(cx) else {
            return;
        };

        let mut backends = snapshot.backends;
        if let Some(backend) = backends.get_mut(snapshot.selected_backend) {
            backend.provisioning_state = state.clone();
        }

        let _ = self.dispatch_parent_action(
            SettingsParentAction::ReplaceAdiBackends {
                backends: backends.into_boxed_slice(),
                selected: snapshot.selected_backend,
                persist: false,
            },
            cx,
        );
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
        let Some(value) = self.editable_machine_identity_value(field, cx) else {
            return;
        };
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(value)
                .clean_on_escape()
        });
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
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let snapshot = self.adi_settings_snapshot(cx)?;

        let backend = snapshot.backends.get(snapshot.selected_backend)?;
        if backend.kind != AdiBackendKind::AndroidCoreAdi || !backend.editable_identity {
            return None;
        }

        Some(field.value(&snapshot.android_device_identity).clone())
    }

    fn save_machine_identity_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.machine_identity_edit.take() else {
            return;
        };

        let mut changed_identity = None;
        let mut should_erase_provisioning = false;
        let mut android_adi_identifier = String::new();
        let mut error = None;

        let Some(snapshot) = self.adi_settings_snapshot(cx) else {
            cx.notify();
            return;
        };
        let Some(backend) = snapshot.backends.get(snapshot.selected_backend) else {
            cx.notify();
            return;
        };
        if backend.kind != AdiBackendKind::AndroidCoreAdi || !backend.editable_identity {
            cx.notify();
            return;
        }

        let mut android_device_identity = snapshot.android_device_identity;
        let new_value = edit.input.read(cx).value().to_string();
        let old_value = edit.field.value(&android_device_identity).to_string();
        if old_value != new_value {
            edit.field
                .set_value(&mut android_device_identity, new_value);
            changed_identity = Some(android_device_identity);
            should_erase_provisioning = matches!(
                backend.provisioning_state,
                AdiProvisioningState::Provisioned
            );
            android_adi_identifier = snapshot.android_adi_identifier;
        }

        if let Some(identity) = changed_identity {
            let _ = self.dispatch_parent_action(
                SettingsParentAction::ReplaceAndroidDeviceIdentity(identity),
                cx,
            );

            if should_erase_provisioning {
                error = app_effects::erase_adi_provisioning(
                    AdiBackendKind::AndroidCoreAdi,
                    &android_adi_identifier,
                )
                .map_err(|error| error.user_message())
                .err();
            }

            self.refresh_adi_backend_options(Some(AdiBackendKind::AndroidCoreAdi), cx);
            if let Some(error) = error {
                self.set_selected_adi_provisioning_state(AdiProvisioningState::Error(error), cx);
            }
        } else {
            cx.notify();
        }
    }

    fn edit_app_icon(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.app_settings_error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select App Icon".into()),
        });

        let task_generation = self.task_generation();
        let task_mode = self.task_mode();
        cx.spawn(async move |settings, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    log::warn!("Failed to select app icon: {error}");
                    let _ = settings.update(cx, |settings, cx| {
                        if !settings.accepts_task_result(task_generation, &task_mode) {
                            return;
                        }
                        settings.app_settings_error =
                            Some(format!("Failed to select app icon: {error}").into());
                        cx.notify();
                    });
                    return;
                }
            };

            let Some(path) = paths.into_iter().next() else {
                return;
            };

            let _ = settings.update(cx, |settings, cx| {
                if !settings.accepts_task_result(task_generation, &task_mode) {
                    return;
                }
                let updated_app = settings.app_settings_snapshot(cx).map(|mut snapshot| {
                    settings.app_settings_error = None;
                    snapshot.app.icon_override_path = Some(path.to_string_lossy().to_string());
                    (snapshot.app_index, snapshot.app)
                });

                if let Some((app_index, updated)) = updated_app {
                    settings.replace_app_in_parent(app_index, updated, cx);
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
        let Some(snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };
        if field == AppMetadataField::SupportedDevices {
            self.app_detail_edit = Some(AppDetailEdit::SupportedDevices {
                selected: snapshot.app.supported_devices().to_vec(),
            });
        } else {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(snapshot.app.field(field).value().clone())
                    .clean_on_escape()
            });
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

        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            cx.notify();
            return;
        };

        match edit {
            AppDetailEdit::Text { field, input } => {
                let value = input.read(cx).value().to_string();
                snapshot.app.set_field_override(field, value);
            }
            AppDetailEdit::SupportedDevices { selected } => {
                snapshot.app.set_supported_devices_override(selected);
            }
        }
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
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
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };
        snapshot.app.clear_field_override(field);
        if self
            .app_detail_edit
            .as_ref()
            .is_some_and(|edit| edit.field() == field)
        {
            self.app_detail_edit = None;
        }
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
        cx.notify();
    }

    fn add_entitlement(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };

        let mut entitlements = crate::app::entitlements::effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
        entitlements.push(AppEntitlement {
            key: "new.entitlement".into(),
            value: EntitlementValue::String("".into()),
        });
        let selected = entitlements.len().saturating_sub(1);
        snapshot.app.entitlement_overrides = Some(entitlements);
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("new.entitlement")
                .clean_on_escape()
        });
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
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };

        let mut entitlements = crate::app::entitlements::effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
        if entitlements.is_empty() {
            return;
        }
        let index = self
            .selected_entitlement
            .filter(|index| *index < entitlements.len())
            .unwrap_or_else(|| entitlements.len() - 1);
        entitlements.remove(index);
        let entitlements_len = entitlements.len();
        snapshot.app.entitlement_overrides = Some(entitlements);
        self.selected_entitlement = if entitlements_len == 0 {
            None
        } else {
            Some(index.min(entitlements_len - 1))
        };
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
        cx.notify();
    }

    fn revert_entitlements(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };
        snapshot.app.entitlement_overrides = None;
        self.selected_entitlement = None;
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
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
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };

        let mut entitlements = crate::app::entitlements::effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
        if row >= entitlements.len() {
            return;
        }

        if let Some(default) = entitlement_revert_target(
            &entitlements[row],
            row,
            &crate::app::entitlements::default_effective_entitlements_for_app(
                &snapshot.app,
                &snapshot.team_id,
            ),
        ) {
            entitlements[row] = default;
        } else {
            entitlements.remove(row);
        }

        let defaults = crate::app::entitlements::default_effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
        snapshot.app.entitlement_overrides = (entitlements != defaults).then_some(entitlements);
        let selected_entitlement =
            snapshot
                .app
                .entitlement_overrides
                .as_ref()
                .and_then(|entitlements| {
                    (!entitlements.is_empty()).then_some(row.min(entitlements.len() - 1))
                });
        self.selected_entitlement = selected_entitlement;
        self.entitlement_edit = None;
        self.entitlement_type_picker_open = false;
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
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
        let Some(snapshot) = self.app_settings_snapshot(cx) else {
            return;
        };
        let entitlements = crate::app::entitlements::effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
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
                        .map(|value| {
                            cx.new(|cx| {
                                InputState::new(window, cx)
                                    .default_value(value)
                                    .clean_on_escape()
                            })
                        })
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
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(value)
                .clean_on_escape()
        });
        self.entitlement_edit = Some(EntitlementEdit::Text {
            row,
            field,
            input: input.clone(),
        });
        input.update(cx, |input, cx| input.focus(window, cx));
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
        let input = cx.new(|cx| InputState::new(window, cx).clean_on_escape());
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
        let Some(mut snapshot) = self.app_settings_snapshot(cx) else {
            cx.notify();
            return;
        };

        let row = edit.row();
        let mut entitlements = crate::app::entitlements::effective_entitlements_for_app(
            &snapshot.app,
            &snapshot.team_id,
        );
        let Some(entitlement) = entitlements.get_mut(row) else {
            cx.notify();
            return;
        };

        match edit {
            EntitlementEdit::Text { field, input, .. } => {
                let value = input.read(cx).value();
                match field {
                    EntitlementField::Key => entitlement.key = value.to_string(),
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
                entitlement.value = entitlement.value.with_type_label(selected.as_str());
            }
        }

        snapshot.app.entitlement_overrides = Some(entitlements);
        self.selected_entitlement = Some(row);
        self.replace_app_in_parent(snapshot.app_index, snapshot.app, cx);
        cx.notify();
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::app::models::{
        AdiBackendAvailability, AppMetadata, EntitlementsSource, PatchOption, SideloadOperation,
    };
    use crate::app::selection::AppSelection;
    use crate::device_selection::DeviceSelection;
    use std::path::Path;

    fn sample_app(name: &str) -> AppOption {
        AppOption {
            metadata: AppMetadata::sample(
                name,
                "com.example.app",
                "1.0",
                "1",
                name,
                "16.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            path: format!("/tmp/{name}.ipa"),
            icon_path: None,
            icon_override_path: None,
            entitlements: Vec::new(),
            entitlements_source: EntitlementsSource::GeneratedFallback,
            entitlement_overrides: None,
            patches: Vec::<PatchOption>::new(),
        }
    }

    fn sample_team(name: &str) -> TeamOption {
        TeamOption {
            name: name.to_string(),
            identifier: "TEAM123".to_string(),
            role: "Admin".to_string(),
            app_id_available_quantity: Some(5),
            app_id_max_quantity: Some(10),
            app_ids: vec![AppIdOption {
                developer_id: "app-id-1".to_string(),
                name: "App ID".to_string(),
                identifier: "TEAM123.com.example.app".to_string(),
                kind: "Explicit".to_string(),
                capabilities: Vec::new(),
            }],
            certificates: vec![DevelopmentCertificateOption {
                id: "cert-1".to_string(),
                name: "Certificate".to_string(),
                serial_number: "SERIAL".to_string(),
                machine_name: "Mac".to_string(),
                private_key_available: true,
                public_key_fingerprint: Some("fingerprint".to_string()),
            }],
        }
    }

    fn sample_identity(machine_id: &str) -> MachineIdentity {
        MachineIdentity {
            machine_name: "Mac".to_string(),
            os_name: "macOS".to_string(),
            os_version: "15.0".to_string(),
            machine_id: machine_id.to_string(),
        }
    }

    fn sample_backend() -> AdiBackendOption {
        AdiBackendOption {
            kind: AdiBackendKind::AndroidCoreAdi,
            name: "Android CoreADI".to_string(),
            detail: String::new(),
            availability: AdiBackendAvailability::Ready,
            details: Vec::new(),
            provisioning_state: AdiProvisioningState::Unknown,
            editable_identity: true,
            repair_action: None,
        }
    }

    fn sample_state() -> SideloaderState {
        let mut app_selection = AppSelection::default();
        app_selection.finish_loading(Path::new("/tmp/Original.ipa"), Ok(sample_app("Original")));

        SideloaderState {
            theme_preference: ThemePreference::System,
            accounts: vec![AccountOption {
                id: "account-1".to_string(),
                label: "Account".to_string(),
                apple_id: "user@example.com".to_string(),
                detail: "1 developer team".to_string(),
                status: "Ready".to_string(),
                teams: vec![sample_team("Original Team")],
            }],
            app_selection,
            device_selection: DeviceSelection::default(),
            adi_backends: vec![sample_backend()],
            selected_account: 0,
            selected_team: 0,
            selected_certificate: 0,
            auto_app_id: false,
            selected_app_id: 0,
            selected_adi_backend: 0,
            machine_identity: sample_identity("HOST"),
            android_device_identity: sample_identity("ANDROID"),
            android_adi_identifier: "0123456789abcdef".to_string(),
            enabled_patches: Vec::new(),
            sideload_operation: SideloadOperation::Idle,
        }
    }

    #[test]
    fn settings_task_generation_rejects_stale_results() {
        let team = SettingsMode::Team;
        let app = SettingsMode::AppSettings { app_index: 0 };

        assert!(settings_task_is_current(7, &team, 7, &team));
        assert!(!settings_task_is_current(8, &team, 7, &team));
        assert!(!settings_task_is_current(7, &team, 7, &app));
        assert!(!settings_task_is_current(
            7,
            &SettingsMode::AppSettings { app_index: 1 },
            7,
            &app,
        ));
    }

    #[test]
    fn settings_snapshots_are_derived_from_latest_parent_state() {
        let mut state = sample_state();

        let Some(SettingsSnapshot::Team(before_team)) =
            SettingsSnapshot::from_state(&SettingsMode::Team, &state)
        else {
            panic!("team snapshot should be available");
        };
        assert_eq!(before_team.teams[0].name, "Original Team");

        state.accounts[0].teams[0].name = "Fresh Team".to_string();
        let Some(SettingsSnapshot::Team(after_team)) =
            SettingsSnapshot::from_state(&SettingsMode::Team, &state)
        else {
            panic!("team snapshot should be available");
        };
        assert_eq!(after_team.teams[0].name, "Fresh Team");

        let app_mode = SettingsMode::AppSettings { app_index: 0 };
        let Some(SettingsSnapshot::AppSettings(before_app)) =
            SettingsSnapshot::from_state(&app_mode, &state)
        else {
            panic!("app snapshot should be available");
        };
        assert_eq!(before_app.app.name(), "Original");

        assert!(state.replace_app(0, sample_app("Fresh")));
        let Some(SettingsSnapshot::AppSettings(after_app)) =
            SettingsSnapshot::from_state(&app_mode, &state)
        else {
            panic!("app snapshot should be available");
        };
        assert_eq!(after_app.app.name(), "Fresh");
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_preference = self.theme_preference(cx);
        sync_window_theme(window, cx, theme_preference);

        if self.adi_operation.is_some() || self.team_refreshing {
            self.spinner_turns = (self.spinner_turns + 0.035) % 1.;
            window.request_animation_frame();
        } else {
            self.spinner_turns = 0.;
        }

        let team_picker_open = self.team_picker_open;
        let certificate_picker_open = self.certificate_picker_open;
        let app_id_picker_open = self.app_id_picker_open;
        let theme_picker_open = self.theme_picker_open;
        let team_refreshing = self.team_refreshing;
        let team_refresh_error = self.team_refresh_error.clone();
        let certificate_error = self.certificate_error.clone();
        let adi_operation = self.adi_operation.clone();
        let adi_busy = adi_operation.is_some();
        let adi_backend_picker_open = self.adi_backend_picker_open && !adi_busy;
        let spinner_turns = self.spinner_turns;
        let machine_identity_edit = self.machine_identity_edit.clone();
        let app_detail_edit = self.app_detail_edit.clone();
        let app_settings_error = self.app_settings_error.clone();
        let app_id_add_form = self.app_id_add_form.clone();
        let app_id_edit_form = self.app_id_edit_form.clone();
        let selected_entitlement = self.selected_entitlement;
        let entitlement_edit = self.entitlement_edit.clone();
        let entitlement_type_picker_open = self.entitlement_type_picker_open;
        let snapshot = self.snapshot(cx);

        match snapshot {
            Some(SettingsSnapshot::DeveloperLogin(_)) => developer_login_view::render(
                &self.focus_handle,
                &self.scroll_handle,
                &self.developer_login,
                cx,
            ),
            Some(SettingsSnapshot::Team(snapshot)) => developer_view::render(
                developer_view::DeveloperViewProps {
                    focus_handle: &self.focus_handle,
                    scroll_handle: &self.scroll_handle,
                    teams: &snapshot.teams,
                    selected_team: snapshot.selected_team,
                    selected_certificate: snapshot.selected_certificate,
                    auto_app_id: snapshot.auto_app_id,
                    selected_app_id: snapshot.selected_app_id,
                    team_picker_open,
                    certificate_picker_open,
                    app_id_picker_open,
                    app_id_add_form: app_id_add_form.as_ref(),
                    app_id_edit_form: app_id_edit_form.as_ref(),
                    team_refreshing,
                    team_refresh_error,
                    certificate_error,
                    spinner_turns,
                },
                cx,
            ),
            Some(SettingsSnapshot::AppSettings(snapshot)) => app_view::render(
                app_view::AppViewProps {
                    focus_handle: &self.focus_handle,
                    scroll_handle: &self.scroll_handle,
                    app: &snapshot.app,
                    enabled_patches: &snapshot.enabled_patches,
                    team_id: &snapshot.team_id,
                    app_detail_edit: app_detail_edit.as_ref(),
                    selected_entitlement,
                    entitlement_edit: entitlement_edit.as_ref(),
                    entitlement_type_picker_open,
                    operation_error: app_settings_error,
                },
                cx,
            ),
            Some(SettingsSnapshot::AdiSettings(snapshot)) => adi_view::render(
                adi_view::AdiViewProps {
                    focus_handle: &self.focus_handle,
                    scroll_handle: &self.scroll_handle,
                    theme_preference: snapshot.theme_preference,
                    backends: &snapshot.backends,
                    selected_backend: snapshot.selected_backend,
                    machine_identity: &snapshot.machine_identity,
                    android_device_identity: &snapshot.android_device_identity,
                    theme_picker_open,
                    adi_backend_picker_open,
                    adi_operation: adi_operation.as_ref(),
                    spinner_turns,
                    machine_identity_edit: machine_identity_edit.as_ref(),
                },
                cx,
            ),
            None => settings_window_shell()
                .track_focus(&self.focus_handle)
                .child(settings_window_header(
                    self.mode.title(),
                    ParentWindowUnavailable.user_message(),
                )),
        }
    }
}

pub(crate) fn show_or_open_settings_window(
    handle: Option<SettingsWindowHandle>,
    parent: WeakEntity<SideloaderView>,
    mode: SettingsMode,
    state: &SideloaderState,
    width: f32,
    height: f32,
    cx: &mut App,
) -> SettingsWindowHandle {
    let theme_preference = state.theme_preference;
    let render_snapshot = SettingsSnapshot::from_state(&mode, state);
    if let Some(handle) = handle {
        if handle.show_request(
            parent.clone(),
            mode.clone(),
            theme_preference,
            render_snapshot.clone(),
            cx,
        ) {
            return handle;
        }
    }

    open_settings_window(
        parent,
        mode,
        theme_preference,
        render_snapshot,
        width,
        height,
        cx,
    )
}

fn open_settings_window(
    parent: WeakEntity<SideloaderView>,
    mode: SettingsMode,
    theme_preference: ThemePreference,
    render_snapshot: Option<SettingsSnapshot>,
    width: f32,
    height: f32,
    cx: &mut App,
) -> SettingsWindowHandle {
    let window_size = size(px(width), px(height));
    let bounds = Bounds::centered(None, window_size, cx);
    let title = mode.title();
    let mut settings_entity = None;

    let window = cx
        .open_window(
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
                let settings = cx.new(|cx| {
                    SettingsWindow::new(parent, mode, theme_preference, render_snapshot, window, cx)
                });
                settings_entity = Some(settings.clone());
                cx.new(|cx| Root::new(settings, window, cx))
            },
        )
        .expect("failed to open settings window");

    SettingsWindowHandle {
        window,
        settings: settings_entity.expect("settings window entity was not created"),
    }
}

fn scroll_panel(
    scroll_id: &'static str,
    scroll_handle: &ScrollHandle,
    content: impl IntoElement,
) -> gpui::Div {
    div()
        .min_h_0()
        .flex_1()
        .relative()
        .child(
            div()
                .id(scroll_id)
                .min_w_0()
                .h_full()
                .w_full()
                .overflow_y_scroll()
                .track_scroll(scroll_handle)
                .child(div().min_w_0().pr_5().child(content)),
        )
        .vertical_scrollbar(scroll_handle)
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
        .child(div().min_w_0().flex_1().child(header))
        .child(action)
}

fn open_data_folder_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("open-data-folder")
        .flex_none()
        .h_8()
        .cursor_pointer()
        .on_click(cx.listener(SettingsWindow::open_data_folder))
        .child(action_button_surface(
            "Open Data",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
            Some(lucide_icon_tinted("icons/folder-open.svg", 0x53666d)),
        ))
}

fn settings_label(label: &'static str) -> gpui::Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x66767c))
        .child(label)
}

fn machine_identity_editor(input: &Entity<InputState>) -> gpui::Div {
    div().min_w_0().flex_1().child(Input::new(input).small())
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

fn entitlement_field_value(entitlement: &AppEntitlement, field: EntitlementField) -> String {
    match field {
        EntitlementField::Key => entitlement.key.clone(),
        EntitlementField::ValueType => entitlement.value.type_label().into(),
        EntitlementField::Value => entitlement.value.display_text(),
    }
}
