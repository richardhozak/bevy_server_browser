use std::{
    collections::HashMap,
    fmt::{self, Debug},
};

use bevy_utils::StableHashMap;
use mdns_sd::TxtProperties;

/// A string key-value map for additional server info to send to clients.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ServerMetadata(StableHashMap<String, String>);

impl ServerMetadata {
    pub(crate) fn from_txt_properties(props: &TxtProperties) -> ServerMetadata {
        let mut metadata = ServerMetadata::default();

        for property in props.iter() {
            metadata.set(property.key(), property.val_str());
        }

        metadata
    }

    pub(crate) fn into_hash_map(self) -> HashMap<String, String> {
        self.0.into_iter().collect()
    }

    /// Returns new empty server metadata.
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&str> {
        self.0.get(key.as_ref()).map(|v| v.as_str())
    }

    /// Sets the value of a key.
    pub fn set<K: AsRef<str>, V: ToString>(&mut self, key: K, value: V) {
        self.0.entry_ref(key.as_ref()).insert(value.to_string());
    }

    /// Sets the value of a key and returns self.
    /// This function is useful for chaining metadata creation:
    /// ```
    /// let metadata = ServerMetadata::new()
    ///     .with("key", "value")
    ///     .with("another_key", "another_value")
    ///     .with("answer", 42)
    /// ```
    pub fn with<K: AsRef<str>, V: ToString>(mut self, key: K, value: V) -> Self {
        self.set(key, value);
        self
    }

    /// Iterate over all key-value pairs stored in metadata.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl Debug for ServerMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(&self.0).finish()
    }
}

/// Iterator over server metadata in stable order.
pub struct ServerMetadataIter<'a> {
    inner: bevy_utils::hashbrown::hash_map::Iter<'a, String, String>,
}

impl<'a> Iterator for ServerMetadataIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some((k, v)) => Some((k.as_ref(), v.as_ref())),
            None => None,
        }
    }
}

impl<'a> IntoIterator for &'a ServerMetadata {
    type Item = (&'a str, &'a str);
    type IntoIter = ServerMetadataIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: (&self.0).into_iter(),
        }
    }
}
