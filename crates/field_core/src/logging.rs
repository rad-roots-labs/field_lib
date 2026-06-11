use std::path::PathBuf;

#[cfg_attr(not(coverage_nightly), uniffi::export)]
pub fn init_logging(
    dir: Option<String>,
    file_name: Option<String>,
    is_stdout: Option<bool>,
) -> Result<(), crate::RadrootsAppError> {
    let opts = radroots_log::LoggingOptions {
        dir: dir.map(PathBuf::from),
        file_name: file_name.unwrap_or_else(|| "radroots.log".to_string()),
        stdout: is_stdout.unwrap_or(true),
        default_level: None,
    };
    match radroots_log::init_logging(opts) {
        Ok(()) => Ok(()),
        Err(err) => Err(crate::RadrootsAppError::Msg(format!("{err}"))),
    }
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
pub fn init_logging_stdout() -> Result<(), crate::RadrootsAppError> {
    match radroots_log::init_stdout() {
        Ok(()) => Ok(()),
        Err(err) => Err(crate::RadrootsAppError::Msg(format!("{err}"))),
    }
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
pub fn log_info(msg: String) -> Result<(), crate::RadrootsAppError> {
    radroots_log::log_info(msg);
    Ok(())
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
pub fn log_error(msg: String) -> Result<(), crate::RadrootsAppError> {
    radroots_log::log_error(msg);
    Ok(())
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
pub fn log_debug(msg: String) -> Result<(), crate::RadrootsAppError> {
    radroots_log::log_debug(msg);
    Ok(())
}
