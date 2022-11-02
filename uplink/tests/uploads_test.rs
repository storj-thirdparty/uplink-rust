use uplink::access::Grant;
use uplink::error;
use uplink::project::options;
use uplink::{metadata, Error, Project};

use std::io::{Read, Write};
use std::time::Duration;
use std::vec::Vec;

use rand::{self, RngCore};

mod common;

#[test]
fn integration_upload_commit_and_abort() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("upload");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Create an upload and check that it appears in the list of uploads.
    let object_1_key = "test-data-1.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_1_key, None)
        .expect("upload object");
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // Aborting a committed upload.
    upload.abort().expect_err("abort a committed upload");

    // Abort an uncommitted upload.
    let object_2_key = "test-data-2.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_2_key, None)
        .expect("upload object");
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.abort().expect("abort an uncommitted upload");

    // Commit an upload without any written data.
    let object_3_key = "test-data-3.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_3_key, None)
        .expect("upload object");
    upload.commit().expect("upload object commit");

    // List objects should only show the one with a committed upload
    let it = &mut project
        .list_objects(&bucket_name, None)
        .expect("list objects");

    let mut count = 0;
    let mut items_found: u8 = 0;

    for item in it {
        let info = item.expect("object from list objects");
        count += 1;

        if info.key == object_1_key {
            items_found |= 1;
            continue;
        }

        if info.key == object_3_key {
            items_found |= 2;
            continue;
        }

        panic!("list an unexpected object: {}", info.key);
    }

    assert_eq!(2, count, "number of listed objects");
    assert_eq!(3, items_found, "objects found in the listing");

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up delete bucket with objects");
}

#[test]
fn integration_upload_multipart_commit() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("upload");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Multipart upload empty object.
    let object_empty_key = "test-empty.txt";
    let upload_info = project
        .begin_upload(&bucket_name, object_empty_key, None)
        .expect("begin upload empty object");
    let object = project
        .commit_upload(&bucket_name, object_empty_key, &upload_info.upload_id, None)
        .expect("commit upload empty object");
    assert_eq!(object_empty_key, object.key, "object key");
    assert!(!object.is_prefix, "object is prefix");
    assert_eq!(
        0, object.metadata_system.content_length,
        "object content length"
    );

    // Multipart upload with multiple parts.
    let object_multipart_key = "test-multipart.txt";
    let upload_info = project
        .begin_upload(&bucket_name, object_multipart_key, None)
        .expect("begin upload empty object");

    // List uploads to see that this multipart upload is pending.
    let mut it = project
        .list_uploads(&bucket_name, None)
        .expect("list uploads");
    let item = it
        .next()
        .expect("an item in the uploads list")
        .expect("a pending upload");
    // TODO: uncomment the following assertion when the bug is fixed:
    // https://github.com/storj/storj/issues/5298
    // assert_eq!(upload_info.upload_id, item.upload_id, "pending upload key");
    assert_eq!(object_multipart_key, item.key, "pending upload key");
    assert!(!item.is_prefix, "pending upload is prefix");
    assert!(it.next().is_none(), "only one pending upload in the list");

    // Uploading 2 parts in reverse order using `data`.
    // A part must be at least of 5 MiB.
    let mut data = vec![0u8; 10 * 1024 * 1024];
    rand::thread_rng().fill_bytes(&mut data);
    let mut part = project
        .upload_part(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            1,
        )
        .expect("upload part 1");
    part.write_all(&data[data.len() / 2..])
        .expect("write data mutipart 1");
    part.commit().expect("commit multipart 1");

    // List parts of the multipart pending upload.
    let mut it = project
        .list_upload_parts(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            None,
        )
        .expect("list upload parts");
    let item = it
        .next()
        .expect("an item in the upload parts list")
        .expect("a part in the pending upload");
    assert_eq!(1, item.part_number, "pending upload part number");
    assert_eq!(
        data[data.len() / 2..].len(),
        item.size,
        "pending upload part size"
    );
    assert!(
        it.next().is_none(),
        "only one part in the list of parts of the pending upload"
    );

    let mut part = project
        .upload_part(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            0,
        )
        .expect("upload part 0");
    part.write_all(&data[..data.len() / 2])
        .expect("write data mutipart 0");

    let etag = "this-is-its-etag";
    part.set_etag(etag.as_bytes())
        .expect("non error setting a valid etag to an upload part");
    part.commit().expect("commit multipart 0");

    // List parts of the multipart pending upload.
    let it = project
        .list_upload_parts(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            None,
        )
        .expect("list upload parts");
    for item in it {
        let part = item.expect("a part in the pending upload");

        if part.part_number == 0 {
            assert_eq!(
                data[..data.len() / 2].len(),
                part.size,
                "pending upload part 0 size"
            );

            assert_eq!(etag.as_bytes(), part.etag.as_slice(), "etag part");
            continue;
        }
        if part.part_number == 1 {
            assert_eq!(
                data[data.len() / 2..].len(),
                part.size,
                "pending upload part 1 size"
            );
            continue;
        }

        panic!(
            "unexpected part when listing part of a pending upload: {}",
            part.part_number
        );
    }

    // Commit the upload.
    let object_committed = project
        .commit_upload(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            None,
        )
        .expect("commit a multipart upload");

    // Download the committed multipart uploaded object to verify it.
    let download = &mut project
        .download_object(&bucket_name, object_multipart_key, None)
        .expect("download object");

    let downloaded_object = download.info().expect("download object info");
    assert_eq!(
        object_committed.key, downloaded_object.key,
        "downloaded object key"
    );
    assert_eq!(
        object_committed.is_prefix, downloaded_object.is_prefix,
        "downloaded object is_prefix"
    );
    assert!(
        downloaded_object.metadata_system.created != Duration::ZERO,
        "downloaded object created at cannot be 0",
    );
    assert_eq!(
        data.len(),
        downloaded_object.metadata_system.content_length as usize,
        "uploaded object content length"
    );

    let mut downloaded_data = Vec::with_capacity(data.len());
    download
        .read_to_end(&mut downloaded_data)
        .expect("download object read");

    assert_eq!(data.len(), downloaded_data.len(), "downloaded object data",);
    for (i, v) in downloaded_data.iter().enumerate() {
        assert_eq!(data[i], *v, "downloaded object data at position: {}", i);
    }

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up delete bucket with objects");
}

