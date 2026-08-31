use guardian_core::error::{ALL_ERROR_CATEGORIES, GuardianErrorCategory};
use std::collections::HashSet;

#[test]
fn p0_dbus_004_every_error_has_a_unique_stable_dbus_identity() {
    assert_eq!(ALL_ERROR_CATEGORIES.len(), 17);
    let identities: HashSet<_> = ALL_ERROR_CATEGORIES
        .iter()
        .map(|category| category.dbus_error_name())
        .collect();
    assert_eq!(identities.len(), ALL_ERROR_CATEGORIES.len());
    assert!(
        identities
            .iter()
            .all(|name| name.starts_with("org.guardianproject.Development.Guardian1.Error."))
    );
    assert_eq!(
        GuardianErrorCategory::ProviderChanged.dbus_error_name(),
        "org.guardianproject.Development.Guardian1.Error.ProviderChanged"
    );
}
