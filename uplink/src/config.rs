//! Storj DCS Uplink configuration.

use crate::{helpers, Result};

use std::ffi::CString;
use std::time::Duration;

use uplink_sys as ulksys;

/// Defines configuration for using Uplink library.
#[derive(Debug)]
pub struct Config<'a> {
    /// The configuration type of the FFI that an instance of this struct represents and guard its
    /// lifetime until this instance drops.
    pub(crate) inner: ulksys::UplinkConfig,

    /// Identifies the application how is contacting with the satellite.
    /// The user agent is used for statistics and for identifying the usage coming from associated
    /// partners.
    user_agent: &'a str,
    /// Defines how long the client should wait for establishing a connection to  peers.
    dial_timeout: Duration,
    /// Path to a directory to be used for storing temporary files when running completely in memory
    /// is disabled. It's `None` when running only in memory.
    temp_dir: Option<&'a str>,
    /// Specifies to only operates using memory, hence it doesn't off-load data to disk.
    in_memory: bool,
}

impl<'a> Config<'a> {
    /// Creates a configuration with the specific user agent, dial timeout and using a specific
    /// directory path for creating temporary files.
    ///
    /// Some operations performed by this configuration or any instance created from it may offload
    /// data from memory to disk.
    ///
    /// When `temp_dir`is `None` or an empty string, a random directory path will be used.
    ///
    /// NOTE:
    /// * Even that the FFI offers this option, it may not use it and just fully operates in memory.
    /// * The directory path isn't checked so the result of using a directory which doesn't exist
    ///   will depend on the result of the FFI at the moment of using the configuration.
    pub fn new(
        user_agent: &'a str,
        dial_timeout: Duration,
        temp_dir: Option<&'a str>,
    ) -> Result<Self> {
        let inner;
        {
            let uagent = helpers::cstring_from_str_fn_arg("user_agent", user_agent)?;
            let tdir = temp_dir.unwrap_or("");
            let tdir = helpers::cstring_from_str_fn_arg("temp_dir", tdir)?;

            inner = ulksys::UplinkConfig {
                user_agent: uagent.into_raw(),
                dial_timeout_milliseconds: dial_timeout.as_millis() as i32,
                temp_directory: tdir.into_raw(),
            };
        }

        Ok(Config {
            inner,
            user_agent,
            dial_timeout,
            temp_dir,
            in_memory: false,
        })
    }

    /// Creates a configuration with the specific user agent and dial timeout.
    /// All the operations performed by this configuration or any instance created from it will
    /// operate entirely in memory.
    pub fn new_inmemory(user_agent: &'a str, dial_timeout: Duration) -> Result<Self> {
        let inner;
        {
            let uagent = helpers::cstring_from_str_fn_arg("user_agent", user_agent)?;
            let temp_dir = CString::new("inmemory")
                .expect("BUG: hard-coded temp_dir string must never contains  null bytes (0 byte)");
            inner = ulksys::UplinkConfig {
                user_agent: uagent.into_raw(),
                dial_timeout_milliseconds: dial_timeout.as_millis() as i32,
                temp_directory: temp_dir.into_raw(),
            };
        }

        Ok(Config {
            inner,
            user_agent,
            dial_timeout,
            temp_dir: None,
            in_memory: true,
        })
    }

    /// Returns the configured dial timeout.
    pub fn dial_timeout(&self) -> Duration {
        self.dial_timeout
    }

    /// Returns if the configuration is specifying to use only memory or not.
    ///
    /// It returns `true` and always `None` when it only uses memory, otherwise `false` and:
    /// * `None` when using a random directory.
    /// * `Some` when a temporary directory path is specified.
    pub fn is_inmemory(&self) -> (bool, Option<&str>) {
        if self.in_memory {
            (true, None)
        } else {
            (false, self.temp_dir)
        }
    }

    /// Returns the configured user agent.
    pub fn user_agent(&self) -> &str {
        self.user_agent
    }

    /// Returns the FFI representation of this configuration.
    pub(crate) fn as_ffi_config(&self) -> ulksys::UplinkConfig {
        self.inner
    }
}

