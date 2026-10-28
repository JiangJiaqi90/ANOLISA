//! Retrieval must report a row it cannot decode instead of dropping it.
//!
//! `query_map(...).flatten()` used to swallow row-level errors, so a match
//! that failed to decode simply disappeared and the caller saw a successful
//! empty or partial result.

use agent_memory::error::MemoryError;
use agent_memory::index::BM25Store;
use rusqlite::Connection;

#[test]
fn keyword_search_surfaces_row_decode_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("probe.db");
    let mut store = BM25Store::open(&path, 0.01, 0.3, true).unwrap();
    store
        .upsert("notes/known.md", 100, 10, "deploymentmarker", None)
        .unwrap();
    assert_eq!(store.search("deploymentmarker", 5, true).unwrap().len(), 1);

    // Fault injection in a private test index: `mtime_ms` is selected as an
    // i64, so storing text there makes the row decoder fail. This is not a
    // normal write path.
    let conn = Connection::open(&path).unwrap();
    conn.execute("UPDATE files SET mtime_ms = 'invalid'", [])
        .unwrap();

    let err = store.search("deploymentmarker", 5, true).unwrap_err();
    assert!(
        matches!(err, MemoryError::Sqlite(_)),
        "a row decode error must not look like an empty result: {err:?}"
    );
}

#[test]
fn vector_search_surfaces_row_decode_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("probe.db");
    let mut store = BM25Store::open(&path, 0.01, 0.3, true).unwrap();
    store
        .upsert("notes/known.md", 100, 10, "deploymentmarker", None)
        .unwrap();
    store.upsert_vec("notes/known.md", &[1.0, 0.0]).unwrap();
    assert_eq!(store.search_vec(&[1.0, 0.0], 5).unwrap().len(), 1);

    let conn = Connection::open(&path).unwrap();
    conn.execute("UPDATE files SET mtime_ms = 'invalid'", [])
        .unwrap();

    let err = store.search_vec(&[1.0, 0.0], 5).unwrap_err();
    assert!(
        matches!(err, MemoryError::Sqlite(_)),
        "a row decode error must not look like an empty vector result: {err:?}"
    );
}

#[test]
fn empty_result_stays_a_successful_empty_search() {
    // The error path above must not turn a genuine zero-match into an error.
    let tmp = tempfile::tempdir().unwrap();
    let mut store = BM25Store::open(&tmp.path().join("probe.db"), 0.01, 0.3, true).unwrap();
    store
        .upsert("notes/known.md", 100, 10, "deploymentmarker", None)
        .unwrap();

    assert!(store.search("absentterm", 5, true).unwrap().is_empty());
}
