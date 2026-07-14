use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const ROUND: &str = r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: []
"#;

#[test]
fn scores_round_from_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should follow the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ortalab-{unique}.yml"));
    fs::write(&path, ROUND).expect("temporary round should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_ortalab"))
        .arg(&path)
        .output()
        .expect("Ortalab should start");
    fs::remove_file(path).expect("temporary round should be removable");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"16\n");
}

#[test]
fn scores_round_from_standard_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ortalab"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Ortalab should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(ROUND.as_bytes())
        .expect("round should be written to stdin");

    let output = child
        .wait_with_output()
        .expect("Ortalab should finish successfully");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"16\n");
}
