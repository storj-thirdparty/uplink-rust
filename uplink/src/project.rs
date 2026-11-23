//! Storj DCS Project.

pub mod options;

use crate::access::Grant;
use crate::config::Config;
use crate::object::upload;
use crate::uplink_c::Ensurer;
use crate::{bucket, error, helpers, metadata, object, Bucket, Error, Object, Result};

use std::os::raw::c_char;
use std::ptr;

use uplink_sys as ulksys;

/// Provides access to manage buckets and objects.
pub struct Project {
    /// The project type of the FFI that an instance of this struct represents and guards its life
    /// time until this instance drops.
    ///
    /// It's a project result because it's the one that holds the project and allows to free its
    /// memory.
    inner: ulksys::UplinkProjectResult,
}

impl Project {
    /// Opens a project with the specified access grant.
    pub fn open(grant: &Grant) -> Self {
        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        // The `grant.as_ffi_access` return a pointer to its FFI representation that only lives as
        // long as `grant` but we don't need to take ownership of `grant` because the FFI access is
        // only a handler, not the actual access value, so `grant` can be dropped without affecting
        // the FFI project instance.
        let inner = unsafe { ulksys::uplink_open_project(grant.as_ffi_access()) };
        Self { inner }
    }

    /// Opens a project with the specified access grant and configuration.
    pub fn open_with_config(grant: Grant, config: &Config) -> Self {
        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let inner = unsafe {
            ulksys::uplink_config_open_project(config.as_ffi_config(), grant.as_ffi_access())
        };
        Self { inner }
    }

    /// Aborts a multipart upload started with [`Self::begin_upload`].
    ///
    /// The `upload_id` is an upload identifier that [`Self::begin_upload`] has returned.
    pub fn abort_upload(&self, bucket: &str, key: &str, upload_id: &str) -> Result<()> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;
        let c_upload_id = helpers::cstring_from_str_fn_arg("upload_id", upload_id)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_err = unsafe {
            ulksys::uplink_abort_upload(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_upload_id.as_ptr() as *mut c_char,
            )
        };

        if let Some(err) = Error::new_uplink(uc_err) {
            helpers::drop_uplink_sys_error(uc_err);
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Begins a new multipart upload to `bucket` and `key` with optional options.
    ///
    /// Use
    /// * [`Self::upload_part`] to upload individual parts.
    /// * [`Self::commit_upload`] to finish the upload.
    /// * [`Self::abort_upload`] to cancel the upload at any time.
    ///
    /// For uploading single parts objects use [`Self::upload_object`] because it's more
    /// convenient.
    pub fn begin_upload(
        &self,
        bucket: &str,
        key: &str,
        opts: Option<&options::Upload>,
    ) -> Result<upload::Info> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_upload_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_begin_upload(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        upload::Info::from_ffi_upload_info_result(uc_res)
    }

    /// Commits a multipart upload with `upload_id` to `bucket` and `key` with optional options.
    ///
    /// `opts` wraps a mutable reference because the [`options::CommitUpload`] requires a mutable
    /// reference to obtain its FFI representation.
    ///
    /// The `upload_id` is an upload identifier that [`Self::begin_upload`] has returned.
    pub fn commit_upload(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        opts: Option<&mut options::CommitUpload>,
    ) -> Result<Object> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;
        let c_upload_id = helpers::cstring_from_str_fn_arg("upload_id", upload_id)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.to_ffi_commit_upload_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_commit_upload(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_upload_id.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        Object::from_ffi_commit_upload_result(uc_res)
    }

    /// Atomically copies an object to a different bucket or/and key without downloading and
    /// uploading it.
    pub fn copy_object(
        &self,
        current_bucket: &str,
        current_key: &str,
        new_bucket: &str,
        new_key: &str,
        opts: Option<&options::CopyObject>,
    ) -> Result<Object> {
        let c_cur_bucket = helpers::cstring_from_str_fn_arg("current_bucket", current_bucket)?;
        let c_cur_key = helpers::cstring_from_str_fn_arg("current_key", current_key)?;
        let c_new_bucket = helpers::cstring_from_str_fn_arg("new_bucket", new_bucket)?;
        let c_new_key = helpers::cstring_from_str_fn_arg("new_key", new_key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_copy_object_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_copy_object(
                self.inner.project,
                c_cur_bucket.as_ptr() as *mut c_char,
                c_cur_key.as_ptr() as *mut c_char,
                c_new_bucket.as_ptr() as *mut c_char,
                c_new_key.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        Object::from_ffi_object_result(uc_res)
            .map(|op| op.expect("successful copying an object must always return an object"))
    }

    /// Creates a new bucket.
    ///
    /// It returns the bucket information and `true` when it's created or `false` if it already
    /// existed.
    ///
    /// It returns an [`crate::Error::Uplink`] error with
    /// [`crate::error::Uplink::BucketAlreadyExists`] variant if the bucket already existed but its
    /// information cannot be retrieved (e.g. the access grant doesn't have enough permissions).
    pub fn create_bucket(&self, bucket: &str) -> Result<(Bucket, bool)> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_create_bucket(self.inner.project, c_bucket.as_ptr() as *mut c_char)
        };

        Self::created_bucket_from_ffi_bucket_result(uc_res)
    }

    /// Returns the bucket and if it was created from the FFI result of creating a bucket.
    fn created_bucket_from_ffi_bucket_result(
        uc_res: ulksys::UplinkBucketResult,
    ) -> Result<(Bucket, bool)> {
        uc_res.ensure();

        let Some(err) = Error::new_uplink(uc_res.error) else {
            return Ok((Bucket::from_ffi_bucket(uc_res.bucket)?, true));
        };

        // When the bucket already exists, the FFI returns an error and also the bucket, unless that
        // it cannot retrieve it, for example, because the access grant doesn't have permissions to
        // read the bucket's information.
        let already_exists = matches!(&err, Error::Uplink(error::Uplink::BucketAlreadyExists(_)));
        if !already_exists || uc_res.bucket.is_null() {
            // SAFETY: the `Error` constructor doesn't take ownership of the FFI error pointer
            // so it's still allocated at this point and we trust the FFI of freeing memory of
            // pointers allocated by itself.
            unsafe { ulksys::uplink_free_bucket_result(uc_res) };
            return Err(err);
        }

        // `Bucket::from_ffi_bucket` only takes ownership of the `bucket` field.
        helpers::drop_uplink_sys_error(uc_res.error);
        Ok((Bucket::from_ffi_bucket(uc_res.bucket)?, false))
    }

    /// Deletes a bucket.
    ///
    /// It returns an [`crate::Error::Uplink`] error with [`crate::error::Uplink::BucketNotEmpty`]
    /// variant if `bucket` isn't empty.
    pub fn delete_bucket(&self, bucket: &str) -> Result<Bucket> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_delete_bucket(self.inner.project, c_bucket.as_ptr() as *mut c_char)
        };

