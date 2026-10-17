use crate::sds::repository::Repository;
#[test]
fn pilot_migrates_and_reopens_without_changing_existing_rows() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("pilot.sqlite3");
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute_batch(include_str!("../migrations/001_pilot.sql"))
        .unwrap();
    c.execute(
        "INSERT INTO catalog_cache VALUES('p','a','q','{}','now')",
        [],
    )
    .unwrap();
    drop(c);
    let repo = Repository::open(root.path()).unwrap();
    drop(repo);
    let repo = Repository::open(root.path()).unwrap();
    drop(repo);
    let c = rusqlite::Connection::open(db).unwrap();
    assert_eq!(
        c.pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        7
    );
    assert_eq!(
        c.query_row::<String, _, _>("SELECT payload_json FROM catalog_cache", [], |r| r.get(0))
            .unwrap(),
        "{}"
    );
}
#[test]
fn future_schema_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let c = rusqlite::Connection::open(root.path().join("pilot.sqlite3")).unwrap();
    c.execute_batch("PRAGMA user_version=999").unwrap();
    drop(c);
    assert!(Repository::open(root.path()).is_err());
}
