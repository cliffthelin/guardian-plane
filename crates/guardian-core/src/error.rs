//! Stable public error categories and deterministic D-Bus identities.

const ERROR_PREFIX: &str = "org.guardianproject.Development.Guardian1.Error";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuardianErrorCategory {
    NotAuthorized,
    AuthenticationUnavailable,
    Unsupported,
    ProviderUnavailable,
    ProviderChanged,
    PreconditionFailed,
    Conflict,
    Busy,
    TimedOut,
    Cancelled,
    InvalidRequest,
    Unsafe,
    ApplyFailed,
    ObservationFailed,
    RollbackFailed,
    PersistenceFailed,
    Internal,
}

pub const ALL_ERROR_CATEGORIES: [GuardianErrorCategory; 17] = [
    GuardianErrorCategory::NotAuthorized,
    GuardianErrorCategory::AuthenticationUnavailable,
    GuardianErrorCategory::Unsupported,
    GuardianErrorCategory::ProviderUnavailable,
    GuardianErrorCategory::ProviderChanged,
    GuardianErrorCategory::PreconditionFailed,
    GuardianErrorCategory::Conflict,
    GuardianErrorCategory::Busy,
    GuardianErrorCategory::TimedOut,
    GuardianErrorCategory::Cancelled,
    GuardianErrorCategory::InvalidRequest,
    GuardianErrorCategory::Unsafe,
    GuardianErrorCategory::ApplyFailed,
    GuardianErrorCategory::ObservationFailed,
    GuardianErrorCategory::RollbackFailed,
    GuardianErrorCategory::PersistenceFailed,
    GuardianErrorCategory::Internal,
];

impl GuardianErrorCategory {
    #[must_use]
    pub const fn dbus_error_name(self) -> &'static str {
        match self {
            Self::NotAuthorized => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".NotAuthorized"
            ),
            Self::AuthenticationUnavailable => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".AuthenticationUnavailable"
            ),
            Self::Unsupported => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".Unsupported"
            ),
            Self::ProviderUnavailable => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".ProviderUnavailable"
            ),
            Self::ProviderChanged => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".ProviderChanged"
            ),
            Self::PreconditionFailed => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".PreconditionFailed"
            ),
            Self::Conflict => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".Conflict"
            ),
            Self::Busy => concat!("org.guardianproject.Development.Guardian1.Error", ".Busy"),
            Self::TimedOut => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".TimedOut"
            ),
            Self::Cancelled => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".Cancelled"
            ),
            Self::InvalidRequest => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".InvalidRequest"
            ),
            Self::Unsafe => concat!("org.guardianproject.Development.Guardian1.Error", ".Unsafe"),
            Self::ApplyFailed => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".ApplyFailed"
            ),
            Self::ObservationFailed => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".ObservationFailed"
            ),
            Self::RollbackFailed => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".RollbackFailed"
            ),
            Self::PersistenceFailed => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".PersistenceFailed"
            ),
            Self::Internal => concat!(
                "org.guardianproject.Development.Guardian1.Error",
                ".Internal"
            ),
        }
    }

    #[must_use]
    pub const fn error_prefix() -> &'static str {
        ERROR_PREFIX
    }
}
