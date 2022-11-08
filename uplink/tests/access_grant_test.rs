use uplink::access::{Grant, Permission, SharePrefix};
use uplink::Result as UlResult;
use uplink::{error, Bucket, Error, Object, Project};

use std::io::{Read, Write};

mod common;

#[test]
fn integration_grant_new() {
    let env = common::Environment::load();
    let grant = Grant::new(&env.access_grant).expect("access grant parsing");

    assert_eq!(
        &env.access_grant,
        &grant.serialize().expect("serialize valid access grant"),
        "serialize"
    );

    assert_eq!(
        common::SATELLITE_ADDR,
        grant.satellite_address().expect("satellite address"),
        "satellite address"
    );
}

#[test]
fn integration_grant_request_access_with_passphrase() {
    let env = common::Environment::load();
    let grant = Grant::request_access_with_passphrase(
        common::SATELLITE_ADDR,
        &env.api_key,
        &env.encryption_secret,
    )
    .expect("request access grant not to fail");

    assert_eq!(
        &env.access_grant,
        &grant.serialize().expect("serialize valid access grant"),
        "requested access grant should be equal than the provided one when using the same API key and encryption secret",
    );
}

#[test]
fn integration_grant_override_encryption_key() {
    use uplink::EncryptionKey;

    let env = common::Environment::load();
    let grant_root = Grant::new(&env.access_grant).expect("access grant parsing");

    // Create bucket for user.
    let project = &mut Project::open(&grant_root);
    let bucket_name = common::generate_name("multitenant");
    let (_bucket, _ok) = project.create_bucket(&bucket_name).expect("create bucket");

    // Create an access grant for the user and restrict it to its bucket.
    let grant_user = grant_root
        .share(
            &Permission::full(),
            Some(vec![
                SharePrefix::full_bucket(&bucket_name).expect("share prefix creation")
            ]),
        )
        .expect("no error creating user's grant");

    // User create its encryption key and override the key of the provided access grant.
    let key_user =
        EncryptionKey::derive("pass", "salt".as_bytes()).expect("deriving encryption key");
    grant_user
        .override_encryption_key(&bucket_name, "/", &key_user)
        .expect("no error overriding grant encryption key");

    {
        // Upload an object with the user's grant.
        let proj = &mut Project::open(&grant_user);
        let object_key = "overridden-encryption-key-data.txt";
        let upload = &mut proj
            .upload_object(&bucket_name, object_key, None)
            .expect("upload object");
        let object_data = String::from("Uplink Rust test object: overridden encryption key");
        upload
            .write_all(object_data.as_bytes())
            .expect("upload object write data");
        upload.commit().expect("upload object commit");

        // Download the object with the root grant should fail.
        let proj = &mut Project::open(&grant_root);
        proj.download_object(&bucket_name, object_key, None)
            .expect_err("when tyring to download user's object with the root access grant");
    }

    project
        .delete_bucket_with_objects(&bucket_name)
        .expect("clean up delete bucket with objects");
}

