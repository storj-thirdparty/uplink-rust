// full_cycle_test test the following bucket operations:
//
// * Create a non-existing bucket.
// * Delete an empty bucket.
//
// Other integration tests may check some bucket operations that are also tested in this test file.

use uplink::access::Grant;
use uplink::{error, Error, Project};

mod common;

#[test]
fn integration_bucket_operations() {
    let env = common::Environment::load();
    let grant_root = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&grant_root);

    let bucket_name = common::generate_name("bucket-ops");

    // Check that the bucket returned by the first and second ensure are the same.
    let bucket_1 = project
        .ensure_bucket(&bucket_name)
        .expect("ensure bucket not to fail");
    let bucket_2 = project
        .ensure_bucket(&bucket_name)
        .expect("ensure bucket not to fail");
    let (_bucket_3, ok) = project
        .create_bucket(&bucket_name)
        .expect("create bucket not to fail");
    assert!(!ok, "create bucket that already exists");

    assert_eq!(bucket_1.name, bucket_2.name, "ensured bucket names");
    assert_eq!(
        bucket_1.created_at, bucket_2.created_at,
        "ensured bucket creation times"
    );

    // Stat the exiting bucket and check that matches.
    let bucket_3 = project
        .stat_bucket(&bucket_name)
        .expect("stat existing bucket not to fail");
    assert_eq!(bucket_1.name, bucket_3.name, "stat bucket name");
    assert_eq!(
        bucket_1.created_at, bucket_3.created_at,
        "ensured bucket creation times"
    );

    // Stat an non-existing bucket returns an error.
    let res = project.stat_bucket("does-not-exist");
    match res.expect_err("stat an non-existing bucket should return error") {
        Error::Uplink(error::Uplink::BucketNotFound(_)) => {}
        err => panic!(
            "{} is an unexpected error when stating an non-existing bucket",
            err
        ),
    }

    // List buckets.
    let mut it = project.list_buckets(None);
    let res = it
        .next()
        .expect("listing buckets has the crated bucket as the first element");
    let item = res.expect("list bucket item isn't an error");
    assert_eq!(bucket_1.name, item.name, "listed bucket name");
    assert_eq!(
        bucket_1.created_at, item.created_at,
        "listed bucket creation time"
    );

    assert!(
        it.next().is_none(),
        "listing buckets iterators return only 1 bucket"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with objects");
}
