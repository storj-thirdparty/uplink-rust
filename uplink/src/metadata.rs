//! Storj DCS metadata types.

use std::collections::HashMap;
use std::ffi::c_char;
use std::ptr;
use std::time::Duration;
use std::vec::Vec;

use uplink_sys as ulksys;

/// It's a container for custom information of a specific "item".
/// It's provided by the users as key-value pairs which must only contain valid
/// UTF-8 characters. Keys are unique, so only one value can be associated with
/// it.
///
/// By convention an application that stores metadata should prepend to the keys
/// a prefix, for example an application named "Image Board" might use the
/// "image-board:" prefix and a key could be "image-board:title".
#[derive(Default, Debug)]
pub struct Custom {
    /// The key-value pairs.
    entries: HashMap<String, String>,

    /// Cached FFI representation of this instance that guards its lifetime while it's hold by this
    /// field or this instance drops.
    ///
    /// It's an option because it's only created when calling [`Self::to_ffi_custom_metadata`] and
    /// hold it meanwhile this instance isn't mutated.
    inner: Option<UplinkCustomMetadataWrapper>,
}

impl Custom {
    /// Creates an empty custom metadata with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let map = HashMap::with_capacity(capacity);

        Self {
            entries: map,
            inner: None,
        }
    }

    /// Creates a custom metadata instance from type exposed by the FFI.
    ///
    /// NOTE this method assumes `uc_custom` only contains key-value pairs that have valid UTF-8
    /// bytes. In the case that it doesn't then the mapped key-value may not have the same value in
    /// that byte position and it isn't either guarantee that the same invalid UTF-8 byte produces
    /// the same mapped value.
    pub(crate) fn with_ffi_custom_metadata(uc_custom: &ulksys::UplinkCustomMetadata) -> Self {
        if uc_custom.count == 0 {
            return Default::default();
        }

        let mut custom = Self::with_capacity(uc_custom.count);
        // SAFETY: we trust that the FFI contains a valid pointer to entries and the counter has
        // the exact number of entries, and each entry has a key-value C string with exactly the
        // length specified without leaning that they end with the NULL byte because they could
        // contain NULL bytes.
        unsafe {
            use crate::helpers::unchecked_ptr_c_char_and_length_to_string;

            for i in 0..uc_custom.count as isize {
                let entry = uc_custom.entries.offset(i) as *const ulksys::UplinkCustomMetadataEntry;
                let key =
                    unchecked_ptr_c_char_and_length_to_string((*entry).key, (*entry).key_length);
                let value = unchecked_ptr_c_char_and_length_to_string(
                    (*entry).value,
                    (*entry).value_length,
                );

                custom.insert(key, value);
            }
        }

        custom
    }

    /// Returns the current number of entries (i.e. key-value pairs).
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Gets the entry's value associated with the passed key. Returns none if
    /// there isn't any entry associated to the key.
    pub fn get(&self, key: &str) -> Option<&String> {
        match self.entries.get(key) {
            Some(v) => Some(v),
            None => None,
        }
    }

    /// Inserts a new entry with the specified key and value, returning false if
    /// the key didn't exit, otherwise true and replace the value associated to
    /// the key.
    pub fn insert(&mut self, key: String, value: String) -> bool {
        self.inner = None;
        self.entries.insert(key, value).is_some()
    }

    /// An iterator for visiting all the metadata key-value pairs.
    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&String, &String)> {
        self.entries.iter()
    }

    /// Deletes the entry with the associated key, returning false if the key
    /// didn't exist, otherwise true.
    pub fn delete(&mut self, key: &str) -> bool {
        self.inner = None;
        self.entries.remove(key).is_some()
    }

    /// Returns the FFI representation of this custom metadata container which is valid as long as
    /// `self` isn't mutated or dropped.
    ///
    /// When this method is called more than once and `self` isn't mutated in between, the calls
    /// after the first are very cheap because the returned value is cached.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_ffi_custom_metadata(&mut self) -> ulksys::UplinkCustomMetadata {
        if self.inner.is_none() {
            self.inner = Some(UplinkCustomMetadataWrapper::from_custom(self));
        }

        // We have ensured that `inner` is not None just above so `unwrap` will never panic.
        self.inner.as_ref().unwrap().custom_metadata
    }
}

