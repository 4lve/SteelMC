use rsa::{RsaPrivateKey, traits::PublicKeyParts};

pub struct KeyStore {
    pub private_key: RsaPrivateKey,
    pub public_key_der: Vec<u8>,
}

impl KeyStore {
    #[must_use]
    pub fn create() -> Self {
        log::debug!("Creating encryption keys...");
        let private_key = Self::generate_private_key();

        let public_key_der = rsa_der::public_key_to_der(
            &private_key.n().to_be_bytes(),
            &private_key.e().to_be_bytes(),
        );

        Self {
            private_key,
            public_key_der,
        }
    }

    fn generate_private_key() -> RsaPrivateKey {
        // Found out that OsRng is faster than rand::thread_rng here
        let mut rng = rand::rng();

        // let pub_key = RsaPublicKey::from(&priv_key);
        RsaPrivateKey::new(&mut rng, 1024).expect("Failed to generate a key")
    }
}
