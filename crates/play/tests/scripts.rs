//! Every conformance script in decomp/play/: the definition of done for the port.

use mh_play::{check, decomp_dir, load_data, SCRIPTS};

fn run(name: &str, expected: &str) {
    let decomp = decomp_dir();
    let data = load_data(&decomp);
    let script = SCRIPTS
        .iter()
        .find(|s| s.name == name && s.expected == expected)
        .unwrap();
    let result = check(&data, &decomp, script);
    assert!(
        result.passed(),
        "{name} ({expected}):\n{}",
        result.errors.join("")
    );
}

#[test]
fn walkthrough() {
    run("walkthrough", "walkthrough");
}

#[test]
fn errors() {
    run("errors", "errors");
}

#[test]
fn house() {
    run("house", "house");
}

#[test]
fn deaths() {
    run("deaths", "deaths");
}

#[test]
fn intro() {
    run("intro", "intro");
}

#[test]
fn bugfix_faithful() {
    run("bugfix", "bugfix.original");
}

#[test]
fn bugfix_fixed() {
    run("bugfix", "bugfix");
}
