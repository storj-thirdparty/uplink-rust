//! # Storj
//! This document contains some general concepts regarding to the Storj Decentralized Object
//! Storage.
//!
//! Uplink is the main entrypoint to interacting with Storj Labs' decentralized storage network.
//!
//! Sign up for an account on a Satellite today! <https://storj.io/>
//!
//! NOTE we are using `unwrap` for panicking on any error for avoiding verbosity and show the how to
//! perform certain operations in the network with this crate. Don't use `unwrap` it if you're
//! implementing something serious.
//!
//! ## Access Grants
//!
//! The fundamental unit of access in the Storj Labs storage network is the Access Grant.
//! An access grant is a serialized structure that is internally comprised of an API Key, a set of
//! encryption key information, and information about which Storj Labs or Tardigrade network
//! Satellite is responsible for the metadata. An access grant is always associated with exactly one
//! Project on one Satellite.
//!
//! If you don't already have an access grant, you will need make an account on a Satellite,
//! generate an API Key, and encapsulate that API Key with encryption information into an access
//! grant.
//!
//! If you don't already have an account on a Satellite, first make one at <https://storj.io/> and
//! note the Satellite you choose (such as us1.storj.io, eu1.storj.io, etc). Then, make an API Key
//! in the web interface.
//!
//! The first step to any project is to generate a restricted access grant with the minimal
//! permissions that are needed. Access grants contains all encryption information and they should
//! be restricted as much as possible.
//!
//! To make an access grant, you can create one using our Uplink CLI tool's 'share' sub-command
//! (after setting up the Uplink CLI tool), or you can make one as follows:
//!
//! ```ignore
//! use std::vec::Vec;
//!
//! use uplink::access::{Grant, Permission, SharePrefix};
//!
//! let grant = Grant::request_access_with_passphrase(
//!     satellite_address,
//!     api_key,
//!     root_passphrase,
//! ).unwrap();
//!
//!// Create an access grant for reading bucket "logs".
//! let permission = Permission::read_only();
//! let shared = SharePrefix::full_bucket("logs").unwrap();
//! let restricted_access = grant.share(&permission, Some(vec![shared])).unwrap();
//!
//! // Serialize the restricted access grant.
//! let serialized_access = restricted_access.serialize().unwrap();
//! ```
//!
//! In the above example, `serialized_access` is a human-readable string that represents read-only
//! access to just the "logs" bucket, and is only able to decrypt that one bucket thanks to
//! hierarchical deterministic key derivation.
//!
//! Note:
//! [`Grant::request_access_with_passphrase`](crate::access::Grant::request_access_with_passphrase)
//! is CPU-intensive, and your application's normal life-cycle should avoid it and use
//! [`Grant::new`](crate::access::Grant::new) where possible instead, which takes a serialized
//! access as input.
//!
//! To revoke an access grant see the
//! [`project::Project.revoke_access`](crate::project::Project::revoke_access) method.
//!
//! ## Multitenancy in a Single Application Bucket
//!
//! A common architecture for building applications is to have a single bucket for the entire
//! application to store the objects of all users. In such architecture, it is of utmost importance
//! to guarantee that users can access only their objects but not the objects of other users.
//!
//! This can be achieved by implementing an _app-specific_ authentication service that generates an
//! access grant for each user by restricting the main access grant of the application. This
//! _user-specific_ access grant is restricted to access the objects only within a specific key
//! prefix defined for the user.
//!
//! When initialized, the authentication server creates the main application access
//! grant with an empty passphrase as follows.
//!
//! ```ignore
//! use uplink::access::Grant;
//!
//! let app_grant = Grant::request_access_with_passphrase(
//!     satellite_address,
//!     app_api_key,
//!     "",
//! ).unwrap();
//! ````
//!
//! The authentication service does not hold any encryption information about users, so the
//! passphrase used to request the main application access grant does not matter.
//!
//! The encryption keys related to user objects will be overridden in a next step on the
//! client-side. It is important that once set to a specific value, this passphrase never changes in
//! the future. Therefore, the best practice is to use an empty passphrase.
//!
//! Whenever a user is authenticated, the authentication service generates the _user-specific_
//! access grant as follows:
//!
//! ```ignore
//! use std::time::{SystemTime, Duration};
//! use std::vec::Vec;
//!
//! use uplink::access::{Grant, Permission, SharePrefix};
//!
//! // Create a user access grant for accessing their files, limited for the next 8 hours.
//! let now = SystemTime::now();
//! let permissions =  Permission::full();
//!
//! // 2 minutes leeway to avoid time sync issues with the satellite.
//! let not_before = now.checked_sub(Duration::new(2 * 60, 0)).unwrap();
//! permissions.set_not_before(
//!     Some(
//!         not_before.duration_since(SystemTime::UNIX_EPOCH).unwrap(),
//!     ),
//! ).unwrap();
//!
//! // Up to 8 hours expressed in seconds.
//! let not_after = now.checked_add(Duration::new(8 * 60 * 60, 0)).unwrap();
//! permissions.set_not_after(
//!     Some(
//!         not_after.duration_since(SystemTime::UNIX_EPOCH).unwrap(),
//!     ),
//! ).unwrap();
//!
//! let user_prefix = SharePrefix::new(app_bucket, &format!("{user_id}/")).unwrap();
//! let user_grant = app_grant.share(&permissions, Some(vec![user_prefix])).unwrap();
//!
//! // Serialize the users's access grant.
//! let serialized_access = user_grant.serialize().unwrap();
//! ```
//!
//! The `user_id` is something that uniquely identifies the users in the application and must never
//! change.
//!
//! Along with the user access grant, the authentication service should return a _user-specific_
//! salt. The salt must be always the same for this user. The salt size is 16-byte or 32-byte.
//!
//! Once the application receives the _user-specific_ access grant and the _user-specific_ salt
//! from the authentication service, it has to override the encryption key in the access grant, so
//! users can encrypt and decrypt their files with encryption keys derived from their passphrase.
//!
//! ```ignore
//! use uplink::access::Grant;
//! use uplink::EncryptionKey;
//!
//! let user_grant  = Grant::new(serialized_user_access).unwrap();
//! let salted_user_key = EncryptionKey::derive(user_passphrase, user_salt).unwrap();
//! user_grant.override_encryption_key(app_bucket, &format!("{user_id}/"),
//! &salted_user_key).unwrap();
//!
//! ```
//!
//! The _user-specific_ access grant is now ready to use by the application.
//!
//! ## Projects
//!
//! Once you have a valid access grant, you can open a Project with the access that
//! access grant allows for.
//!
//! ```ignore
//! use uplink::Project;
//!
//! let project = Project::open(&access);
//! ```
//!
//! Projects allow you to manage buckets and objects within buckets.
//!
//! ## Buckets
//!
//! A bucket represents a collection of objects. You can upload, download, list, and delete objects
//! of any size or shape. Objects within buckets are represented by keys, where keys can optionally
//! be listed using the "/" delimiter.
//!
//! Note: Objects and object keys within buckets are end-to-end encrypted, but bucket names
//! themselves are not encrypted, so the billing interface on the Satellite can show you bucket line
//! items.
//!
//! ```ignore
//! let mut buckets = project.list_buckets(None);
//!
//! for res in buckets {
//!      match res {
//!          Ok(bucket) => println!("{}", bucket.name),
//!          Err(e) => {
//!              println!("Error: {e}");
//!              break;
//!          }
//!      }
//! }
//! ```
//!
//! ## Download Object
//!
//! Objects support a couple kilobytes of arbitrary key/value metadata, and arbitrary-size primary
//! data streams with the ability to read at arbitrary offsets.
//!
//! ```ignore
//! use std::io;
//! use std::vec::Vec;
//!
//! let mut download = project.download_object(
//!     "logs",
//!     "2020-04-18/webserver.log",
//!     None,
//! ).unwrap();
//!
//! // The returned type implement the std::io:Read trait, so you can use
//! // any method which is convenient for you to read the object's data.
//! // Let's use io::copy to assimilate the Go example
//! let mut writer: Vec<u8> = vec![];
//! io::copy(&mut download, &mut writer).unwrap();
//! ```
//!
//! If you want to access only a small sub-range of the data you uploaded, you can use
//! `project::options::Download` to specify the download range.
//!
//! ```ignore
//! use std::io;
//! use std::vec::Vec;
//!
//! use uplink::project::options;
//!
//! let mut download = project.download_object(
//!     "logs",
//!     "2020-04-18/webserver.log",
//!     &options::Download{offset: 10, length: 10},
//! ).unwrap();
//!
//! let mut writer: Vec<u8> = vec![];
//! io::copy(&mut download, &mut writer).unwrap();
//! ```
//!
//! ## List Objects
//!
//! Listing objects returns an iterator that allows to walk through all the items:
//!
//! ```ignore
//! let mut objects = project.list_objects("logs", None).unwrap();
//! for res in objects {
//!     match res {
//!         Ok(o) => println!("{} {}", o.is_prefix, o.key),
//!         Err(e) => {
//!             println!("Error: {e}");
//!             break;
//!         }
//!     }
//! }
//! ```
//!
//! ## More
//!
//! You can find how to use other parts of the API in the integration tests, visit or clone the
//! repository and look at the `tests` directory of this crate.
