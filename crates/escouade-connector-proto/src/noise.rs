//! The authentication inside the WebSocket (§ 4.2, § 4.5): `Noise_IK_25519_ChaChaPoly_BLAKE2s`, without I/O.
//!
//! The connector is the initiator: it knows the server's static key from the pairing, and its first message carries
//! its own static key, encrypted, and its [`Hello`]. The server is the responder: from that first message it learns
//! the connector's key, which its caller checks against the pinned ones before anything is answered; the second message
//! carries the [`Welcome`]. Then each side has a [`Transport`] that seals and opens the frames.
//!
//! A first message made for another server's key does not decrypt ([`HandshakeError::NotNoise`]); one from a key the
//! server does not know decrypts, and the caller closes without a word. A proxy that opens the outer TLS sees these
//! bytes alone: it can neither read them nor answer as the server.

use std::fmt;

use snow::params::NoiseParams;
use snow::resolvers::{CryptoResolver, DefaultResolver};
use snow::{HandshakeState, TransportState};
use zeroize::Zeroizing;

use crate::version::{Hello, Welcome};

/// The pattern, its curve, its cipher and its hash.
pub const PATTERN: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
/// Bound into both sides' handshake: another protocol's Noise does not pass for this one.
pub const PROLOGUE: &[u8] = b"escouade-connector-proto/1";
/// The most bytes of a Noise message.
pub const MAX_MESSAGE: usize = 65_535;
/// The bytes a Noise message adds to its payload once the handshake is done (the tag).
pub const TAG: usize = 16;

pub(crate) fn params() -> NoiseParams {
    PATTERN.parse().expect("the pattern is valid")
}

/// A static public key (X25519), pinned by the other side at the pairing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn from_slice(b: &[u8]) -> Option<PublicKey> {
        Some(PublicKey(b.try_into().ok()?))
    }

    pub fn from_hex(text: &str) -> Option<PublicKey> {
        PublicKey::from_slice(&hex::decode(text.trim()).ok()?)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", self.to_hex())
    }
}

/// A static key pair. Its private half never goes to a journal: `Debug` shows the public half alone.
pub struct Keypair {
    private: Zeroizing<[u8; 32]>,
    public: PublicKey,
}

impl Keypair {
    /// A new pair, from the system's randomness.
    pub fn generate() -> Keypair {
        let pair = snow::Builder::new(params())
            .generate_keypair()
            .expect("the system gives randomness");
        let mut private = Zeroizing::new([0u8; 32]);
        private.copy_from_slice(&pair.private);
        let mut raw = pair.private;
        zeroize::Zeroize::zeroize(&mut raw);
        Keypair::from_private(*private)
    }

    /// The pair of a private key kept by the caller (its file, a store of the system).
    pub fn from_private(private: [u8; 32]) -> Keypair {
        let private = Zeroizing::new(private);
        let mut dh = DefaultResolver
            .resolve_dh(&params().dh)
            .expect("X25519 is resolved");
        dh.set(&private[..]);
        let public = PublicKey::from_slice(dh.pubkey()).expect("a public key of 32 bytes");
        Keypair { private, public }
    }

    pub fn public(&self) -> PublicKey {
        self.public
    }

    /// The private key, for the caller to keep (a file readable by its account alone): never to a journal.
    pub fn private_bytes(&self) -> &[u8; 32] {
        &self.private
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keypair {{ public: {}, private: … }}", self.public)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum HandshakeError {
    /// Not a message of this handshake: garbage, another protocol, or made for another server's key.
    #[error("not a handshake of the connectors' protocol")]
    NotNoise,
    /// A message of the handshake whose payload is not understood.
    #[error("the handshake's payload is not understood")]
    BadPayload,
    /// A message that does not open (a transport message altered, replayed, or out of order).
    #[error("a message that does not decrypt")]
    Decrypt,
    #[error("a message too large for Noise")]
    TooLarge,
}

/// The connector's side, between its first message and the server's answer.
pub struct Initiator {
    state: HandshakeState,
}

impl Initiator {
    /// The first message, to a server whose key is `server`.
    pub fn start(
        keys: &Keypair,
        server: &PublicKey,
        hello: &Hello,
    ) -> Result<(Initiator, Vec<u8>), HandshakeError> {
        let mut state = snow::Builder::new(params())
            .local_private_key(&keys.private[..])
            .and_then(|b| b.remote_public_key(&server.0))
            .and_then(|b| b.prologue(PROLOGUE))
            .and_then(|b| b.build_initiator())
            .map_err(|_| HandshakeError::NotNoise)?;
        let mut out = vec![0u8; MAX_MESSAGE];
        let n = state
            .write_message(&hello.encode(), &mut out)
            .map_err(|_| HandshakeError::TooLarge)?;
        out.truncate(n);
        Ok((Initiator { state }, out))
    }