impl Clone for Custom {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            inner: None,
        }
    }
}

/// It allows to create an [`uplink_sys::UplinkCustomMetadata`] instance that
/// guards the used memory of its list of items during the lifetime of the
/// instance of this struct.
#[derive(Debug)]
struct UplinkCustomMetadataWrapper {
    /// The [`uplink_sys::UplinkCustomMetadata`] instance that `self`
    /// represents.
    custom_metadata: ulksys::UplinkCustomMetadata,
    /// The allocated memory of the list of entries referenced by the FFI value in the field
    /// `custom_metadata` and whose lifetime is guarded by an instance of `Self`.
    _entries: Vec<ulksys::UplinkCustomMetadataEntry>,
}

impl UplinkCustomMetadataWrapper {
    /// Creates a wrapped [`uplink_sys::UplinkCustomMetadata`]  which represents
    /// the passed [`Custom`].
    fn from_custom(custom: &Custom) -> Self {
        let num_entries = custom.count();
        if num_entries == 0 {
            return Default::default();
        }

        let mut entries = Vec::with_capacity(num_entries);
        for (k, v) in custom.iter() {
            entries.push(ulksys::UplinkCustomMetadataEntry {
                key: k.as_ptr() as *mut c_char,
                key_length: k.len(),
                value: v.as_ptr() as *mut c_char,
                value_length: v.len(),
            });
        }

        UplinkCustomMetadataWrapper {
            custom_metadata: ulksys::UplinkCustomMetadata {
                entries: entries.as_mut_ptr(),
                count: entries.len(),
            },
            _entries: entries,
        }
    }
}

impl Default for UplinkCustomMetadataWrapper {
    fn default() -> Self {
        UplinkCustomMetadataWrapper {
            custom_metadata: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
            _entries: Vec::new(),
        }
    }
}

/// It's a container of system information of a specific "item".
/// It's provided by the service and only the service can alter it.
#[derive(Debug)]
pub struct System {
    /// When the associated "item" was created.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    pub created: Duration,
    /// When the associated "item" expires. When it never expires is `None`.
    ///
    /// The time is measured with the number of seconds since the Unix Epoch
    /// time.
    pub expires: Option<Duration>,
    /// Then length of the data associated to this metadata.
    ///
    /// NOTE it's a signed integer because the original library uses a signed
    /// integer, so it may be the case now or in the future that negatives
    /// numbers are used.
    pub content_length: i64,
}

