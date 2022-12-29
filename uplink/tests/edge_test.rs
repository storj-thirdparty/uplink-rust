use uplink::access::Grant;
use uplink::edge;

mod common;

const AUTH_SERVICE_URL: &str = "localhost:8888";

// TODO(https://github.com/storj-thirdparty/uplink-rust/issues/49): we need new Uplink API for
// being able to run this test successfully. Remove the `ignore` annotation and adjust the test
// with the new API.
#[test]
#[ignore]
fn integration_config_register_access() {
    let env = common::Environment::load();
    let access_grant = Grant::new(&env.access_grant).expect("access grant parsing");

    let config = edge::Config::new(AUTH_SERVICE_URL).expect("Edge config from AUTH service URL");
    let creds = config
        .register_gateway_access(&access_grant, None)
        .expect("Gateway credentials");
    assert_eq!(env.aws_access_key_id, creds.access_key_id, "access key ID");
    assert_eq!(env.aws_secret_access_key, creds.secret_key, "secret key");
    assert!(creds.endpoint != "", "not empty endpoint");
}

#[test]
fn integration_join_share_url() {
    const BASE_URL: &str = "https://link.us1.storjshare.io";
    let env = common::Environment::load();

    {
        // No bucket, no key.
        let url = edge::linksharing::share_url(BASE_URL, &env.aws_access_key_id, "", "", None)
            .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
    }

    {
        // With bucket, no key.
        let url =
            edge::linksharing::share_url(BASE_URL, &env.aws_access_key_id, "my-bucket", "", None)
                .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
        assert!(
            url.contains("my-bucket"),
            "must contain 'my-bucket', got '{}'",
            url
        );
    }

    {
        // With bucket, and key
        let url = edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "my-bucket",
            "obj-name",
            None,
        )
        .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
        assert!(
            url.contains("my-bucket"),
            "must contain 'my-bucket', got '{}'",
            url
        );
        assert!(
            url.contains("obj-name"),
            "must contain 'obj-name', got '{}'",
            url
        );
    }

    {
        // With bucket, and key
        let url = edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "my-bucket-name",
            "obj-name-2",
            Some(&edge::linksharing::OptionsShareURL { raw: true }),
        )
        .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
        assert!(
            url.contains("my-bucket-name"),
            "must contain 'my-bucket-name', got '{}'",
            url
        );
        assert!(
            url.contains("obj-name-2"),
            "must contain 'obj-name-2', got '{}'",
            url
        );
        assert!(url.contains("/raw/"), "must contain '/raw/', got '{}'", url);
    }

    {
        // With bucket, object key, and raw options to false
        let url = edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "my-bucket-name",
            "obj-name-2",
            Some(&edge::linksharing::OptionsShareURL { raw: false }),
        )
        .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
        assert!(
            url.contains("my-bucket-name"),
            "must contain 'my-bucket-name', got '{}'",
            url
        );
        assert!(
            url.contains("obj-name-2"),
            "must contain 'obj-name-2', got '{}'",
            url
        );
        assert!(
            !url.contains("/raw/"),
            "must not contain '/raw/', got '{}'",
            url
        );
    }

    {
        // With bucket, no object key, and raw options to false
        let url = edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "my-bucket-name",
            "",
            Some(&edge::linksharing::OptionsShareURL { raw: false }),
        )
        .expect("shared URL without bucket");
        assert!(
            url.starts_with(BASE_URL),
            "must start with '{}', got '{}'",
            BASE_URL,
            url
        );
        assert!(
            url.contains(&env.aws_access_key_id),
            "must contain '{}', got '{}'",
            env.aws_access_key_id,
            url
        );
        assert!(
            url.contains("my-bucket-name"),
            "must contain 'my-bucket-name', got '{}'",
            url
        );
        assert!(
            !url.contains("/raw/"),
            "must not contain '/raw/', got '{}'",
            url
        );
    }

    {
        // Invalid URL.
        edge::linksharing::share_url(
            "invalid-url",
            &env.aws_access_key_id,
            "bucket-name",
            "object-name",
            Some(&edge::linksharing::OptionsShareURL { raw: true }),
        )
        .expect_err("invalid URL");
    }

    {
        // No access key.
        edge::linksharing::share_url(
            BASE_URL,
            "",
            "bucket-name",
            "object-name",
            Some(&edge::linksharing::OptionsShareURL { raw: true }),
        )
        .expect_err("access key cannot be empty");
    }

    {
        // Error object key without bucket.
        edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "",
            "object-name",
            Some(&edge::linksharing::OptionsShareURL { raw: true }),
        )
        .expect_err("bucket name required when object key isn't empty");
    }

    {
        // Error options without object key.
        edge::linksharing::share_url(
            BASE_URL,
            &env.aws_access_key_id,
            "my-bucket-name",
            "",
            Some(&edge::linksharing::OptionsShareURL { raw: true }),
        )
        .expect_err("object key required when raw it's set");
    }
}