        Bucket::from_ffi_bucket_result(uc_res)
    }

    /// Deletes a bucket and all its objects.
    pub fn delete_bucket_with_objects(&self, bucket: &str) -> Result<Bucket> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_delete_bucket_with_objects(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
            )
        };

        Bucket::from_ffi_bucket_result(uc_res)
    }

    /// Deletes the object inside of `bucket` and referenced with `key`.
    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<Option<Object>> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_delete_object(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
            )
        };

        Object::from_ffi_object_result(uc_res)
    }

    /// Starts a download of the object inside of `bucket` and referenced with `key` with optional
    /// options.
    pub fn download_object(
        &self,
        bucket: &str,
        key: &str,
        opts: Option<&options::Download>,
    ) -> Result<object::Download> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_download_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_download_object(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        object::Download::from_ffi_download_result(uc_res)
    }

    /// Returns the bucket if it exists otherwise it creates it.
    pub fn ensure_bucket(&self, bucket: &str) -> Result<Bucket> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_ensure_bucket(self.inner.project, c_bucket.as_ptr() as *mut c_char)
        };

        Bucket::from_ffi_bucket_result(uc_res)
    }

    /// Returns an iterator over the list of existing buckets with optional options.
    pub fn list_buckets(&self, opts: Option<&options::ListBuckets>) -> bucket::Iterator {
        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_it = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_list_buckets_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            };

            ulksys::uplink_list_buckets(self.inner.project, c_opts)
        };

        bucket::Iterator::from_ffi_bucket_iterator(uc_it)
    }

    /// Returns an iterator over the list of existing object inside of `bucket` with optional
    /// options.
    pub fn list_objects(
        &self,
        bucket: &str,
        opts: Option<&options::ListObjects>,
    ) -> Result<object::Iterator> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_it = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_list_objects_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            };

            ulksys::uplink_list_objects(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        Ok(object::Iterator::from_ffi_object_iterator(uc_it))
    }

    /// Returns an iterator over the parts of a multipart upload started with [`Self::begin_upload`]
    /// with optional options.
    pub fn list_upload_parts(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        opts: Option<&options::ListUploadParts>,
    ) -> Result<upload::PartIterator> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;
        let c_upload_id = helpers::cstring_from_str_fn_arg("upload_id", upload_id)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_it = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_list_upload_parts_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            };

            ulksys::uplink_list_upload_parts(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_upload_id.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        Ok(upload::PartIterator::from_ffi_part_iterator(uc_it))
    }

    /// Returns an iterator over the uncommitted uploads in `bucket` with optional options.
    pub fn list_uploads(
        &self,
        bucket: &str,
        opts: Option<&options::ListUploads>,
    ) -> Result<upload::Iterator> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_it = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_list_uploads_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_list_uploads(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        Ok(upload::Iterator::from_ffi_upload_iterator(uc_it))
    }

    /// Moves an object to a different bucket or/and key with optional options.
    pub fn move_object(
        &self,
        current_bucket: &str,
        current_key: &str,
        new_bucket: &str,
        new_key: &str,
        opts: Option<&options::MoveObject>,
    ) -> Result<()> {
        let c_cur_bucket = helpers::cstring_from_str_fn_arg("current_bucket", current_bucket)?;
        let c_cur_key = helpers::cstring_from_str_fn_arg("current_key", current_key)?;
        let c_new_bucket = helpers::cstring_from_str_fn_arg("new_bucket", new_bucket)?;
        let c_new_key = helpers::cstring_from_str_fn_arg("new_key", new_key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_err = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_move_object_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_move_object(
                self.inner.project,
                c_cur_bucket.as_ptr() as *mut c_char,
                c_cur_key.as_ptr() as *mut c_char,
                c_new_bucket.as_ptr() as *mut c_char,
                c_new_key.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        if let Some(err) = Error::from_ffi_error(uc_err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Revokes the API key embedded in `access`.
    ///
    /// When an access grant is revoked, the rest of the further-restricted access grants (via the
    /// [`crate::access:Grant.share`]) are revoked.
    ///
    /// An access grant is authorized to revoke any of its further-restricted access grants. It
    /// cannot revoke itself. Revoking an access grant which is not one of its further-restricted
    /// access grants will return an error.
    ///
    /// A successful revocation request may not actually apply the revocation immediately because
    /// of the satellite's access caching policies.
    pub fn revoke_access(&self, access: &Grant) -> Result<()> {
        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_err =
            unsafe { ulksys::uplink_revoke_access(self.inner.project, access.as_ffi_access()) };

        if let Some(err) = Error::from_ffi_error(uc_err) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Returns the bucket's information.
    pub fn stat_bucket(&self, bucket: &str) -> Result<Bucket> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_stat_bucket(self.inner.project, c_bucket.as_ptr() as *mut c_char)
        };

        Bucket::from_ffi_bucket_result(uc_res)
    }

    /// Returns the object's information inside of `bucket` and reference by `key`.
    pub fn stat_object(&self, bucket: &str, key: &str) -> Result<Object> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_stat_object(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
            )
        };

        Object::from_ffi_object_result(uc_res)
            .map(|op| op.expect("successful stat object must always return an object"))
    }

    /// Starts an object upload into `bucket` with the specified `key` and optional options.
    pub fn upload_object(
        &self,
        bucket: &str,
        key: &str,
        opts: Option<&options::Upload>,
    ) -> Result<object::Upload> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_upload_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_upload_object(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_opts,
            )
        };

        object::Upload::from_ffi_upload_result(uc_res)
    }

    /// Uploads a part with `part_number` to a multipart upload started with
    /// [`Self::begin_upload`]. `upload_id` is an identifier returned by [`Self::begin_upload`].
    pub fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        part_number: u32,
    ) -> Result<upload::PartUpload> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;
        let c_upload_id = helpers::cstring_from_str_fn_arg("upload_id", upload_id)?;

        // SAFETY: we trust the FFI is behaving correctly when called with correct value.
        let uc_res = unsafe {
            ulksys::uplink_upload_part(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                c_upload_id.as_ptr() as *mut c_char,
                part_number,
            )
        };

        upload::PartUpload::from_ffi_part_upload_result(uc_res)
    }

    /// Replaces the custom metadata for the object inside of `bucket` and referenced by `key` with
    /// the new specified metadata and with optional options. Any existing custom metadata is
    /// deleted.
    ///
    /// `metadata` is mutable because converting to a Uplink-C representation requires it.
    pub fn update_object_metadata(
        &self,
        bucket: &str,
        key: &str,
        metadata: &mut metadata::Custom,
        opts: Option<&options::UploadObjectMetadata>,
    ) -> Result<()> {
        let c_bucket = helpers::cstring_from_str_fn_arg("bucket", bucket)?;
        let c_key = helpers::cstring_from_str_fn_arg("key", key)?;

        // SAFETY: we get the FFI representation of the opts if it isn't `None` then we get a
        // mutable reference to it but we use the reference only inside of the scope, hence we are
        // always referencing it during its lifetime that the scope establishes.
        // For the rest, we trust the FFI is behaving correctly when called with correct value.
        let uc_err = unsafe {
            let mut c_opts = ptr::null_mut();
            let mut uc_opts;
            if let Some(o) = opts {
                uc_opts = o.as_ffi_upload_object_metadata_options();
                c_opts = ptr::addr_of_mut!(uc_opts);
            }

            ulksys::uplink_update_object_metadata(
                self.inner.project,
                c_bucket.as_ptr() as *mut c_char,
                c_key.as_ptr() as *mut c_char,
                metadata.to_ffi_custom_metadata(),
                c_opts,
            )
        };

        if let Some(err) = Error::from_ffi_error(uc_err) {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        // SAFETY: we trust that the FFI is doing correct operations when closing and freeing a
        // correctly created `UplinkProjectResult` value.
        unsafe {
            // At this point we cannot do anything about the error, so discarded.
            // TODO(https://github.com/storj-thirdparty/uplink-rust/issues/51).
            let _ = ulksys::uplink_close_project(self.inner.project);
            ulksys::uplink_free_project_result(self.inner);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // The FFI frees its values with the C allocator, so the values that the tests create must be
    // allocated with it.
    extern "C" {
        fn calloc(nitems: usize, size: usize) -> *mut std::ffi::c_void;
        fn strdup(s: *const c_char) -> *mut c_char;
    }

    /// Returns a pointer to a copy of `val` allocated with the C allocator.
    fn c_box<T>(val: T) -> *mut T {
        // SAFETY: there are no preconditions to call this C function and the returned pointer, when
        // it isn't `NULL`, has enough room for the value.
        unsafe {
            let ptr = calloc(1, std::mem::size_of::<T>()).cast::<T>();
            assert!(!ptr.is_null(), "C allocation failed");
            ptr.write(val);
            ptr
        }
    }

    /// Returns a copy of `val` allocated with the C allocator.
    fn c_string(val: &std::ffi::CStr) -> *mut c_char {
        // SAFETY: `val` is a valid C string.
        let ptr = unsafe { strdup(val.as_ptr()) };
        assert!(!ptr.is_null(), "C allocation failed");
        ptr
    }

    fn ffi_bucket() -> *mut ulksys::UplinkBucket {
        c_box(ulksys::UplinkBucket {
            name: c_string(c"a-bucket"),
            created: 10,
        })
    }

    fn ffi_error(code: u32) -> *mut ulksys::UplinkError {
        c_box(ulksys::UplinkError {
            code: code as i32,
            message: c_string(c"a message"),
        })
    }

    #[test]
    fn test_created_bucket_from_ffi_bucket_result() {
        {
            // The bucket is created.
            let (bucket, created) =
                Project::created_bucket_from_ffi_bucket_result(ulksys::UplinkBucketResult {
                    bucket: ffi_bucket(),
                    error: ptr::null_mut(),
                })
                .expect("bucket created");
            assert_eq!(bucket.name, "a-bucket", "bucket name");
            assert!(created, "created");
        }
        {
            // The bucket already exists.
            let (bucket, created) =
                Project::created_bucket_from_ffi_bucket_result(ulksys::UplinkBucketResult {
                    bucket: ffi_bucket(),
                    error: ffi_error(ulksys::UPLINK_ERROR_BUCKET_ALREADY_EXISTS),
                })
                .expect("bucket already exists");
            assert_eq!(bucket.name, "a-bucket", "bucket name");
            assert!(!created, "created");
        }
        {
            // The bucket already exists but its information cannot be retrieved.
            let err = Project::created_bucket_from_ffi_bucket_result(ulksys::UplinkBucketResult {
                bucket: ptr::null_mut(),
                error: ffi_error(ulksys::UPLINK_ERROR_BUCKET_ALREADY_EXISTS),
            })
            .expect_err("bucket already exists without information");
            assert!(
                matches!(err, Error::Uplink(error::Uplink::BucketAlreadyExists(_))),
                "unexpected error: {err:?}"
            );
        }
        {
            // Other error.
            let err = Project::created_bucket_from_ffi_bucket_result(ulksys::UplinkBucketResult {
                bucket: ptr::null_mut(),
                error: ffi_error(ulksys::UPLINK_ERROR_PERMISSION_DENIED),
            })
            .expect_err("permission denied");
            assert!(
                matches!(err, Error::Uplink(error::Uplink::PermissionDenied(_))),
                "unexpected error: {err:?}"
            );
        }
    }
}