#[test]
fn integration_upload_multipart_abort_and_list_parts_cursor() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("upload");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Multipart upload with multiple parts.
    let object_multipart_key = "test-multipart.txt";
    let upload_info = project
        .begin_upload(&bucket_name, object_multipart_key, None)
        .expect("begin upload empty object");

    // Uploading 2 parts.
    // A part must be at least of 5 MiB.
    let mut data = vec![0u8; 10 * 1024 * 1024];
    rand::thread_rng().fill_bytes(&mut data);

    let mut part = project
        .upload_part(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            0,
        )
        .expect("upload part 0");
    part.write_all(&data[..data.len() / 2])
        .expect("write data mutipart 0");

    let etag = "this-is-its-etag";
    part.set_etag(etag.as_bytes())
        .expect("non error setting a valid etag to an upload part");
    part.commit().expect("commit multipart 0");

    let mut part = project
        .upload_part(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            1,
        )
        .expect("upload part 1");
    part.write_all(&data[data.len() / 2..])
        .expect("write data mutipart 1");
    part.commit().expect("commit multipart 1");

    // List parts of the multipart pending upload using cursor, so only one of the parts should be
    // listed.
    let mut it = project
        .list_upload_parts(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            Some(&options::ListUploadParts {
                cursor: 0,
                ..Default::default()
            }),
        )
        .expect("list upload parts");

    let part = it
        .next()
        .expect("an item")
        .expect("part in the pending upload");
    assert_eq!(1, part.part_number, "part number");
    assert!(it.next().is_none(), "no more items in iterator");

    // Abort the upload.
    project
        .abort_upload(&bucket_name, object_multipart_key, &upload_info.upload_id)
        .expect("abort a multipart upload");

    // Verify that the parts aren't in the list after abort.
    let mut it = project
        .list_upload_parts(
            &bucket_name,
            object_multipart_key,
            &upload_info.upload_id,
            Some(&options::ListUploadParts {
                cursor: 0,
                ..Default::default()
            }),
        )
        .expect("list upload parts");
    assert!(it.next().is_none(), "no parts in the part iterator");

    // Verify that there isn't uploads in the list of uploads after abort.
    let mut it = project
        .list_uploads(&bucket_name, None)
        .expect("list uploads");
    assert!(it.next().is_none(), "no uploads in the uploads iterator");

    // Verify that the object doesn't exists after aborting.
    let err = project
        .stat_object(&bucket_name, object_multipart_key)
        .expect_err("object not found");
    match err {
        Error::Uplink(error::Uplink::ObjectNotFound(_)) => {}
        _ => panic!("expected object not found error, found: {}", err),
    }

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up delete bucket with objects");
}

#[test]
fn integration_upload_multipart_commit_custom_metadata() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("upload");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    let object_key = "test-object.txt";
    let upload_info = project
        .begin_upload(&bucket_name, object_key, None)
        .expect("begin upload empty object");

    let custom_metadata_key = "uplink-rust:field";
    let custom_metadata_value = "value";
    let mut custom_metadata = metadata::Custom::with_capacity(1);
    custom_metadata.insert(
        String::from(custom_metadata_key),
        String::from(custom_metadata_value),
    );

    let object = project
        .commit_upload(
            &bucket_name,
            object_key,
            &upload_info.upload_id,
            Some(&mut options::CommitUpload::new(&mut custom_metadata)),
        )
        .expect("commit upload empty object");

    assert_eq!(object_key, object.key, "object key");
    assert!(!object.is_prefix, "object is prefix");

    // Stat the object to reverify the committed multipart object metadata.
    let object = project
        .stat_object(&bucket_name, object_key)
        .expect("stat object");
    assert_eq!(object_key, object.key, "object key");
    assert!(!object.is_prefix, "object is prefix");
    assert_eq!(
        1,
        object.metadata_custom.count(),
        "custom metadata number of items",
    );
    assert_eq!(
        custom_metadata_value,
        object
            .metadata_custom
            .get(custom_metadata_key)
            .expect("custom metadata key"),
        "custom metadata value"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up delete bucket with objects");
}
