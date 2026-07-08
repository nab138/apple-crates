use crate::models::DeviceOption;
use crate::preferences::DevicePreferences;

#[derive(Debug, Default)]
pub(crate) struct DeviceSelection {
    devices: Vec<DeviceOption>,
    selected: usize,
    selected_key: Option<(String, String)>,
    refreshing: bool,
    events_watching: bool,
    refresh_generation: u64,
    refresh_error: Option<String>,
}

impl DeviceSelection {
    pub(crate) fn new(preferences: Option<&DevicePreferences>) -> Self {
        Self {
            selected_key: preferences
                .map(|device| (device.udid.clone(), device.connection.clone())),
            ..Self::default()
        }
    }

    pub(crate) fn selected(&self) -> Option<&DeviceOption> {
        self.devices.get(self.selected)
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn len(&self) -> usize {
        self.devices.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub(crate) fn is_refreshing(&self) -> bool {
        self.refreshing
    }

    pub(crate) fn refresh_error(&self) -> Option<&str> {
        self.refresh_error.as_deref()
    }

    pub(crate) fn selected_preferences(&self) -> Option<DevicePreferences> {
        self.selected_key
            .clone()
            .or_else(|| self.selected().map(device_selection_key))
            .map(|(udid, connection)| DevicePreferences { udid, connection })
    }

    pub(crate) fn device(&self, index: usize) -> Option<&DeviceOption> {
        self.devices.get(index)
    }

    pub(crate) fn select(&mut self, index: usize) -> bool {
        let Some(device) = self.devices.get(index) else {
            return false;
        };

        self.selected = index;
        self.selected_key = Some(device_selection_key(device));
        true
    }

    pub(crate) fn begin_refresh(&mut self) -> Option<u64> {
        if self.refreshing {
            return None;
        }

        self.refreshing = true;
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        Some(self.refresh_generation)
    }

    pub(crate) fn finish_refresh(
        &mut self,
        generation: u64,
        result: Result<Vec<DeviceOption>, String>,
    ) {
        if generation != self.refresh_generation {
            return;
        }

        self.refreshing = false;
        match result {
            Ok(devices) => {
                self.refresh_error = None;
                self.replace_devices(devices);
            }
            Err(error) => {
                self.refresh_error = Some(error);
            }
        }
    }

    pub(crate) fn start_events_watch(&mut self) -> bool {
        if self.events_watching {
            return false;
        }

        self.events_watching = true;
        true
    }

    pub(crate) fn finish_events_watch(&mut self) {
        self.events_watching = false;
    }

    pub(crate) fn note_device_event(&mut self) {
        self.refresh_error = None;
    }

    pub(crate) fn fail_events_watch(&mut self, error: String) {
        self.events_watching = false;
        self.refresh_error = Some(error);
    }

    fn replace_devices(&mut self, devices: Vec<DeviceOption>) {
        let selected_key = self
            .selected_key
            .clone()
            .or_else(|| self.selected().map(device_selection_key));
        self.devices = devices;
        self.selected = selected_key
            .and_then(|key| {
                self.devices
                    .iter()
                    .position(|device| device_selection_key(device) == key)
            })
            .unwrap_or(0);

        if self.selected >= self.devices.len() {
            self.selected = 0;
        }

        self.selected_key = self.selected().map(device_selection_key);
    }
}

pub(crate) fn device_selection_key(device: &DeviceOption) -> (String, String) {
    (device.udid.to_string(), device.connection.to_string())
}
