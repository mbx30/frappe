use std::path::PathBuf;

fn test_pdf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/no-bleed.pdf")
}

#[test]
#[ignore = "requires PDF fixture corpus"]
fn bleed_round_trip_adds_bleed_and_passes_recheck() {
    // 1. Load a known-bad bleed fixture.
    // 2. Run check_bleed and assert failure.
    // 3. Run add_bleed with a target amount.
    // 4. Re-run check_bleed on output and assert pass.
    // 5. Assert input file was not overwritten.
}
