//! Convenient methods implemented for the original `uplink-sys` types.

use uplink_sys as ulksys;

/// An interface for ensuring that an instance of type returned by the FFI is correct in terms that
/// it doesn't violate its own rules.
///
/// For example a UplinkAccessResult struct has 2 fields which are 2 pointers,
/// one is the access and the other is an error, always one and only one can be
/// NULL.
pub(crate) trait Ensurer {
    /// Does a shallow check to ensure that the instance is correct according its own rules and it
    /// returns itself, otherwise it panics.
    fn ensure(&self) -> &Self;
}

impl Ensurer for ulksys::UplinkAccessResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.access.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkAccessResult; access and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkBucket {
    fn ensure(&self) -> &Self {
        assert!(
            !self.name.is_null(),
            "FFI returned an invalid UplinkBucket; name field is NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkBucketResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.bucket.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkBucketResult; bucket and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkCommitUploadResult {
    fn ensure(&self) -> &Self {
        assert!(!self.object.is_null() || !self.error.is_null(), "FFI returned an invalid UplinkCommitUploadResult; object and error fields are both NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkDownloadResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.download.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkDownloadResult; download and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkEncryptionKeyResult {
    fn ensure(&self) -> &Self {
        assert!(!self.encryption_key.is_null() || !self.error.is_null(), "FFI returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkObject {
    fn ensure(&self) -> &Self {
        assert!(
            !self.key.is_null(),
            "FFI returned an invalid UplinkObject; key field is NULL",
        );

        self
    }
}

impl Ensurer for ulksys::UplinkObjectResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.object.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkObjectResult; object and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkPartResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.part.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkPartResult; part and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkPartUploadResult {
    fn ensure(&self) -> &Self {
        assert!(!self.part_upload.is_null() || !self.error.is_null(), "FFI returned an invalid UplinkPartUploadResult; part_upload and error fields are both NULL");
        self
    }
}

impl Ensurer for ulksys::UplinkStringResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.string.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkStringResult; string and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkUploadInfo {
    fn ensure(&self) -> &Self {
        assert!(
            !self.upload_id.is_null(),
            "FFI returned an invalid UplinkUploadInfo; upload_id field is NULL"
        );
        assert!(
            !self.key.is_null(),
            "FFI returned an invalid UplinkUploadInfo; key field is NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkUploadInfoResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.info.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkUploadInfoResult; info and error fields are both NULL"
        );
        self
    }
}

impl Ensurer for ulksys::UplinkUploadResult {
    fn ensure(&self) -> &Self {
        assert!(
            !self.upload.is_null() || !self.error.is_null(),
            "FFI returned an invalid UplinkUploadResult; upload and error fields are both NULL"
        );
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_ensurer_access_result_valid() {
        {
            // Has an access.
            let acc_res = ulksys::UplinkAccessResult {
                access: &mut ulksys::UplinkAccess { _handle: 0 },
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            acc_res.ensure();
        }

        {
            // Has an error.
            let acc_res = ulksys::UplinkAccessResult {
                access: ptr::null_mut::<ulksys::UplinkAccess>(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            acc_res.ensure();
        }

        {
            // Has an access and an error.
            let acc_res = ulksys::UplinkAccessResult {
                access: &mut ulksys::UplinkAccess { _handle: 0 },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            acc_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkAccessResult; access and error fields are both NULL"
    )]
    fn test_ensurer_access_result_invalid_both_null() {
        let acc_res = ulksys::UplinkAccessResult {
            access: ptr::null_mut::<ulksys::UplinkAccess>(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        acc_res.ensure();
    }

    #[test]
    fn test_ensurer_bucket_valid() {
        let bucket = ulksys::UplinkBucket {
            name: CString::new("bucket-name").unwrap().into_raw(),
            created: 0,
        };
        bucket.ensure();
    }

    #[test]
    #[should_panic(expected = "FFI returned an invalid UplinkBucket; name field is NULL")]
    fn test_ensurer_bucket_invalid() {
        let bucket = ulksys::UplinkBucket {
            name: ptr::null_mut(),
            created: 0,
        };
        bucket.ensure();
    }

    #[test]
    fn test_ensurer_bucket_result_valid() {
        {
            // Has a bucket.
            let bucket_res = ulksys::UplinkBucketResult {
                bucket: &mut ulksys::UplinkBucket {
                    name: ptr::null_mut(),
                    created: 0,
                },
                error: ptr::null_mut(),
            };

            bucket_res.ensure();
        }

        {
            // Has an error.
            let bucket_res = ulksys::UplinkBucketResult {
                bucket: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            bucket_res.ensure();
        }

        {
            // Has a bucket and an error.
            let bucket_res = ulksys::UplinkBucketResult {
                bucket: &mut ulksys::UplinkBucket {
                    name: ptr::null_mut(),
                    created: 0,
                },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            bucket_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkBucketResult; bucket and error fields are both NULL"
    )]
    fn test_ensurer_bucket_result_invalid_both_null() {
        let bucket_res = ulksys::UplinkBucketResult {
            bucket: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        bucket_res.ensure();
    }

    #[test]
    fn test_ensurer_commit_upload_result_valid() {
        {
            // Has an object.
            let commit_upload_res = ulksys::UplinkCommitUploadResult {
                object: &mut ulksys::UplinkObject {
                    key: CString::new("key").unwrap().into_raw(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: ptr::null_mut(),
            };

            commit_upload_res.ensure();
        }

        {
            // Has an error.
            let commit_upload_res = ulksys::UplinkCommitUploadResult {
                object: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            commit_upload_res.ensure();
        }

        {
            // Has an object and an error.
            let commit_upload_res = ulksys::UplinkCommitUploadResult {
                object: &mut ulksys::UplinkObject {
                    key: CString::new("key").unwrap().into_raw(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            commit_upload_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkCommitUploadResult; object and error fields are both NULL"
    )]
    fn test_ensurer_commit_upload_result_invalid_both_null() {
        let commit_upload_res = ulksys::UplinkCommitUploadResult {
            object: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        commit_upload_res.ensure();
    }

    #[test]
    fn test_ensurer_download_result_valid() {
        {
            // Has a download.
            let download_res = ulksys::UplinkDownloadResult {
                download: &mut ulksys::UplinkDownload { _handle: 0 },
                error: ptr::null_mut(),
            };

            download_res.ensure();
        }

        {
            // Has an error.
            let download_res = ulksys::UplinkDownloadResult {
                download: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            download_res.ensure();
        }

        {
            // Has a download and an error.
            let download_res = ulksys::UplinkDownloadResult {
                download: &mut ulksys::UplinkDownload { _handle: 0 },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            download_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkDownloadResult; download and error fields are both NULL"
    )]
    fn test_ensurer_download_result_invalid_both_null() {
        let download_res = ulksys::UplinkDownloadResult {
            download: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        download_res.ensure();
    }

    #[test]
    fn test_ensurer_encryption_key_result_valid() {
        {
            // Has an encryption key.
            let enckey_res = ulksys::UplinkEncryptionKeyResult {
                encryption_key: &mut ulksys::UplinkEncryptionKey { _handle: 0 },
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            enckey_res.ensure();
        }

        {
            // Has an error.
            let enckey_res = ulksys::UplinkEncryptionKeyResult {
                encryption_key: ptr::null_mut::<ulksys::UplinkEncryptionKey>(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            enckey_res.ensure();
        }

        {
            // Has an encryption key and an error.
            let enckey_res = ulksys::UplinkEncryptionKeyResult {
                encryption_key: &mut ulksys::UplinkEncryptionKey { _handle: 0 },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            enckey_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkEncryptionKeyResult; encryption_key and error fields are both NULL"
    )]
    fn test_ensurer_encryption_key_result_invalid_both_null() {
        let enckey_res = ulksys::UplinkEncryptionKeyResult {
            encryption_key: ptr::null_mut::<ulksys::UplinkEncryptionKey>(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        enckey_res.ensure();
    }

    #[test]
    fn test_ensurer_object_valid() {
        let obj = ulksys::UplinkObject {
            key: CString::new("key").unwrap().into_raw(),
            is_prefix: false,
            system: ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 0,
                content_length: 0,
            },
            custom: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
        };
        obj.ensure();
    }

    #[test]
    #[should_panic(expected = "FFI returned an invalid UplinkObject; key field is NULL")]
    fn test_ensurer_object_invalid() {
        let obj = ulksys::UplinkObject {
            key: ptr::null_mut(),
            is_prefix: false,
            system: ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 0,
                content_length: 0,
            },
            custom: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
        };
        obj.ensure();
    }

    #[test]
    fn test_ensurer_object_result_valid() {
        {
            // Has an object.
            let obj_res = ulksys::UplinkObjectResult {
                object: &mut ulksys::UplinkObject {
                    key: CString::new("key").unwrap().into_raw(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: ptr::null_mut(),
            };

            obj_res.ensure();
        }

        {
            // Has an error.
            let obj_res = ulksys::UplinkObjectResult {
                object: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            obj_res.ensure();
        }

        {
            // Has an object and an error.
            let obj_res = ulksys::UplinkObjectResult {
                object: &mut ulksys::UplinkObject {
                    key: ptr::null_mut(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            obj_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkObjectResult; object and error fields are both NULL"
    )]
    fn test_ensurer_object_result_invalid_both_null() {
        let obj_res = ulksys::UplinkObjectResult {
            object: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        obj_res.ensure();
    }

    #[test]
    fn test_ensurer_part_result_valid() {
        {
            // Has a part.
            let upload_part_res = ulksys::UplinkPartResult {
                part: &mut ulksys::UplinkPart {
                    part_number: 0,
                    size: 0,
                    modified: 0,
                    etag: ptr::null_mut(),
                    etag_length: 0,
                },
                error: ptr::null_mut(),
            };

            upload_part_res.ensure();
        }

        {
            // Has an error.
            let upload_part_res = ulksys::UplinkPartResult {
                part: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_part_res.ensure();
        }

        {
            // Has a part and an error.
            let upload_res = ulksys::UplinkPartResult {
                part: &mut ulksys::UplinkPart {
                    part_number: 0,
                    size: 0,
                    modified: 0,
                    etag: ptr::null_mut(),
                    etag_length: 0,
                },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkPartResult; part and error fields are both NULL"
    )]
    fn test_ensurer_part_result_invalid_both_null() {
        let upload_res = ulksys::UplinkPartResult {
            part: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }

    #[test]
    fn test_ensurer_part_upload_result_valid() {
        {
            // Has a part upload.
            let upload_res = ulksys::UplinkPartUploadResult {
                part_upload: &mut ulksys::UplinkPartUpload { _handle: 0 },
                error: ptr::null_mut(),
            };

            upload_res.ensure();
        }

        {
            // Has an error.
            let upload_res = ulksys::UplinkPartUploadResult {
                part_upload: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_res.ensure();
        }

        {
            // Has a part upload and an error.
            let upload_res = ulksys::UplinkPartUploadResult {
                part_upload: &mut ulksys::UplinkPartUpload { _handle: 0 },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkPartUploadResult; part_upload and error fields are both NULL"
    )]
    fn test_ensurer_part_upload_result_invalid_both_null() {
        let pupload_res = ulksys::UplinkPartUploadResult {
            part_upload: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        pupload_res.ensure();
    }

    #[test]
    fn test_ensurer_string_result_valid() {
        {
            // Has a string.
            let str_res = ulksys::UplinkStringResult {
                string: CString::new("whatever").unwrap().into_raw(),
                error: ptr::null_mut::<ulksys::UplinkError>(),
            };

            str_res.ensure();
        }

        {
            // Has an error.
            let str_res = ulksys::UplinkStringResult {
                string: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            str_res.ensure();
        }

        {
            // Has a string and an error.
            let str_res = ulksys::UplinkStringResult {
                string: CString::new("whatever").unwrap().into_raw(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            str_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkStringResult; string and error fields are both NULL"
    )]
    fn test_ensurer_string_result_invalid_both_null() {
        let str_res = ulksys::UplinkStringResult {
            string: ptr::null_mut(),
            error: ptr::null_mut::<ulksys::UplinkError>(),
        };

        str_res.ensure();
    }

    #[test]
    fn test_ensurer_upload_info_valid() {
        let info = ulksys::UplinkUploadInfo {
            upload_id: CString::new("upload-id").unwrap().into_raw(),
            key: CString::new("key").unwrap().into_raw(),
            is_prefix: false,
            system: ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 0,
                content_length: 0,
            },
            custom: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
        };

        info.ensure();
    }

    #[test]
    #[should_panic(expected = "FFI returned an invalid UplinkUploadInfo; upload_id field is NULL")]
    fn test_ensurer_upload_info_null_id() {
        let info = ulksys::UplinkUploadInfo {
            upload_id: ptr::null_mut(),
            key: CString::new("key").unwrap().into_raw(),
            is_prefix: false,
            system: ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 0,
                content_length: 0,
            },
            custom: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
        };

        info.ensure();
    }

    #[test]
    #[should_panic(expected = "FFI returned an invalid UplinkUploadInfo; key field is NULL")]
    fn test_ensurer_upload_info_null_key() {
        let info = ulksys::UplinkUploadInfo {
            upload_id: CString::new("upload-id").unwrap().into_raw(),
            key: ptr::null_mut(),
            is_prefix: false,
            system: ulksys::UplinkSystemMetadata {
                created: 0,
                expires: 0,
                content_length: 0,
            },
            custom: ulksys::UplinkCustomMetadata {
                entries: ptr::null_mut(),
                count: 0,
            },
        };

        info.ensure();
    }

    #[test]
    fn test_ensurer_upload_info_result_valid() {
        {
            // Has an upload info.
            let upload_info_res = ulksys::UplinkUploadInfoResult {
                info: &mut ulksys::UplinkUploadInfo {
                    key: ptr::null_mut(),
                    upload_id: ptr::null_mut(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: ptr::null_mut(),
            };

            upload_info_res.ensure();
        }

        {
            // Has an error
            let upload_info_res = ulksys::UplinkUploadInfoResult {
                info: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_info_res.ensure();
        }

        {
            // Has an upload info and an error.
            let upload_info_res = ulksys::UplinkUploadInfoResult {
                info: &mut ulksys::UplinkUploadInfo {
                    key: CString::new("key").unwrap().into_raw(),
                    upload_id: CString::new("upload_id").unwrap().into_raw(),
                    is_prefix: false,
                    system: ulksys::UplinkSystemMetadata {
                        created: 0,
                        expires: 0,
                        content_length: 0,
                    },
                    custom: ulksys::UplinkCustomMetadata {
                        entries: ptr::null_mut(),
                        count: 0,
                    },
                },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_info_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkUploadInfoResult; info and error fields are both NULL"
    )]
    fn test_ensurer_upload_info_result_invalid_both_null() {
        let upload_info_res = ulksys::UplinkUploadInfoResult {
            info: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_info_res.ensure();
    }

    #[test]
    fn test_ensurer_upload_result_valid() {
        {
            // Has an upload.
            let upload_res = ulksys::UplinkUploadResult {
                upload: &mut ulksys::UplinkUpload { _handle: 0 },
                error: ptr::null_mut(),
            };

            upload_res.ensure();
        }

        {
            // Has an error
            let upload_res = ulksys::UplinkUploadResult {
                upload: ptr::null_mut(),
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_res.ensure();
        }

        {
            // Has an upload and an error.
            let upload_res = ulksys::UplinkUploadResult {
                upload: &mut ulksys::UplinkUpload { _handle: 0 },
                error: &mut ulksys::UplinkError {
                    code: 0,
                    message: ptr::null_mut(),
                },
            };

            upload_res.ensure();
        }
    }

    #[test]
    #[should_panic(
        expected = "FFI returned an invalid UplinkUploadResult; upload and error fields are both NULL"
    )]
    fn test_ensurer_upload_result_invalid_both_null() {
        let upload_res = ulksys::UplinkUploadResult {
            upload: ptr::null_mut(),
            error: ptr::null_mut(),
        };

        upload_res.ensure();
    }
}
