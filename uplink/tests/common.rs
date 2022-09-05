use std::env;
use std::time::{Duration, SystemTime};

pub struct Environment {
    pub user: String,
    pub project_id: String,
    pub access_grant: String,
    pub s3_gateway_url: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
}

impl Environment {
    pub fn load() -> Self {
        Self {
            user: env::var("STORJ_USER").expect("STORJ_USER env var isn't defined"),
            project_id: env::var("STORJ_PROJECT_ID")
                .expect("STORJ_PROJECT_ID env var isn't defined"),
            access_grant: env::var("STORJ_ACCESS").expect("STORJ_ACCESS env var isn't defined"),
            s3_gateway_url: env::var("STORJ_GATEWAY").expect("STORJ_GATEWAY env var isn't defined"),
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID")
                .expect("AWS_ACCESS_KEY_ID env var isn't defined"),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY")
                .expect("AWS_SECRET_ACCESS_KEY env var isn't defined"),
        }
    }
}

pub fn generate_name<'a>(ctx: &'a str) -> String {
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time since Unix epoch failed");
    format!("uplink-rust-{}-{}", ctx, d.as_nanos())
}

/// Asserts that `timestamp` is older than current timestamp but not older than the current
/// timestamp less `leeway`.
///
/// `timestamp` must be calculated since epoch.
pub fn assert_epoch_timestamp_from_now<'a>(
    timestamp: Duration,
    leeway: Duration,
    ctx_msg: &'a str,
) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time since Unix epoch failed");

    assert!(
        now.as_nanos() - timestamp.as_nanos() <= leeway.as_nanos(),
        "timestamp is older than now and leeway {}",
        ctx_msg,
    );
}

/// Get numbers of seconds since UNIX epoch at the time to call this function.
pub fn seconds_since_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time since Unix epoch failed")
        .as_secs()
}
