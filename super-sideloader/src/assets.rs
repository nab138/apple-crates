use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub(crate) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let bytes = match path {
            "icons/chevron-down.svg" => {
                include_bytes!("../assets/icons/chevron-down.svg").as_slice()
            }
            "icons/chevron-up.svg" => include_bytes!("../assets/icons/chevron-up.svg").as_slice(),
            "icons/settings-2.svg" => include_bytes!("../assets/icons/settings-2.svg").as_slice(),
            "icons/settings.svg" => include_bytes!("../assets/icons/settings.svg").as_slice(),
            "icons/wrench.svg" => include_bytes!("../assets/icons/wrench.svg").as_slice(),
            "icons/plus.svg" => include_bytes!("../assets/icons/plus.svg").as_slice(),
            "icons/minus.svg" => include_bytes!("../assets/icons/minus.svg").as_slice(),
            "icons/pencil.svg" => include_bytes!("../assets/icons/pencil.svg").as_slice(),
            "icons/refresh-cw.svg" => include_bytes!("../assets/icons/refresh-cw.svg").as_slice(),
            "icons/arrow-right.svg" => include_bytes!("../assets/icons/arrow-right.svg").as_slice(),
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
                "refresh-cw.svg".into(),
                "arrow-right.svg".into(),
            ])
        } else {
            Ok(Vec::new())
        }
    }
}