    /// The server's answer: its welcome, and the transport.
    pub fn finish(mut self, message: &[u8]) -> Result<(Welcome, Transport), HandshakeError> {
        if message.len() > MAX_MESSAGE {
            return Err(HandshakeError::TooLarge);
        }
        let mut payload = vec![0u8; MAX_MESSAGE];
        let n = self
            .state
            .read_message(message, &mut payload)
            .map_err(|_| HandshakeError::NotNoise)?;
        let welcome = Welcome::decode(&payload[..n]).map_err(|_| HandshakeError::BadPayload)?;
        let state = self
            .state
            .into_transport_mode()
            .map_err(|_| HandshakeError::NotNoise)?;
        Ok((welcome, Transport { state }))
    }
}

/// The server's side, once the first message is read: whose key it is and what it says.
pub struct Responder {
    state: HandshakeState,
    remote: PublicKey,
    hello: Hello,
}

impl Responder {
    /// Reads the connector's first message with the server's keys.
    pub fn read(keys: &Keypair, message: &[u8]) -> Result<Responder, HandshakeError> {
        if message.len() > MAX_MESSAGE {
            return Err(HandshakeError::TooLarge);
        }
        let mut state = snow::Builder::new(params())
            .local_private_key(&keys.private[..])
            .and_then(|b| b.prologue(PROLOGUE))
            .and_then(|b| b.build_responder())
            .map_err(|_| HandshakeError::NotNoise)?;
        let mut payload = vec![0u8; MAX_MESSAGE];
        let n = state
            .read_message(message, &mut payload)
            .map_err(|_| HandshakeError::NotNoise)?;
        let remote = state
            .get_remote_static()
            .and_then(PublicKey::from_slice)
            .ok_or(HandshakeError::NotNoise)?;
        let hello = Hello::decode(&payload[..n]).map_err(|_| HandshakeError::BadPayload)?;
        Ok(Responder {
            state,
            remote,
            hello,
        })
    }

    /// The connector's static key, to check against the pinned ones.
    pub fn remote(&self) -> PublicKey {
        self.remote
    }

    pub fn hello(&self) -> Hello {
        self.hello
    }

    /// The second message, with the server's welcome, and the transport.
    pub fn respond(mut self, welcome: &Welcome) -> Result<(Vec<u8>, Transport), HandshakeError> {
        let mut out = vec![0u8; MAX_MESSAGE];
        let n = self
            .state
            .write_message(&welcome.encode(), &mut out)
            .map_err(|_| HandshakeError::TooLarge)?;
        out.truncate(n);
        let state = self
            .state
            .into_transport_mode()
            .map_err(|_| HandshakeError::NotNoise)?;
        Ok((out, Transport { state }))
    }
}

/// The two ciphers of a channel, once the handshake is done.
pub struct Transport {
    state: TransportState,
}

impl Transport {
    /// The peer's static key.
    pub fn remote(&self) -> Option<PublicKey> {
        self.state
            .get_remote_static()
            .and_then(PublicKey::from_slice)
    }

