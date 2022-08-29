use std::io::{Read, Write};
use std::time::Duration;

use uplink::access::Grant;
use uplink::Project;

mod common;

#[test]
fn integration_create_upload_list_download_delete() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("full-cycle");
    let (created_bucket, ok) = project.create_bucket(&bucket_name).expect("create bucket");
    assert!(ok, "bucket shouldn't exist",);
    assert_eq!(bucket_name, created_bucket.name);
    common::assert_epoch_timestamp_from_now(
        created_bucket.created_at,
        Duration::from_secs(3),
        "bucket created at",
    );

    // Check that the new created bucket exists.
    assert!(
        project
            .list_buckets(None)
            .find(|res| {
                match res {
                    Ok(b) => b.name == bucket_name,
                    _ => panic!("not expected result with error when listing buckets"),
                }
            })
            .is_some(),
        "list buckets haven't listed the newly create bucket"
    );

    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");

    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // Check that new uploaded object exists.
    assert!(
        project
            .list_objects(&bucket_name, None)
            .expect("list objects")
            .find(|res| {
                match res {
                    Ok(o) => o.key == object_key,
                    _ => panic!("not expected result with error when listing objects"),
                }
            })
            .is_some(),
        "list objects haven't found the newly uploaded object",
    );

    let download = &mut project
        .download_object(&bucket_name, object_key, None)
        .expect("download object");

    let downloaded_object = download.info().expect("download object info");
    assert_eq!(object_key, downloaded_object.key, "downloaded object key");
    assert!(!downloaded_object.is_prefix, "downloaded object is_prefix");
    assert_eq!(
        upload.info().expect("upload info").metadata_system.created,
        downloaded_object.metadata_system.created,
        "deleted object created at",
    );
    common::assert_epoch_timestamp_from_now(
        downloaded_object.metadata_system.created,
        Duration::from_secs(3),
        "downloaded object created at",
    );

    let mut downloaded_object_data = String::new();
    download
        .read_to_string(&mut downloaded_object_data)
        .expect("download object read");
    assert_eq!(object_data, downloaded_object_data, "object data");

    let deleted_object = project
        .delete_object(&bucket_name, object_key)
        .expect("delete object");
    assert_eq!(object_key, deleted_object.key, "deleted object key");
    assert!(!deleted_object.is_prefix, "deleted object is_prefix");
    assert_eq!(
        downloaded_object.metadata_system.created, deleted_object.metadata_system.created,
        "deleted object created at",
    );

    let deleted_bucket = project.delete_bucket(&bucket_name).expect("delete bucket");
    assert_eq!(
        created_bucket.name, deleted_bucket.name,
        "deleted bucket name"
    );
    assert_eq!(
        created_bucket.created_at, deleted_bucket.created_at,
        "deleted bucket created at"
    );
}