impl Drop for Config<'_> {
    fn drop(&mut self) {
        use std::os::raw::c_char;

        // SAFETY: The inner field is initialized when an instance of this struct is initialized and
        // it's only used by this crate to passed to the FFI.
        // The FFI never free the memory or mutate the fields of its exposed struct instance held by
        // the inner field, hence the lifetime of its fields which are pointers belong to this
        // instance, so we must free when this instance drops.
        // The 2 pointers explicitly freed here came from the call to the `into_raw` method of the
        // `CString` instances crated from `&str`.
        unsafe {
            // Retake ownership of the CString(s) transferred to `self.inner`
            // for freeing its memory when the created CString drops.

            // `self.inner.user_agent` and `self.inner.temp_directory` are never
            // null, otherwise there is bug in the implementation of this
            // struct.
            let _ = CString::from_raw(self.inner.user_agent as *mut c_char);
            let _ = CString::from_raw(self.inner.temp_directory as *mut c_char);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helpers::test::assert_c_string;
    use crate::{error, Error};

    #[test]
    fn test_new() {
        {
            // OK case: use a randomly generated temp directory.
            let ua = "rust-uplink";
            let config = Config::new(ua, Duration::new(2, 5000000), None)
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(config.user_agent, ua, "user_agent");
            assert_eq!(
                config.dial_timeout,
                Duration::new(2, 5000000),
                "dial_timeout"
            );
            assert_eq!(config.temp_dir, None, "temp_dir");
            assert!(!config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, ua);
            assert_ne!(config.inner.temp_directory, std::ptr::null());
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 2005,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // OK case: use a specific temp directory.
            let ua = "rust-uplink-custom-temp-dir";
            let temp_dir = "/tmp/rust-uplink";
            let config = Config::new(ua, Duration::new(1, 785999999), Some(temp_dir))
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(config.user_agent, ua, "user_agent");
            assert_eq!(
                config.dial_timeout,
                Duration::new(1, 785999999),
                "dial_timeout"
            );
            assert_eq!(config.temp_dir, Some(temp_dir), "temp_dir");
            assert!(!config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, ua);
            assert_c_string(config.inner.temp_directory, temp_dir);
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 1785,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // Error case: User agent has null characters.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new("rust-uplink\0", Duration::new(3, 0), None)
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "user_agent", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 11",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
        {
            // Error case: Temp directory has null characters.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new("rust-uplink", Duration::new(3, 0), Some("\0invalid"))
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "temp_dir", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 0",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_new_inmemory() {
        {
            // OK case.
            let config = Config::new_inmemory("rust-uplink", Duration::new(3, 0))
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(config.user_agent, "rust-uplink", "user_agent");
            assert_eq!(config.dial_timeout, Duration::new(3, 0), "dial_timeout");
            assert_eq!(config.temp_dir, None, "temp_dir");
            assert!(config.in_memory, "in_memory");

            assert_c_string(config.inner.user_agent, "rust-uplink");
            assert_c_string(config.inner.temp_directory, "inmemory");
            assert_eq!(
                config.inner.dial_timeout_milliseconds, 3000,
                "inner.dial_tiemout_milliseconds"
            );
        }
        {
            // Error case.
            if let Error::InvalidArguments(error::Args { names, msg }) =
                Config::new_inmemory("rust\0uplink", Duration::new(3, 0))
                    .expect_err("new passing a user agent with NULL bytes")
            {
                assert_eq!(names, "user_agent", "invalid error argument name");
                assert_eq!(
                    msg, "cannot contains null bytes (0 byte). Null byte found at 4",
                    "invalid error argument message"
                );
            } else {
                panic!("expected an invalid argument error");
            }
        }
    }

    #[test]
    fn test_dial_timeout() {
        let config = Config::new("rust-uplink", Duration::new(1, 635578), None)
            .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

        assert_eq!(
            config.dial_timeout(),
            Duration::new(1, 635578),
            "dial_timeout"
        );
    }

    #[test]
    fn test_is_inmeory() {
        {
            // Using disk with random temp directory path.
            let config = Config::new("rust-uplink", Duration::new(1, 635578), None)
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(
                config.is_inmemory(),
                (false, None),
                "disk and random directory"
            );
        }
        {
            // Using disk with a specific temp directory path.
            let config = Config::new(
                "rust-uplink",
                Duration::new(1, 635578),
                Some("/tmp/uplink-rs"),
            )
            .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(
                config.is_inmemory(),
                (false, Some("/tmp/uplink-rs")),
                "disk and specific directory "
            );
        }
        {
            // Using only memory case.
            let config = Config::new_inmemory("rust-uplink", Duration::new(1, 635578))
                .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

            assert_eq!(config.is_inmemory(), (true, None), "using only memory");
        }
    }

    #[test]
    fn test_user_agent() {
        let config = Config::new("rust-uplink", Duration::new(1, 635578), None)
            .expect("new shouldn't fail when 'user agent' doesn't contain any null character");

        assert_eq!(config.user_agent(), "rust-uplink", "user_agent");
    }
}
