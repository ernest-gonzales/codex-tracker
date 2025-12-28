mod support;

use support::setup_db;

#[test]
fn set_active_home_returns_expected_home() {
    let mut test_db = setup_db();
    let db = &mut test_db.db;
    let home = db
        .add_home("/tmp/codex-secondary", Some("Secondary"))
        .expect("add home");
    db.set_active_home(home.id).expect("set active");

    let active = db.get_active_home().expect("active home").expect("home");
    assert_eq!(active.id, home.id);
    assert_eq!(active.path, "/tmp/codex-secondary");
    assert_eq!(active.label, "Secondary");
}
