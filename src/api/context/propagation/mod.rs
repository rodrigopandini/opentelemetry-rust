//! # OpenTelemetry Propagator interface
//!
//! Propagators API consists of two main formats:
//!
//! - `BinaryFormat` is used to serialize and deserialize a value
//! into a binary representation.
//! - `TextMapFormat` is used to inject and extract a value as
//! text into injectors and extractors that travel in-band across process boundaries.
//!
//! Deserializing must set `is_remote` to true on the returned
//! `SpanContext`.
//!
//! ## Binary Format
//!
//! `BinaryFormat` is a formatter to serialize and deserialize a value
//! into a binary format.
//!
//! `BinaryFormat` MUST expose the APIs that serializes values into bytes,
//! and deserializes values from bytes.
//!
//! ### ToBytes
//!
//! Serializes the given value into the on-the-wire representation.
//!
//! Required arguments:
//!
//! - the value to serialize, can be `SpanContext` or `DistributedContext`.
//!
//! Returns the on-the-wire byte representation of the value.
//!
//! ### FromBytes
//!
//! Creates a value from the given on-the-wire encoded representation.
//!
//! If the value could not be parsed, the underlying implementation
//! SHOULD decide to return ether an empty value, an invalid value, or
//! a valid value.
//!
//! Required arguments:
//!
//! - on-the-wire byte representation of the value.
//!
//! Returns a value deserialized from bytes.
//!
//! ## TextMap Format
//!
//! `TextMapFormat` is a formatter that injects and extracts a value
//! as text into injectors and extractors that travel in-band across process boundaries.
//!
//! Encoding is expected to conform to the HTTP Header Field semantics.
//! Values are often encoded as RPC/HTTP request headers.
//!
//! The carrier of propagated data on both the client (injector) and
//! server (extractor) side is usually a http request. Propagation is
//! usually implemented via library-specific request interceptors, where
//! the client-side injects values and the server-side extracts them.
//!
//! `TextMapFormat` MUST expose the APIs that injects values into injectors,
//! and extracts values from extractors.
//!
//! ### Fields
//!
//! The propagation fields defined. If your injector is reused, you should
//! delete the fields here before calling `inject`.
//!
//! For example, if the injector is a single-use or immutable request object,
//! you don't need to clear fields as they couldn't have been set before.
//! If it is a mutable, retryable object, successive calls should clear
//! these fields first.
//!
//! The use cases of this are:
//!
//! - allow pre-allocation of fields, especially in systems like gRPC
//! Metadata
//! - allow a single-pass over an iterator
//!
//! Returns list of fields that will be used by this formatter.
//!
//! ### Inject
//!
//! Injects the value downstream. For example, as http headers.
//!
//! Required arguments:
//!
//! - the `SpanContext` to be injected.
//! - the injector that holds propagation fields. For example, an outgoing
//! message or http request.
//! - the `Setter` invoked for each propagation key to add or remove.
//!
//! #### Setter argument
//!
//! Setter is an argument in `Inject` that puts value into given field.
//!
//! `Setter` allows a `TextMapFormat` to set propagated fields into a
//! injector.
//!
//! `Setter` MUST be stateless and allowed to be saved as a constant to
//! avoid runtime allocations. One of the ways to implement it is `Setter`
//! class with `Put` method as described below.
//!
//! ##### Put
//!
//! Replaces a propagated field with the given value.
//!
//! Required arguments:
//!
//! - the injector holds propagation fields. For example, an outgoing message
//! or http request.
//! - the key of the field.
//! - the value of the field.
//!
//! The implementation SHOULD preserve casing (e.g. it should not transform
//! `Content-Type` to `content-type`) if the used protocol is case insensitive,
//! otherwise it MUST preserve casing.
//!
//! ### Extract
//!
//! Extracts the value from upstream. For example, as http headers.
//!
//! If the value could not be parsed, the underlying implementation will
//! decide to return an object representing either an empty value, an invalid
//! value, or a valid value.
//!
//! Required arguments:
//!
//! - the extractor holds propagation fields. For example, an outgoing message
//! or http request.
//! - the instance of `Getter` invoked for each propagation key to get.
//!
//! Returns the non-null extracted value.
//!
//! #### Getter argument
//!
//! Getter is an argument in `Extract` that get value from given field
//!
//! `Getter` allows a `TextMapFormat` to read propagated fields from a
//! extractor.
//!
//! `Getter` MUST be stateless and allowed to be saved as a constant to avoid
//! runtime allocations. One of the ways to implement it is `Getter` class
//! with `Get` method as described below.
//!
//! ##### Get
//!
//!  The Get function MUST return the first value of the given propagation
//! key or return `None` if the key doesn't exist.
//!
//! Required arguments:
//!
//! - the extractor of propagation fields, such as an HTTP request.
//! - the key of the field.
//!
//! The `get` function is responsible for handling case sensitivity. If
//! the getter is intended to work with an HTTP request object, the getter
//! MUST be case insensitive. To improve compatibility with other text-based
//! protocols, text format implementations MUST ensure to always use the
//! canonical casing for their attributes. NOTE: Canonical casing for HTTP
//! headers is usually title case (e.g. `Content-Type` instead of `content-type`).
//!
use crate::api;
use std::collections::HashMap;

