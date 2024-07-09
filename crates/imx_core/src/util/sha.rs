#[macro_export]
macro_rules! sha1 (
    ( $( $x:expr ),* ) => ({
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        $(
            hasher.update($x);
        )+
        let sha: [u8; 20] = hasher.finalize().into();
        sha
    })
);

#[macro_export]
macro_rules! sha256 (
    ( $( $x:expr ),* ) => ({
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        $(
            hasher.update($x);
        )+
        let sha: [u8; 32] = hasher.finalize().into();
        sha
    })
);
