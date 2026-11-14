//! Who a leaf of the team's MLS group is: a member, one of their devices, or the team's recovery key.
//!
//! MLS leaves the authentication service to the application (RFC 9420 §5.3). Here a leaf's `BasicCredential`
//! carries an [`Identity`] (37 bytes), and the team's roster (`roster.rs`, in the group context) says which members
//! exist and what they may do. The device's signature key is the leaf's own: the identity names, the key proves.

use std::fmt;

use openmls::prelude::{BasicCredential, Credential};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error;

const MAGIC: &[u8; 4] = b"esc1";

macro_rules! id16 {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub [u8; 16]);

        impl $name {
            /// A fresh one, from the provider's randomness.
            pub fn random(rand: &impl openmls_traits::random::OpenMlsRand) -> Result<Self, Error> {
                Ok(Self(rand.random_array().map_err(|_| Error::Random)?))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), hex::encode(&self.0[..4]))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&hex::encode(self.0))
            }
        }

        // As a hex string, so that it is a key of a JSON map (the roster's members).
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&hex::encode(self.0))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = String::deserialize(d)?;
                let mut out = [0u8; 16];
                hex::decode_to_slice(&text, &mut out).map_err(serde::de::Error::custom)?;
                Ok(Self(out))
            }
        }
    };
}

id16!(
    MemberId,
    "A member of the team (a person, or a machine in server mode), the same on all their devices."
);
id16!(DeviceId, "One device of a member.");
id16!(
    ProjectId,
    "A project of the team: an opaque id, its name is in the encrypted content."
);
id16!(
    ObjectId,
    "An object of a project: a ticket, a spec page, a memory."
);

/// What a leaf stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    /// A device of a member, its keys made on it.
    Device,
    /// The team's recovery key (24 words): an administrator device, offline, that joins only to recover the team
    /// (`recovery.rs`).
    Recovery,
}

/// A leaf's identity, in its credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub member: MemberId,
    pub device: DeviceId,
    pub kind: DeviceKind,
}

impl Identity {
    /// `esc1`, the kind (0 a device, 1 the recovery key), the member, the device.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(37);
        out.extend_from_slice(MAGIC);
        out.push(match self.kind {
            DeviceKind::Device => 0,
            DeviceKind::Recovery => 1,
        });
        out.extend_from_slice(&self.member.0);
        out.extend_from_slice(&self.device.0);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 37 || &bytes[..4] != MAGIC {
            return Err(Error::Encoding("identity".into()));
        }
        let kind = match bytes[4] {
            0 => DeviceKind::Device,
            1 => DeviceKind::Recovery,
            _ => return Err(Error::Encoding("identity kind".into())),
        };
        let mut member = [0u8; 16];
        member.copy_from_slice(&bytes[5..21]);
        let mut device = [0u8; 16];
        device.copy_from_slice(&bytes[21..37]);
        Ok(Self {
            member: MemberId(member),
            device: DeviceId(device),
            kind,
        })
    }

    pub fn credential(&self) -> Credential {
        BasicCredential::new(self.to_bytes()).into()
    }

    /// The identity a leaf's credential carries; any other credential is refused.
    pub fn from_credential(credential: &Credential) -> Result<Self, Error> {
        let basic = BasicCredential::try_from(credential.clone())
            .map_err(|_| Error::Encoding("credential type".into()))?;
        Self::from_bytes(basic.identity())
    }
}