impl System {
    /// Create a new instance from its FFI representation.
    ///
    /// The function doesn't check `created` nor `expires` have correct values.
    /// For example if they are negative or `expires` is less than `created`. Nonetheless because
    /// Rust `std::time::Duration` types don't support negative values, if any of them contains a
    /// negative or zero, `created` is set to zero duration and `expires` to `None`.
    pub fn with_ffi_system_metadata(uc_system: &ulksys::UplinkSystemMetadata) -> Self {
        let created = if uc_system.created > 0 {
            Duration::from_secs(uc_system.created as u64)
        } else {
            Duration::ZERO
        };

        let expires = if uc_system.expires > 0 {
            Some(Duration::from_secs(uc_system.expires as u64))
        } else {
            None
        };

        Self {
            created,
            expires,
            content_length: uc_system.content_length,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_custom_with_entries() {
        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";

        let mut custom = Custom::with_capacity(2);
        custom.insert(String::from(key1), String::from(val1));
        custom.insert(String::from(key2), String::from(val2));

        assert_eq!(custom.count(), 2, "count");
        assert_eq!(custom.get(key1), Some(&String::from(val1)), "get: 'key1'");
        assert_eq!(custom.get(key2), Some(&String::from(val2)), "get: 'key2'");
        assert_eq!(custom.get("unexisting"), None, "get 'unexisting'");
    }

    #[test]
    fn test_custom_insert() {
        let key1 = "key-a";
        let val1 = "val-a";
        let val1_2 = "val-a-2";
        let key2 = "key-b";
        let val2 = "val-b";

        let mut custom = Custom::with_capacity(2);
        custom.insert(String::from(key1), String::from(val1));

        assert_eq!(custom.count(), 1, "count before inserting a new key");
        assert_eq!(custom.get(key2), None, "get 'key2' before inserting it");
        assert!(
            !custom.insert(String::from(key2), String::from(val2)),
            "insert 'key2'"
        );
        assert_eq!(custom.count(), 2, "count after inserting a new key");
        assert_eq!(
            custom.get(key2),
            Some(&String::from(val2)),
            "get 'key2' after inserting it"
        );
        assert_eq!(
            custom.get(key1),
            Some(&String::from(val1)),
            "get 'key1' before updating it"
        );
        assert!(
            custom.insert(String::from(key1), String::from(val1_2)),
            "insert 'key1' with new value"
        );
        assert_eq!(custom.count(), 2, "count after inserting an existing key");
        assert_eq!(
            custom.get(key1),
            Some(&String::from(val1_2)),
            "get 'key1' after updating it"
        );
    }

    #[test]
    fn test_custom_remove() {
        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";

        let mut custom = Custom::with_capacity(2);
        custom.insert(String::from(key1), String::from(val1));
        custom.insert(String::from(key2), String::from(val2));

        assert_eq!(custom.count(), 2, "count before removing a new key");
        assert!(custom.delete(key1), "remove 'key1'");
        assert_eq!(custom.count(), 1, "count after removing a new key");
        assert_eq!(custom.get(key1), None, "get 'key1'");
        assert_eq!(custom.get(key2), Some(&String::from(val2)), "get 'key2'");
        assert!(!custom.delete(key1), "remove an unexisting key");
        assert_eq!(custom.count(), 1, "count after removing a unexisting key");
    }

    #[test]
    fn test_custom_clone() {
        let mut source = Custom::default();
        assert_eq!(
            source.count(),
            0,
            "count on 'source' after it's initialized with 'default'"
        );

        let clone = source.clone();
        assert_eq!(
            clone.count(),
            0,
            "count on 'clone' after cloning an instance with 0 entries"
        );

        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";
        assert!(
            !source.insert(String::from(key1), String::from(val1)),
            "insert 'key1' into 'source'"
        );

        let mut clone = source.clone();
        assert_eq!(
            clone.count(),
            1,
            "count on 'clone' after cloning an instance with 1 entries"
        );
        assert_eq!(
            clone.get(key1),
            Some(&String::from(val1)),
            "get 'key1' from 'clone'"
        );

        assert!(
            !source.insert(String::from(key2), String::from(val2)),
            "insert 'key2' into 'soure'"
        );
        assert_eq!(
            clone.count(),
            1,
            "count of 'clone' after inserting 'key2' in 'source'"
        );
        assert_eq!(
            clone.get(key1),
            Some(&String::from(val1)),
            "get 'key1' from 'clone' after inserting 'key2' in 'source'"
        );
        assert_eq!(
            clone.get(key2),
            None,
            "get 'key2' from 'clone' which has never been inserted"
        );

        assert!(source.delete(key1), "remove 'key1' from 'soruce'");
        assert_eq!(
            clone.count(),
            1,
            "count on 'clone' after removing 'key1' of 'source'"
        );
        assert_eq!(
            clone.get(key1),
            Some(&String::from(val1)),
            "get 'key1' from 'clone' after remove 'key1' of 'source'"
        );
        assert_eq!(
            source.count(),
            1,
            "count on 'source' before removing 'key1' of 'clone'"
        );
        assert!(clone.delete(key1), "remove 'key1' from 'clone'");
        assert_eq!(
            source.count(),
            1,
            "count on 'source' after removing 'key1' of 'clone'"
        );
    }

    #[test]
    fn test_custom_iterator() {
        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";

        let mut custom = Custom::with_capacity(2);
        custom.insert(String::from(key1), String::from(val1));
        custom.insert(String::from(key2), String::from(val2));

        assert_eq!(custom.count(), 2, "number of entries");
        for entry in (&custom).iter() {
            if entry.0 == &String::from(key1) && entry.1 == &String::from(val1) {
                continue;
            }
            if entry.0 == &String::from(key2) && entry.1 == &String::from(val2) {
                continue;
            }

            panic!("Custom shouldn't contains the entry {:#?}", entry);
        }
    }

    use crate::helpers::test::{assert_c_string, compare_c_string};

    #[test]
    fn test_custom_with_ffi_custom_metadata() {
        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";

        let from;
        {
            // This scope drops `to` for doing the commented check right after
            // the scope closes.
            let mut to = Custom::with_capacity(2);
            to.insert(String::from(key1), String::from(val1));
            to.insert(String::from(key2), String::from(val2));
            from = Custom::with_ffi_custom_metadata(&to.to_ffi_custom_metadata());

            assert_eq!(from.count(), 2, "count");
            assert_eq!(from.get(key1), Some(&String::from(val1)), "get: 'key1'");
            assert_eq!(from.get(key2), Some(&String::from(val2)), "get: 'key2'");

            // Ensure that to is dropped.
            drop(to);
        }

        // Check that a Custom instance generated from an UplinkCustomMetadata
        // that has dropped is still valid.
        assert_eq!(from.count(), 2, "count");
        assert_eq!(from.get(key1), Some(&String::from(val1)), "get: 'key1'");
        assert_eq!(from.get(key2), Some(&String::from(val2)), "get: 'key2'");
    }

    #[test]
    fn test_custom_to_ffi_custom_metadata() {
        let key1 = "key-a";
        let val1 = "val-a";
        let key2 = "key-b";
        let val2 = "val-b";

        let mut custom = Custom::with_capacity(2);
        custom.insert(String::from(key1), String::from(val1));
        custom.insert(String::from(key2), String::from(val2));

        let c_custom = custom.to_ffi_custom_metadata();
        assert_eq!(c_custom.count, 2, "count");

        let c_entries = c_custom.entries as *const ulksys::UplinkCustomMetadataEntry;
        unsafe {
            for i in 0..1 {
                let entry = *c_entries.offset(i);

                if compare_c_string(entry.key, key1).is_none() {
                    assert_c_string(entry.value, val1);
                    continue;
                }

                if compare_c_string(entry.key, key2).is_none() {
                    assert_c_string(entry.value, val2);
                    continue;
                }

                panic!("UplinkCustomMetadata instance doesn't contains one of the expected keys ({}, {})", key1, key2);
            }
        }

        // Modify the custom metadata and verify that the methods returns an
        // UplinkCustomMetadata which reflets the current custom metadata state.
        custom.delete(key1);

        let c_custom = custom.to_ffi_custom_metadata();
        assert_eq!(c_custom.count, 1, "count");

        let c_entries = c_custom.entries as *const ulksys::UplinkCustomMetadataEntry;
        unsafe {
            let entry = *c_entries;
            assert_c_string(entry.key, key2);
            assert_c_string(entry.value, val2);
        }
    }

    #[test]
    fn test_system_with_ffi_system_metadata() {
        {
            // 0 expiration
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 3600,
                expires: 0,
                content_length: 10,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(sysm.expires, None, "zero expires");
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // postive correct expiration
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 3600,
                expires: 3601,
                content_length: 10,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // postive incorrect expiration (it's before created)
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 3600,
                expires: 100,
                content_length: 10,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires before created"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // 0 content length
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 3600,
                expires: 9999,
                content_length: 0,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "zero content length"
            );
        }

        {
            // negative content length
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 3600,
                expires: 9999,
                content_length: -3,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "negative content length"
            );
        }

        {
            // negative created
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: -1,
                expires: 9999,
                content_length: 99,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(sysm.created, Duration::ZERO, "negative created");
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // 0 created
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 75,
                content_length: 99,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(sysm.created, Duration::ZERO, "zero created");
            assert_eq!(
                sysm.expires,
                Some(Duration::from_secs(uc_sysm.expires as u64)),
                "positive expires"
            );
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // 0 expiration
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 876543,
                expires: 0,
                content_length: 99,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(sysm.expires, None, "zero expires");
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }

        {
            // negative expiration
            let uc_sysm = ulksys::UplinkSystemMetadata {
                created: 876543,
                expires: -1,
                content_length: 99,
            };

            let sysm = System::with_ffi_system_metadata(&uc_sysm);
            assert_eq!(
                sysm.created,
                Duration::from_secs(uc_sysm.created as u64),
                "positive created"
            );
            assert_eq!(sysm.expires, None, "negative expires");
            assert_eq!(
                sysm.content_length, uc_sysm.content_length,
                "positive content length"
            );
        }
    }
}
