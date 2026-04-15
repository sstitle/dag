use slotmap::{DefaultKey, Key, KeyData};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Opaque identifier for a node; backed by a slotmap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(u64);

/// Opaque identifier for an edge; backed by a slotmap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeId(u64);

impl NodeId {
    /// Returns the raw `u64` encoding of this ID.
    ///
    /// The encoding is an implementation detail (slotmap FFI key) and may
    /// change across versions. Exposed only for language-binding layers.
    pub fn raw(self) -> u64 {
        self.0
    }

    pub(crate) fn key(self) -> DefaultKey {
        DefaultKey::from(KeyData::from_ffi(self.0))
    }
}

impl From<DefaultKey> for NodeId {
    fn from(k: DefaultKey) -> Self {
        NodeId(k.data().as_ffi())
    }
}

#[cfg(feature = "raw-id-access")]
impl NodeId {
    /// Constructs a `NodeId` from its raw `u64` encoding.
    ///
    /// Intended exclusively for language-binding layers (e.g. the Node.js
    /// binding that round-trips IDs through JavaScript `number`). Enable the
    /// `raw-id-access` crate feature.
    ///
    /// **Safety:** Only pass values previously returned by [`NodeId::raw`] for
    /// IDs that still belong to the same graph. Arbitrary `u64` values can
    /// refer to empty slots or the wrong entity; internal slot-map indexing may
    /// **panic** on some invalid encodings, and API methods typically return
    /// [`crate::DagError::NodeNotFound`] for keys that do not resolve to a live node.
    #[doc(hidden)]
    pub fn from_raw(v: u64) -> Self {
        NodeId(v)
    }
}

impl EdgeId {
    /// Returns the raw `u64` encoding of this ID.
    pub fn raw(self) -> u64 {
        self.0
    }

    pub(crate) fn key(self) -> DefaultKey {
        DefaultKey::from(KeyData::from_ffi(self.0))
    }
}

impl From<DefaultKey> for EdgeId {
    fn from(k: DefaultKey) -> Self {
        EdgeId(k.data().as_ffi())
    }
}

#[cfg(feature = "raw-id-access")]
impl EdgeId {
    /// Constructs an `EdgeId` from its raw `u64` encoding.
    ///
    /// Same caveats as [`NodeId::from_raw`].
    ///
    /// **Safety:** See [`NodeId::from_raw`]; invalid keys can cause panics when
    /// indexing internal storage or yield [`crate::DagError::EdgeNotFound`].
    #[doc(hidden)]
    pub fn from_raw(v: u64) -> Self {
        EdgeId(v)
    }
}
