use std::process::Command;

fn run_export(format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_parket"))
        .args(["export", "sample1.parquet", "--format", format])
        .output()
        .expect("failed to run parket");
    assert!(output.status.success(), "export failed: {:?}", output.stderr);
    String::from_utf8(output.stdout).expect("non-utf8 output")
}

#[test]
fn export_json_matches_fixture() {
    let got = run_export("json");
    let expected = std::fs::read_to_string("tests/fixtures/sample1.json").unwrap();
    assert_eq!(got, expected);
}

#[test]
fn export_jsonl_matches_fixture() {
    let got = run_export("jsonl");
    let expected = std::fs::read_to_string("tests/fixtures/sample1.jsonl").unwrap();
    assert_eq!(got, expected);
}

#[test]
fn export_csv_matches_fixture() {
    let got = run_export("csv");
    let expected = std::fs::read_to_string("tests/fixtures/sample1.csv").unwrap();
    assert_eq!(got, expected);
}
