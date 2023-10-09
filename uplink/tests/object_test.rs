use uplink::access::Grant;
use uplink::project::options;
use uplink::{metadata, Project};

use std::io::Write;
use std::time::Duration;

mod common;

#[test]
fn integration_object_stat() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-stat");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Create an upload.
    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");

    // Upload object's data.
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");

    // Set custom metdata to the uploading object.
    let metadata_custom_key = "uplink-rust:field";
    let metadata_custom_value = "value";
    let mut custom_metadata = metadata::Custom::with_capacity(1);
    custom_metadata.insert(
        String::from(metadata_custom_key),
        String::from(metadata_custom_value),
    );
    upload
        .set_custom_metadata(&mut custom_metadata)
        .expect("setting custom metatada to the upload object");

    upload.commit().expect("upload object commit");

    // Check stat_object.
    let object_info = upload.info().expect("upload object info not to fail");
    let object_info_stat = project
        .stat_object(&bucket_name, &object_key)
        .expect("stat an existing object not to fail");

    assert_eq!(
        object_info.key, object_info_stat.key,
        "uploaded object & stat object key"
    );
    assert_eq!(
        object_info.is_prefix, object_info_stat.is_prefix,
        "uploaded object & stat object is prefix"
    );
    assert_eq!(
        object_info.metadata_system.created, object_info_stat.metadata_system.created,
        "uploaded object & stat object system metadata created time"
    );
    assert_eq!(
        object_info.metadata_system.expires, object_info_stat.metadata_system.expires,
        "uploaded object & stat object system metadata expiration time"
    );
    assert_eq!(
        object_info.metadata_system.content_length, object_info_stat.metadata_system.content_length,
        "uploaded object & stat object system metadata content length"
    );
    assert_eq!(
        1,
        object_info.metadata_custom.count(),
        "uploaded object has one custom metadata entry"
    );
    assert_eq!(
        1,
        object_info_stat.metadata_custom.count(),
        "stat object has one custom metadata entry"
    );
    assert_eq!(
        Some(&String::from(metadata_custom_value)),
        object_info.metadata_custom.get(metadata_custom_key),
        "uploaded object custom metadata entry value"
    );
    assert_eq!(
        object_info.metadata_custom.get(metadata_custom_key),
        object_info_stat.metadata_custom.get(metadata_custom_key),
        "uploaded object & stat object custom metadata entry value comparison"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_listing_metadata() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-listing-metadata");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Create an upload.
    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");

    // Upload object's data.
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");

    // Set custom metdata to the uploading object.
    let metadata_custom_key = "uplink-rust:field";
    let metadata_custom_value = "value";
    let mut custom_metadata = metadata::Custom::with_capacity(1);
    custom_metadata.insert(
        String::from(metadata_custom_key),
        String::from(metadata_custom_value),
    );
    upload
        .set_custom_metadata(&mut custom_metadata)
        .expect("setting custom metatada to the upload object");
    upload.commit().expect("upload object commit");

    // List objects without metadata.
    let mut it = project
        .list_objects(&bucket_name, None)
        .expect("list objects without options");
    let res = it
        .next()
        .expect("list object iterator to return the first and only object in the list");
    let object_info = res.expect("first and only listed object is an object not an error");
    assert_eq!(object_key, object_info.key, "listed object key");
    assert!(!object_info.is_prefix, "listed object is not a prefix");
    assert_eq!(
        Duration::ZERO,
        object_info.metadata_system.created,
        "listed object system metadata created"
    );
    assert_eq!(
        None, object_info.metadata_system.expires,
        "listed object system metadata expires"
    );
    assert_eq!(
        0, object_info.metadata_system.content_length,
        "listed object system metadata content lenght"
    );
    assert_eq!(
        0,
        object_info.metadata_custom.count(),
        "listed object custom metadata number of items"
    );
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // List objects with metadata.
    let mut opts_list: options::ListObjects = Default::default();
    opts_list.system = true;
    opts_list.custom = true;
    let mut it = project
        .list_objects(&bucket_name, Some(&opts_list))
        .expect("list objects without options");
    let res = it
        .next()
        .expect("list object iterator to return the first and only object in the list");
    let object_info = res.expect("first and only listed object is an object not an error");
    assert_eq!(object_key, object_info.key, "listed object key");
    assert!(!object_info.is_prefix, "listed object is not a prefix");
    assert!(
        object_info.metadata_system.created != Duration::ZERO,
        "listed object system metadata created isn't 0"
    );
    assert_eq!(
        None, object_info.metadata_system.expires,
        "listed object system metadata expires is None"
    );
    assert!(
        object_info.metadata_system.content_length != 0,
        "listed object system metadata content lenght isn't 0"
    );
    assert_eq!(
        1,
        object_info.metadata_custom.count(),
        "listed object custom metadata number of items"
    );
    assert_eq!(
        Some(&String::from(metadata_custom_value)),
        object_info.metadata_custom.get(metadata_custom_key),
        "uploaded object custom metadata entry value"
    );
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_listing_prefix() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-listing-prefix");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Upload the first object.
    let object1_key = "test-data-1.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object1_key, None)
        .expect("upload object");
    upload
        .write_all(String::from("Uplink Rust test object").as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // Upload the second object with a prefix.
    let object2_prefix = "folder/";
    let object2_name = "test-data-2.txt";
    let object2_key = &format!("{}{}", object2_prefix, object2_name);
    let upload = &mut project
        .upload_object(&bucket_name, object2_key, None)
        .expect("upload object");
    upload
        .write_all(String::from("Uplink Rust test object").as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // List objects without prefix.
    let mut it = project
        .list_objects(&bucket_name, None)
        .expect("list objects without options");

    let mut visited_keys: u8 = 0;
    for _i in 1..=2 {
        let res = it
            .next()
            .expect("list object iterator to return an object in the list");
        let object_info = res.expect("an object not an error");

        if &object_info.key == object1_key {
            visited_keys |= 1;
            continue;
        }

        if &object_info.key == object2_prefix {
            assert!(object_info.is_prefix, "listed object is prefix");
            visited_keys |= 2;
            continue;
        }

        panic!(
            "listed an object with an unexpected key: {}",
            object_info.key
        );
    }

    assert_eq!(3, visited_keys, "listing objects missed some object");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a fourth item"
    );

    // List objects with prefix.
    let mut it = project
        .list_objects(
            &bucket_name,
            Some(
                &options::ListObjects::with_prefix(object2_prefix)
                    .expect("list objects with a correct prefix"),
            ),
        )
        .expect("list objects without options");

    let res = it
        .next()
        .expect("list object iterator to return an object in the list");
    let object_info = res.expect("an object not an error");
    assert_eq!(object2_key, &object_info.key, "object key");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a fourth item"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_listing_recursive() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-listing-recursive");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Upload the first object.
    let object1_key = "test-data-1.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object1_key, None)
        .expect("upload object");
    upload
        .write_all(String::from("Uplink Rust test object").as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // Upload the second object with a prefix.
    let object2_prefix = "folder/";
    let object2_key = &format!("{}test-data-2.txt", object2_prefix);
    let upload = &mut project
        .upload_object(&bucket_name, object2_key, None)
        .expect("upload object");
    upload
        .write_all(String::from("Uplink Rust test object").as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    // List objects without prefix.
    let mut opts_list = options::ListObjects::default();
    opts_list.recursive = true;
    let mut it = project
        .list_objects(&bucket_name, Some(&opts_list))
        .expect("list objects without options");

    let mut visited_keys: u8 = 0;
    for _i in 1..=2 {
        let res = it
            .next()
            .expect("list object iterator to return an object in the list");
        let object_info = res.expect("an object not an error");

        if &object_info.key == object1_key {
            visited_keys |= 1;
            continue;
        }

        if &object_info.key == object2_key {
            visited_keys |= 2;
            continue;
        }

        panic!(
            "listed an object with an unexpected key: {}",
            object_info.key
        );
    }

    assert_eq!(3, visited_keys, "listing objects missed some object");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a fourth item"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_copy() {
    use std::thread;

    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-copy");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Upload an object.
    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("commit upload object");
    let object_original = upload.info().expect("upload object info");

    // Wait a second to ensure that created time of the copies are different because its
    // resolution is in seconds.
    thread::sleep(Duration::from_secs(1));

    // Copy object without options.
    let object_copy_noops_key = "test-data-copy-no-options.txt";
    let object_copied_noops = project
        .copy_object(
            &bucket_name,
            object_key,
            &bucket_name,
            object_copy_noops_key,
            None,
        )
        .expect("copy object");
    assert_eq!(
        object_copy_noops_key, object_copied_noops.key,
        "copied object key"
    );
    assert!(!object_copied_noops.is_prefix, "copied object is prefix");

    // List objects to see if the copies exist.
    let mut list_ops = options::ListObjects::default();
    list_ops.system = true;
    let mut it = project
        .list_objects(&bucket_name, Some(&list_ops))
        .expect("list objects");
    while let Some(robj) = it.next() {
        let obj = robj.expect("no error");
        if obj.key == object_key {
            assert_eq!(
                obj.metadata_system.created, object_original.metadata_system.created,
                "created time object copy no options"
            );
            continue;
        }

        if obj.key == object_copy_noops_key {
            assert_eq!(
                obj.metadata_system.created, object_copied_noops.metadata_system.created,
                "created time object copy no options"
            );
            assert!(
                obj.metadata_system.created != object_original.metadata_system.created,
                "created time object copy no options not equal to the original object"
            );
            continue;
        }

        panic!("listing objects get an unexpected object key: {}", obj.key);
    }

    // Copy object with options and do it in anohter bucket.
    let bucket_name_copy = common::generate_name("object-copy");
    let (_bucket, _ok) = project
        .create_bucket(&bucket_name_copy)
        .expect("create bucket");
    let object_copy_ops_key = "test-data-copy-options.txt";
    let object_copied_ops = project
        .copy_object(
            &bucket_name,
            object_key,
            &bucket_name_copy,
            object_copy_ops_key,
            None,
        )
        .expect("copy object");
    assert_eq!(
        object_copy_ops_key, object_copied_ops.key,
        "copied object key"
    );
    assert!(!object_copied_ops.is_prefix, "copied object is prefix");

    let mut it = project
        .list_objects(&bucket_name_copy, Some(&list_ops))
        .expect("list objects");
    let obj = it
        .next()
        .expect("list the only object")
        .expect("list object");
    assert_eq!(object_copy_ops_key, obj.key, "object key copy options");
    assert_eq!(
        obj.metadata_system.created, object_copied_ops.metadata_system.created,
        "created time object copy options"
    );
    assert!(
        obj.metadata_system.created != object_original.metadata_system.created,
        "created time object copy no options not equal to the original object"
    );
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
    project
        .delete_bucket_with_objects(&bucket_name_copy)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_move() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-move");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Upload an object.
    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");
    let object_data = String::from("Uplink Rust test object");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("commit upload object");
    upload.info().expect("upload object info");

    // Move object to a different key without options.
    let object_move_key = "test-data-moved.txt";
    project
        .move_object(
            &bucket_name,
            object_key,
            &bucket_name,
            object_move_key,
            None,
        )
        .expect("move object without options");

    // List objects to check that it's moved.
    let mut it = project
        .list_objects(&bucket_name, None)
        .expect("list objects");
    let obj = it
        .next()
        .expect("list the only object")
        .expect("list object");
    assert_eq!(object_move_key, obj.key, "object key move no options");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // Move object to a different bucket with options.
    let bucket_move_name = common::generate_name("object-move");
    let (_bucket, _ok) = project
        .create_bucket(&bucket_move_name)
        .expect("create bucket");
    project
        .move_object(
            &bucket_name,
            object_move_key,
            &bucket_move_name,
            object_key,
            Some(&options::MoveObject::default()),
        )
        .expect("move object with options");

    // List objects to check that it's moved.
    let mut it = project
        .list_objects(&bucket_move_name, None)
        .expect("list objects");
    let obj = it
        .next()
        .expect("list the only object")
        .expect("list object");
    assert_eq!(object_key, obj.key, "object key move options");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // Check that original bucket is empty after the object is moved to another bucket.
    let mut it = project
        .list_objects(&bucket_name, None)
        .expect("list objects");
    assert!(
        it.next().is_none(),
        "listing objects iterator doesn't have a second item"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
    project
        .delete_bucket_with_objects(&bucket_move_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}

#[test]
fn integration_object_update_metadata() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&access_grant);

    let bucket_name = common::generate_name("object-listing-metadata");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Create an upload.
    let object_key = "test-data.txt";
    let upload = &mut project
        .upload_object(&bucket_name, object_key, None)
        .expect("upload object");

    // Set custom metdata to the uploading object.
    let metadata_custom_key = "uplink-rust:field";
    let metadata_custom_value = "value";
    let mut custom_metadata = metadata::Custom::with_capacity(1);
    custom_metadata.insert(
        String::from(metadata_custom_key),
        String::from(metadata_custom_value),
    );
    upload
        .set_custom_metadata(&mut custom_metadata)
        .expect("setting custom metatada to the upload object");
    upload.commit().expect("upload object commit");

    // Stat object to check the initial metadata.
    let object = project
        .stat_object(&bucket_name, object_key)
        .expect("stat object");
    assert_eq!(1, object.metadata_custom.count(), "custom metadata entries");
    assert_eq!(
        metadata_custom_value,
        object
            .metadata_custom
            .get(metadata_custom_key)
            .expect("metadata value"),
        "initial metadata value"
    );

    // Update object's metadata.
    let metadata_custom_value_override = "value-overridden";
    assert!(
        custom_metadata.insert(
            String::from(metadata_custom_key),
            String::from(metadata_custom_value_override)
        ),
        "insert metadata with an existing key",
    );
    let metadata_custom_key_new = "uplink-rust:field-2";
    let metadata_custom_value_new = "value-2";
    custom_metadata.insert(
        String::from(metadata_custom_key_new),
        String::from(metadata_custom_value_new),
    );
    project
        .update_object_metadata(&bucket_name, object_key, &mut custom_metadata, None)
        .expect("update object metadata");

    // Stat object to check that its metadata is updated.
    let object = project
        .stat_object(&bucket_name, object_key)
        .expect("stat object");
    assert_eq!(2, object.metadata_custom.count(), "custom metadata entries");
    assert_eq!(
        metadata_custom_value_override,
        object
            .metadata_custom
            .get(metadata_custom_key)
            .expect("metadata value"),
        "initial metadata value"
    );
    assert_eq!(
        metadata_custom_value_new,
        object
            .metadata_custom
            .get(metadata_custom_key_new)
            .expect("metadata value"),
        "initial metadata value"
    );

    // Clean up.
    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up: delete bucket with all the objects not to fail");
}