    pub fn seal(&mut self, plain: &[u8]) -> Result<Vec<u8>, HandshakeError> {
        if plain.len() + TAG > MAX_MESSAGE {
            return Err(HandshakeError::TooLarge);
        }
        let mut out = vec![0u8; plain.len() + TAG];
        let n = self
            .state
            .write_message(plain, &mut out)
            .map_err(|_| HandshakeError::TooLarge)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn open(&mut self, message: &[u8]) -> Result<Vec<u8>, HandshakeError> {
        if message.len() > MAX_MESSAGE {
            return Err(HandshakeError::TooLarge);
        }
        if message.len() < TAG {
            return Err(HandshakeError::Decrypt);
        }
        let mut out = vec![0u8; message.len()];
        let n = self
            .state
            .read_message(message, &mut out)
            .map_err(|_| HandshakeError::Decrypt)?;
        out.truncate(n);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::{Verdict, Version};

    fn hello() -> Hello {
        Hello {
            version: Version::new(2, 3, 0),
        }
    }

    #[test]
    fn the_handshake_pins_both_keys() {
        let server = Keypair::generate();
        let connector = Keypair::generate();
        let (initiator, first) = Initiator::start(&connector, &server.public(), &hello()).unwrap();
        // e, s (encrypted), the hello (encrypted): 32 + 48 + 6 + 16.
        assert_eq!(first.len(), 102);
        let responder = Responder::read(&server, &first).unwrap();
        assert_eq!(responder.remote(), connector.public());
        assert_eq!(responder.hello(), hello());
        let welcome = Welcome::for_hello(Version::new(2, 4, 1), &responder.hello());
        let (second, mut at_server) = responder.respond(&welcome).unwrap();
        let (told, mut at_connector) = initiator.finish(&second).unwrap();
        assert_eq!(told, welcome);
        assert_eq!(told.verdict, Verdict::Current);
        assert_eq!(at_connector.remote(), Some(server.public()));
        assert_eq!(at_server.remote(), Some(connector.public()));

        let sealed = at_connector.seal(b"open gitlab:22").unwrap();
        assert!(
            !sealed.windows(6).any(|w| w == b"gitlab"),
            "nothing in clear"
        );
        assert_eq!(at_server.open(&sealed).unwrap(), b"open gitlab:22");
        // Replayed or altered: refused.
        assert_eq!(at_server.open(&sealed), Err(HandshakeError::Decrypt));
        let mut altered = at_server.seal(b"opened").unwrap();
        altered[0] ^= 1;
        assert_eq!(at_connector.open(&altered), Err(HandshakeError::Decrypt));
    }

    #[test]
    fn a_first_message_for_another_server_does_not_open() {
        let server = Keypair::generate();
        let other = Keypair::generate();
        let connector = Keypair::generate();
        let (_, first) = Initiator::start(&connector, &other.public(), &hello()).unwrap();
        assert_eq!(
            Responder::read(&server, &first).err(),
            Some(HandshakeError::NotNoise)
        );
        assert_eq!(
            Responder::read(&server, b"GET / HTTP/1.1").err(),
            Some(HandshakeError::NotNoise)
        );
        assert_eq!(
            Responder::read(&server, &[]).err(),
            Some(HandshakeError::NotNoise)
        );
    }

    #[test]
    fn an_answer_from_another_key_does_not_open() {
        // A proxy that plays the server, with a key of its own, cannot answer the first message.
        let server = Keypair::generate();
        let proxy = Keypair::generate();
        let connector = Keypair::generate();
        let (initiator, first) = Initiator::start(&connector, &server.public(), &hello()).unwrap();
        assert!(Responder::read(&proxy, &first).is_err());
        // Nor by answering the message of a handshake it made itself with the real server's key.
        let (_, first_to_proxy) = Initiator::start(&proxy, &server.public(), &hello()).unwrap();
        let responder = Responder::read(&server, &first_to_proxy).unwrap();
        let (second, _) = responder
            .respond(&Welcome::for_hello(Version::new(2, 3, 0), &hello()))
            .unwrap();
        assert_eq!(
            initiator.finish(&second).err(),
            Some(HandshakeError::NotNoise)
        );
        let _ = first;
    }

    #[test]
    fn a_pair_comes_back_from_its_private_key() {
        let pair = Keypair::generate();
        let again = Keypair::from_private(*pair.private_bytes());
        assert_eq!(again.public(), pair.public());
        assert_eq!(
            PublicKey::from_hex(&pair.public().to_hex()),
            Some(pair.public())
        );
        assert_eq!(PublicKey::from_hex("00"), None);
        let shown = format!("{pair:?}");
        assert!(
            !shown.contains(&hex::encode(pair.private_bytes())),
            "{shown}"
        );
    }
}
