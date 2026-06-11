use radroots_field_core::RadrootsAppError;
use radroots_field_core::logging;

#[test]
fn init_logging_stdout_maps_global_subscriber_error() {
    let _ = tracing_subscriber::fmt().try_init();
    let err = logging::init_logging_stdout();
    assert!(matches!(err, Err(RadrootsAppError::Msg(_))));
}
