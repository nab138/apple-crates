use crate::backend::BackendError;
use thiserror::Error;

pub(crate) type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error("{0}")]
    Message(String),
}

impl AppError {
    pub(crate) fn user_message(&self) -> String {
        match self {
            Self::Backend(error) => error.user_message(),
            Self::Message(message) => message.clone(),
        }
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_flattens_backend_errors_for_ui() {
        let backend_error = AppError::from(BackendError::from("ADI provisioning failed"));
        assert_eq!(backend_error.user_message(), "ADI provisioning failed");

        let app_error = AppError::from("Select an IPA first");
        assert_eq!(app_error.user_message(), "Select an IPA first");
    }
}
