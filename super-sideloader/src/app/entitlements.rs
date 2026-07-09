use crate::app::models::{AppEntitlement, AppOption, EntitlementValue, EntitlementsSource};

pub(crate) fn effective_entitlements_for_app(
    app: &AppOption,
    team_id: &str,
) -> Vec<AppEntitlement> {
    effective_entitlements(
        app.entitlements_source,
        &app.entitlements,
        app.entitlement_overrides.as_deref(),
        app.bundle_id(),
        team_id,
    )
}

pub(crate) fn default_effective_entitlements_for_app(
    app: &AppOption,
    team_id: &str,
) -> Vec<AppEntitlement> {
    default_effective_entitlements(
        app.entitlements_source,
        &app.entitlements,
        app.bundle_id(),
        team_id,
    )
}

pub(crate) fn effective_entitlements(
    entitlements_source: EntitlementsSource,
    embedded_entitlements: &[AppEntitlement],
    entitlement_overrides: Option<&[AppEntitlement]>,
    bundle_id: &str,
    team_id: &str,
) -> Vec<AppEntitlement> {
    if let Some(overrides) = entitlement_overrides {
        return overrides.to_vec();
    }

    default_effective_entitlements(
        entitlements_source,
        embedded_entitlements,
        bundle_id,
        team_id,
    )
}

pub(crate) fn default_effective_entitlements(
    entitlements_source: EntitlementsSource,
    embedded_entitlements: &[AppEntitlement],
    bundle_id: &str,
    team_id: &str,
) -> Vec<AppEntitlement> {
    match entitlements_source {
        EntitlementsSource::Embedded => embedded_entitlements
            .iter()
            .map(|entitlement| team_adjusted_entitlement(entitlement, bundle_id, team_id))
            .collect(),
        EntitlementsSource::GeneratedFallback => generated_default_entitlements(bundle_id, team_id),
    }
}

pub(crate) fn generated_default_entitlements(
    bundle_id: &str,
    team_id: &str,
) -> Vec<AppEntitlement> {
    vec![
        AppEntitlement {
            key: "application-identifier".into(),
            value: EntitlementValue::String(team_prefixed_bundle_identifier(bundle_id, team_id)),
        },
        AppEntitlement {
            key: "com.apple.developer.team-identifier".into(),
            value: EntitlementValue::String(team_id.to_string()),
        },
        AppEntitlement {
            key: "get-task-allow".into(),
            value: EntitlementValue::Boolean(true),
        },
        AppEntitlement {
            key: "keychain-access-groups".into(),
            value: EntitlementValue::Array(vec![EntitlementValue::String(
                team_prefixed_bundle_identifier(bundle_id, team_id),
            )]),
        },
    ]
}

fn team_adjusted_entitlement(
    entitlement: &AppEntitlement,
    bundle_id: &str,
    team_id: &str,
) -> AppEntitlement {
    let mut entitlement = entitlement.clone();
    match entitlement.key.as_str() {
        "application-identifier" => {
            entitlement.value = EntitlementValue::String(team_prefixed_identifier(
                &entitlement.value.edit_text(),
                bundle_id,
                team_id,
            ));
        }
        "com.apple.developer.team-identifier" => {
            entitlement.value = EntitlementValue::String(team_id.to_string());
        }
        "keychain-access-groups" => {
            entitlement.value =
                team_prefixed_keychain_groups(&entitlement.value, bundle_id, team_id);
        }
        _ => {}
    }
    entitlement
}

fn team_prefixed_keychain_groups(
    value: &EntitlementValue,
    bundle_id: &str,
    team_id: &str,
) -> EntitlementValue {
    match value {
        EntitlementValue::Array(values) => EntitlementValue::Array(
            values
                .iter()
                .map(|value| match value {
                    EntitlementValue::String(value) => EntitlementValue::String(
                        team_prefixed_identifier(value.as_str(), bundle_id, team_id),
                    ),
                    value => value.clone(),
                })
                .collect(),
        ),
        value => EntitlementValue::Array(
            value
                .edit_text()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    EntitlementValue::String(team_prefixed_identifier(value, bundle_id, team_id))
                })
                .collect(),
        ),
    }
}

fn team_prefixed_identifier(value: &str, bundle_id: &str, team_id: &str) -> String {
    let suffix = value
        .split_once('.')
        .map(|(_, suffix)| suffix)
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(bundle_id);
    team_prefixed_bundle_identifier(suffix, team_id)
}

fn team_prefixed_bundle_identifier(bundle_id: &str, team_id: &str) -> String {
    let team_id = team_id.trim();
    if team_id.is_empty() {
        bundle_id.to_string()
    } else {
        format!("{team_id}.{bundle_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{AppMetadata, PatchOption, SupportedDeviceFamily};

    #[test]
    fn generated_entitlements_use_selected_team_identifier() {
        let entitlements = generated_default_entitlements("com.example.app", "TEAM123");

        assert!(entitlements.iter().any(|entitlement| {
            entitlement.key == "application-identifier"
                && entitlement.value == EntitlementValue::String("TEAM123.com.example.app".into())
        }));
        assert!(entitlements.iter().any(|entitlement| {
            entitlement.key == "com.apple.developer.team-identifier"
                && entitlement.value == EntitlementValue::String("TEAM123".into())
        }));
    }

    #[test]
    fn generated_entitlements_do_not_contain_placeholder_team_id() {
        let text = generated_default_entitlements("com.example.app", "REALTEAM")
            .into_iter()
            .map(|entitlement| entitlement.value.display_text())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!text.contains("TEAMID"));
    }

    #[test]
    fn embedded_entitlements_are_adjusted_for_selected_team() {
        let entitlements = default_effective_entitlements(
            EntitlementsSource::Embedded,
            &[
                AppEntitlement {
                    key: "application-identifier".into(),
                    value: EntitlementValue::String("OLDTEAM.com.old.app".into()),
                },
                AppEntitlement {
                    key: "keychain-access-groups".into(),
                    value: EntitlementValue::Array(vec![EntitlementValue::String(
                        "OLDTEAM.com.old.app".into(),
                    )]),
                },
            ],
            "com.example.app",
            "TEAM123",
        );

        assert_eq!(
            entitlements[0].value,
            EntitlementValue::String("TEAM123.com.old.app".into())
        );
        assert_eq!(
            entitlements[1].value,
            EntitlementValue::Array(vec![EntitlementValue::String("TEAM123.com.old.app".into())])
        );
    }

    #[test]
    fn app_entitlement_adapter_matches_lower_level_helper() {
        let app = AppOption {
            metadata: AppMetadata::sample(
                "Example",
                "com.example.app",
                "1.0",
                "1",
                "Example",
                "16.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            path: "/tmp/Example.ipa".to_string(),
            icon_path: None,
            icon_override_path: None,
            entitlements: vec![AppEntitlement {
                key: "application-identifier".into(),
                value: EntitlementValue::String("OLDTEAM.com.example.app".into()),
            }],
            entitlements_source: EntitlementsSource::Embedded,
            entitlement_overrides: None,
            patches: Vec::<PatchOption>::new(),
        };

        assert_eq!(
            effective_entitlements_for_app(&app, "TEAM123"),
            effective_entitlements(
                app.entitlements_source,
                &app.entitlements,
                app.entitlement_overrides.as_deref(),
                app.bundle_id(),
                "TEAM123",
            )
        );
        assert_eq!(
            default_effective_entitlements_for_app(&app, "TEAM123"),
            default_effective_entitlements(
                app.entitlements_source,
                &app.entitlements,
                app.bundle_id(),
                "TEAM123",
            )
        );
    }
}
