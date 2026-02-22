fn main() {
    run_bindgen();
}

#[cfg(not(coverage_nightly))]
fn run_bindgen() {
    uniffi::uniffi_bindgen_main()
}

#[cfg(coverage_nightly)]
fn run_bindgen() {}

#[cfg(all(test, coverage_nightly))]
mod tests {
    #[test]
    fn main_is_callable_in_coverage_builds() {
        super::main();
    }
}