#[test]
fn integration_grant_share() {
    let env = common::Environment::load();
    let grant_root = Grant::new(&env.access_grant).expect("access grant parsing");
    let project = &mut Project::open(&grant_root);

    // Create buckets.
    let bucket1_name = common::generate_name("grant-share-1");
    let (_bucket1, ok) = project.create_bucket(&bucket1_name).expect("create bucket");
    assert!(ok, "bucket shouldn't exist",);

    let bucket2_name = common::generate_name("grant-share-2");
    let (_bucket2, ok) = project.create_bucket(&bucket2_name).expect("create bucket");
    assert!(ok, "bucket shouldn't exist",);

    // Upload objects with the root grant.
    let object_key_root = "root-data.txt";
    let upload = &mut project
        .upload_object(&bucket1_name, object_key_root, None)
        .expect("upload object");

    let object_data = String::from("Uplink Rust test object: Root access grant");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    let upload = &mut project
        .upload_object(&bucket2_name, object_key_root, None)
        .expect("upload object");

    let object_data = String::from("Uplink Rust test object: Root access grant");
    upload
        .write_all(object_data.as_bytes())
        .expect("upload object write data");
    upload.commit().expect("upload object commit");

    {
        // Create an access grant with write only permissions to a certain bucket and check that it
        // can only perform the indicated operations.

        let share_prefix = SharePrefix::full_bucket(&bucket1_name).expect("create share prefix");
        assert_eq!(&bucket1_name, share_prefix.bucket(), "share prefix: bucket");
        assert_eq!("", share_prefix.prefix(), "share prefix: prefix");

        // Create access grant with only upload data to one of the buckets.
        let grant = grant_root
            .share(&Permission::write_only(), Some(vec![share_prefix]))
            .expect("shared grant");

        // Listing buckets with this restricted access grant.
        let proj_restricted = &mut Project::open(&grant);
        let it = proj_restricted.list_buckets(None);
        assert_eq!(
            1,
            it.count(),
            "restricted access to a bucket can only list that bucket"
        );

        // Uploading an object with this restrictive access grant.
        let object_key_writeonly = "write-only-data.txt";
        let upload = &mut proj_restricted
            .upload_object(&bucket1_name, object_key_writeonly, None)
            .expect("upload object");

        let object_data = String::from("Uplink Rust test object: write-only access grant");
        upload
            .write_all(object_data.as_bytes())
            .expect("upload object write data");
        upload.commit().expect("upload object commit");

        // Verify that the object was uploaded with the restricted access grant.
        let it = project
            .list_objects(&bucket1_name, None)
            .expect("list objects not to fail");
        let res: UlResult<Vec<Object>> = it.collect();
        assert!(
            res.is_ok(),
            "root access grant doesn't return an error when listing objects",
        );
        assert_eq!(
            2,
            res.unwrap().len(),
            "bucket 1 after write-only access uploaded an object"
        );

        // Listing objects with this restricted access grant.
        let it = proj_restricted
            .list_objects(&bucket1_name, None)
            .expect("list objects not to fail");
        let res: UlResult<Vec<Object>> = it.collect();
        assert!(
            res.is_err(),
            "write-only access grant returns an error when listing objects",
        );
        match res.unwrap_err() {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when listing objects with a write-only restricted access grant", err),
        };

        // Deleting buckets with this restricted access grant.
        let res = proj_restricted.delete_bucket_with_objects(&bucket1_name);
        assert!(
            res.is_err(),
            "write-only access grant returns an error when deleting buckets",
        );
        match res.unwrap_err() {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when deleting buckets with a write-only restricted access grant", err),
        };

        // Deleting objects with the write-only access grant.
        let res = proj_restricted.delete_object(&bucket1_name, &object_key_writeonly);
        assert!(
            res.is_ok(),
            "write-only access grant returns no error when deleting objects",
        );
        assert!(
            res.unwrap().is_none(),
            "write-only access grant returns no object when deleting it because it doesn't have read access",
        );
    }

    {
        // Test access 2 access grants restricted to a certain prefix. Each one has a different
        // permission, one has permissions to upload and the other done to download.

        let share_prefix_upload =
            SharePrefix::new(&bucket1_name, "/pair").expect("create share prefix");
        assert_eq!(
            &bucket1_name,
            share_prefix_upload.bucket(),
            "share prefix: bucket"
        );
        assert_eq!(
            "/pair",
            share_prefix_upload.prefix(),
            "share prefix: prefix"
        );

        // Create an access grant with only upload permissions and restricted to certain prefix and
        // upload an object.
        let mut perm = Permission::new();
        perm.allow_upload = true;

        let grant_upload = grant_root
            .share(&perm, Some(vec![share_prefix_upload]))
            .expect("shared grant");

        let proj_upload = &mut Project::open(&grant_upload);
        let object_key = "/pair/data.txt";
        let upload = &mut proj_upload
            .upload_object(&bucket1_name, object_key, None)
            .expect("upload object");

        let object_data =
            String::from("Uplink Rust test object: upload & download access grants pair");
        upload
            .write_all(object_data.as_bytes())
            .expect("upload object write data");
        upload
            .commit()
            .expect("commit an object upload to another prefix");

        let upload = &mut proj_upload
            .upload_object(&bucket1_name, &format!("{}2", object_key), None)
            .expect("upload object");
        upload
            .write_all(object_data.as_bytes())
            .expect("upload object write data");
        upload
            .commit()
            .expect("commit an object upload to another prefix");

        // Uploading to another prefix fails.
        let upload = &mut proj_upload
            .upload_object(&bucket1_name, "/pair-2/data.txt", None)
            .expect("upload object");

        let object_data =
            String::from("Uplink Rust test object: upload & download access grants pair");
        match upload
            .write_all(object_data.as_bytes())
            .expect_err("upload object write data")
            .kind()
        {
            std::io::ErrorKind::Other => {}
            kind => panic!(
                "{} is an unexpected std::io::ErrorKind when uploading an object to another prefix",
                kind
            ),
        }
        match upload
            .commit()
            .expect_err("commit an object upload to another prefix")
        {
            Error::Uplink(error::Uplink::Internal(_)) => {}
            err => panic!(
                "{} is an unexpected error when uploading an object to another prefix",
                err
            ),
        };

        // Create an access grant with only download permissions and restricted to certain prefix
        // and download the previously uploaded object.
        let mut perm = Permission::new();
        perm.allow_download = true;
        let grant_download = grant_root
            .share(
                &perm,
                Some(vec![
                    SharePrefix::new(&bucket1_name, object_key).expect("create share prefix")
                ]),
            )
            .expect("shared grant");

        let proj_download = &mut Project::open(&grant_download);
        let download = &mut proj_download
            .download_object(&bucket1_name, object_key, None)
            .expect("download object");

        download.info().expect("object info");
        let mut downloaded_data = String::new();
        download
            .read_to_string(&mut downloaded_data)
            .expect("downloaded data object");
        assert_eq!(object_data, downloaded_data, "downloaded data");

        // Downloading from another prefix.
        match &mut proj_download
            .download_object(&bucket1_name, &format!("{}2", object_key), None)
            .expect_err("download object")
        {
            Error::Uplink(error::Uplink::Internal(_)) => {}
            err => panic!(
                "{} is an unexpected error when downloading an object from another prefix",
                err
            ),
        };

        // None of access grants can list.
        let it = proj_upload
            .list_objects(&bucket1_name, None)
            .expect("list objects without list permissions");
        match it.collect::<UlResult<Vec<Object>>>().expect_err("list objects iterator without list permissions") {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when listing objects with an upload restricted access grant", err),
        };

        let it = proj_download
            .list_objects(&bucket1_name, None)
            .expect("list objects without list permissions");
        match it.collect::<UlResult<Vec<Object>>>().expect_err("list objects iterator without list permissions") {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when listing objects with an upload restricted access grant", err),
        };

        // Upload access grant cannot download the object.
        let err = &mut proj_upload
            .download_object(&bucket1_name, object_key, None)
            .expect_err("download object");
        match err {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when downloading the object with an upload restricted access grant", err),
        };

        // Download access grant cannot upload the object.
        let upload = &mut proj_download
            .upload_object(&bucket1_name, object_key, None)
            .expect("upload object");
        upload
            .write_all(object_data.as_bytes())
            .expect("upload object write data");
        match upload.commit().expect_err("commit an object upload with a download restricted access grant") {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when uploading an object with a download restricted access grant", err),
        };

        // None of the access grants can delete the object.
        let err = proj_upload
            .delete_object(&bucket1_name, object_key)
            .expect_err("list objects without delete permissions");
        match err {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when deleting the object with an upload restricted access grant", err),
        };

        let err = proj_download
            .delete_object(&bucket1_name, object_key)
            .expect_err("list objects without delete permissions");
        match err {
            Error::Uplink(error::Uplink::Internal(_)) => {},
            err => panic!("{} is an unexpected error when deleting the object with a download restricted access grant", err),
        };
    }

    {
        // Test an access grant that's restricted to a certain period of time.

        use std::thread;
        use std::time::Duration;

        let mut perm = Permission::full();
        perm.set_not_before(Some(Duration::from_secs(
            common::seconds_since_unix_epoch() + 1,
        )))
        .expect("setting not before to sharing permissions");
        perm.set_not_after(Some(Duration::from_secs(
            common::seconds_since_unix_epoch() + 3,
        )))
        .expect("setting not before to sharing permissions");
        let grant = grant_root.share(&perm, None).expect("shared grant");

        let project = &mut Project::open(&grant);
        let it = project.list_buckets(None);
        match it.collect::<UlResult<Vec<Bucket>>>().expect_err("listing buckets with a grant that cannot be used before a future date") {
            Error::Uplink(error::Uplink::Internal(_)) => {}
            err => panic!(
                "{} is an unexpected error when listing buckets with a grant that cannot be used before a future date",
                err
            ),
        };

        thread::sleep(Duration::from_secs(1));
        let it = project.list_buckets(None);
        let buckets = it
            .collect::<UlResult<Vec<Bucket>>>()
            .expect("listing buckets with a grant that has a not before in the past");
        assert_eq!(2, buckets.len(), "number of buckets");

        thread::sleep(Duration::from_secs(2));
        let it = project.list_buckets(None);
        match it.collect::<UlResult<Vec<Bucket>>>().expect_err("listing buckets with a grant that cannot be used after a past date") {
            Error::Uplink(error::Uplink::Internal(_)) => {}
            err => panic!(
                "{} is an unexpected error when listing buckets with a grant that cannot be used after a past date",
                err
            ),
        };
    }

    project
        .delete_bucket_with_objects(&bucket1_name)
        .expect("clean up delete bucket with objects");
    project
        .delete_bucket_with_objects(&bucket2_name)
        .expect("clean up delete bucket with objects");
}
