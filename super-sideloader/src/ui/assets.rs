use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub(crate) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let bytes = match path {
            "icons/chevron-down.svg" => {
                include_bytes!("../../assets/icons/chevron-down.svg").as_slice()
            }
            "icons/chevron-up.svg" => {
                include_bytes!("../../assets/icons/chevron-up.svg").as_slice()
            }
            "icons/settings-2.svg" => {
                include_bytes!("../../assets/icons/settings-2.svg").as_slice()
            }
            "icons/settings.svg" => include_bytes!("../../assets/icons/settings.svg").as_slice(),
            "icons/wrench.svg" => include_bytes!("../../assets/icons/wrench.svg").as_slice(),
            "icons/plus.svg" => include_bytes!("../../assets/icons/plus.svg").as_slice(),
            "icons/minus.svg" => include_bytes!("../../assets/icons/minus.svg").as_slice(),
            "icons/pencil.svg" => include_bytes!("../../assets/icons/pencil.svg").as_slice(),
            "icons/check.svg" => include_bytes!("../../assets/icons/check.svg").as_slice(),
            "icons/x.svg" => include_bytes!("../../assets/icons/x.svg").as_slice(),
            "icons/refresh-cw.svg" => {
                include_bytes!("../../assets/icons/refresh-cw.svg").as_slice()
            }
            "icons/rotate-ccw.svg" => {
                include_bytes!("../../assets/icons/rotate-ccw.svg").as_slice()
            }
            "icons/arrow-right.svg" => {
                include_bytes!("../../assets/icons/arrow-right.svg").as_slice()
            }
            "icons/log-out.svg" => include_bytes!("../../assets/icons/log-out.svg").as_slice(),
            "icons/folder-open.svg" => {
                include_bytes!("../../assets/icons/folder-open.svg").as_slice()
            }
            "icons/download.svg" => include_bytes!("../../assets/icons/download.svg").as_slice(),
            "icons/key-round.svg" => include_bytes!("../../assets/icons/key-round.svg").as_slice(),
            "icons/trash-2.svg" => include_bytes!("../../assets/icons/trash-2.svg").as_slice(),
            _ => return Ok(None),
        };

        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path == "icons" {
            Ok(vec![
                "chevron-down.svg".into(),
                "chevron-up.svg".into(),
                "settings-2.svg".into(),
                "settings.svg".into(),
                "wrench.svg".into(),
                "plus.svg".into(),
                "minus.svg".into(),
                "pencil.svg".into(),
                "check.svg".into(),
                "x.svg".into(),
                "refresh-cw.svg".into(),
                "rotate-ccw.svg".into(),
                "arrow-right.svg".into(),
                "log-out.svg".into(),
                "folder-open.svg".into(),
                "download.svg".into(),
                "key-round.svg".into(),
                "trash-2.svg".into(),
            ])
        } else {
            Ok(Vec::new())
        }
    }
}
