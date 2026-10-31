use objc2_service_management::{SMAppService, SMAppServiceStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    Disabled,
    Enabled,
    RequiresApproval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToggleAction {
    Register,
    Unregister,
    OpenSettings,
}

const fn toggle_action(status: Status) -> ToggleAction {
    match status {
        Status::Disabled => ToggleAction::Register,
        Status::Enabled => ToggleAction::Unregister,
        Status::RequiresApproval => ToggleAction::OpenSettings,
    }
}

pub(crate) fn status() -> Result<Status, String> {
    // SAFETY: mainAppService and status do not retain caller-owned pointers.
    let status = unsafe { SMAppService::mainAppService().status() };
    match status {
        SMAppServiceStatus::NotRegistered | SMAppServiceStatus::NotFound => Ok(Status::Disabled),
        SMAppServiceStatus::Enabled => Ok(Status::Enabled),
        SMAppServiceStatus::RequiresApproval => Ok(Status::RequiresApproval),
        _ => Err("macOS returned an unknown login item status".into()),
    }
}

pub(crate) fn toggle() -> Result<Status, String> {
    let current = status()?;
    // SAFETY: SMAppService owns NSError values produced by its synchronous APIs.
    let service = unsafe { SMAppService::mainAppService() };
    match toggle_action(current) {
        ToggleAction::Register => {
            // SAFETY: Synchronous registration call on `mainAppService` with autoreleased NSError.
            unsafe { service.registerAndReturnError() }.map_err(|error| error.to_string())?
        }
        ToggleAction::Unregister => {
            // SAFETY: Synchronous unregistration call on `mainAppService` with autoreleased NSError.
            unsafe { service.unregisterAndReturnError() }.map_err(|error| error.to_string())?
        }
        ToggleAction::OpenSettings => {
            open_settings();
            return Ok(current);
        }
    }
    status()
}

fn open_settings() {
    // SAFETY: This asks macOS to open the Login Items settings pane.
    unsafe { SMAppService::openSystemSettingsLoginItems() };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_registers() {
        assert_eq!(toggle_action(Status::Disabled), ToggleAction::Register);
    }

    #[test]
    fn enabled_unregisters() {
        assert_eq!(toggle_action(Status::Enabled), ToggleAction::Unregister);
    }

    #[test]
    fn approval_opens_settings() {
        assert_eq!(
            toggle_action(Status::RequiresApproval),
            ToggleAction::OpenSettings
        );
    }
}