pub mod composite_propagator;
pub mod text_propagator;

/// Injector provides an interface for adding fields from an underlying struct like `HashMap`
pub trait Injector {
    /// Add a key and value to the underlying.
    fn set(&mut self, key: &str, value: String);
}

/// Extractor provides an interface for removing fields from an underlying struct like `HashMap`
pub trait Extractor {
    /// Get a value from a key from the underlying data.
    fn get(&self, key: &str) -> Option<&str>;
}

impl<S: std::hash::BuildHasher> api::Injector for HashMap<String, String, S> {
    /// Set a key and value in the HashMap.
    fn set(&mut self, key: &str, value: String) {
        self.insert(key.to_lowercase(), value);
    }
}

impl<S: std::hash::BuildHasher> api::Extractor for HashMap<String, String, S> {
    /// Get a value for a key from the HashMap.
    fn get(&self, key: &str) -> Option<&str> {
        self.get(&key.to_lowercase()).map(|v| v.as_str())
    }
}

#[cfg(feature = "http")]
impl api::Injector for http::HeaderMap {
    /// Set a key and value in the HeaderMap.  Does nothing if the key or value are not valid inputs.
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = http::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = http::header::HeaderValue::from_str(&value) {
                self.insert(name, val);
            }
        }
    }
}

#[cfg(feature = "http")]
impl api::Extractor for http::HeaderMap {
    /// Get a value for a key from the HeaderMap.  If the value is not valid ASCII, returns None.
    fn get(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|value| value.to_str().ok())
    }
}

#[cfg(feature = "tonic")]
impl api::Injector for tonic::metadata::MetadataMap {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::from_str(&value) {
                self.insert(key, val);
            }
        }
    }
}

#[cfg(feature = "tonic")]
impl api::Extractor for tonic::metadata::MetadataMap {
    /// Get a value for a key from the MetadataMap.  If the value can't be converted to &str, returns None
    fn get(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|metadata| metadata.to_str().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn hash_map() {
        let mut carrier = HashMap::new();
        carrier.set("headerName", "value".to_string());

        assert_eq!(
            Extractor::get(&carrier, "HEADERNAME"),
            Some("value"),
            "case insensitive extraction"
        )
    }

    #[test]
    #[cfg(feature = "http")]
    fn http_headers() {
        let mut carrier = http::HeaderMap::new();
        carrier.set("headerName", "value".to_string());

        assert_eq!(
            Extractor::get(&carrier, "HEADERNAME"),
            Some("value"),
            "case insensitive extraction"
        )
    }

    #[test]
    #[cfg(feature = "tonic")]
    fn tonic_headers() {
        let mut carrier = tonic::metadata::MetadataMap::new();
        carrier.set("headerName", "value".to_string());

        assert_eq!(
            Extractor::get(&carrier, "HEADERNAME"),
            Some("value"),
            "case insensitive extraction"
        )
    }
}
