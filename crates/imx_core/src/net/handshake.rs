use anyhow::{bail, Result};
use num::BigUint;
use rand::{RngCore, thread_rng};
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::RsaPublicKey;
use thiserror::Error;
use crate::defines::TEMP_AUTH_KEY_EXPIRE_TIME;

use crate::proto;
use crate::proto::{MtSer, PQInnerData};
use crate::util::factorize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HandshakeType {
    Perm,
    Temp,
    MediaTemp,
    Current,
    All
}
/// https://core.telegram.org/mtproto/auth_key#dh-exchange-initiation
pub struct Step1 {
    nonce: [u8; 16],
    dc_id: i32,
}
pub struct Step2 {
    nonce: [u8; 16],
    server_nonce: [u8; 16],
    new_nonce: [u8; 32],
}
pub struct Step3 {
    nonce: [u8; 16],
    server_nonce: [u8; 16],
    new_nonce: [u8; 32],
    gab: BigUint,
    time_diff: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    pub auth_key: [u8; 256],
    pub time_diff: i32,
    pub first_salt: i64,
}

#[derive(Error, Clone, Debug, PartialEq)]
pub enum Error {
    #[error("invalid nonce: got {got:?}, expected {expected:?}")]
    InvalidNonce { got: [u8; 16], expected: [u8; 16] },
    #[error("invalid pq size {0}")]
    InvalidPQSize(usize),
    #[error("inner data too large {0}")]
    InnerDataTooLarge(usize),
    #[error("all server fingerprints are unknown: {0:?}")]
    UnknownFingerprints(Vec<i64>),
}

pub fn step1(dc_id: i32) -> Result<(proto::ReqPQMulti, Step1)> {
    let mut random_bytes = [0; 16];
    thread_rng().fill_bytes(&mut random_bytes);
    let req = proto::ReqPQMulti { nonce: random_bytes };
    let step = Step1 { nonce: random_bytes, dc_id };
    Ok((req, step))
}

pub fn step2(prev: Step1, res: proto::ResPQ) -> Result<(proto::ReqDHParams, Step2)> {
    let Step1 { nonce, dc_id } = prev;

    check_nonce(&res.nonce, &nonce)?;

    let pq_len = res.pq.len();
    if pq_len != 8 {
        bail!(Error::InvalidPQSize(pq_len));
    }

    let pq = {
        let mut buf = [0; 8];
        buf.copy_from_slice(&res.pq);
        u64::from_be_bytes(buf)
    };

    let (p, q) = factorize(pq);
    let p = [(p >> 24) as u8, (p >> 16) as u8, (p >> 8) as u8, p as u8];
    let q = [(q >> 24) as u8, (q >> 16) as u8, (q >> 8) as u8, q as u8];

    let mut random_bytes = [0; 32 + 224];
    thread_rng().fill_bytes(&mut random_bytes);
    let new_nonce = {
        let mut buf = [0; 32];
        buf.copy_from_slice(&random_bytes[..32]);
        buf
    };
    // Remove the now-used first part from our available random data.
    let random_bytes = {
        let mut buf = [0; 224];
        buf.copy_from_slice(&random_bytes[32..]);
        buf
    };

    let handshake_type = HandshakeType::Perm;
    let pq_inner_data = if handshake_type == HandshakeType::Perm {
        PQInnerData::Dc {
            pq: res.pq,
            p,
            q,
            nonce,
            server_nonce: res.server_nonce,
            new_nonce,
            dc: dc_id, // todo if testBackend { cur_dc_id + 10000 } else { cur_dc_id }
        }
    } else {
        PQInnerData::TempDc {
            pq: res.pq,
            p,
            q,
            nonce,
            server_nonce: res.server_nonce,
            new_nonce,
            dc: dc_id, // todo if testBackend { cur_dc_id + 10000 } else { cur_dc_id }
            expires_in: TEMP_AUTH_KEY_EXPIRE_TIME,
        }
    };
    let pq_inner_data = pq_inner_data.to_bytes()?;
    if pq_inner_data.len() > 144 {
        bail!(Error::InnerDataTooLarge(pq_inner_data.len()));
    }
    // todo

    let public_key_fingerprint = match res.server_public_key_fingerprints
        .iter().cloned().find(|&f| pub_key_for_fingerprint(f).is_some()) {
        Some(x) => x,
        None => bail!(Error::UnknownFingerprints(res.server_public_key_fingerprints.clone()))
    };

    let pub_key = pub_key_for_fingerprint(public_key_fingerprint).unwrap();

    let encrypted_data = vec![]; // todo rsa::encrypt_hashed(&pq_inner_data, &pub_key, &random_bytes);



    let req = proto::ReqDHParams {
        nonce,
        server_nonce: res.server_nonce,
        p,
        q,
        public_key_fingerprint,
        encrypted_data,
    };
    let step = Step2 {
        nonce,
        server_nonce: res.server_nonce,
        new_nonce,
    };

    Ok((req, step))
}

pub fn step3(prev: Step2, res: proto::ServerDHParams) -> Result<(proto::SetClientDHParams, Step3)> {
    todo!()
}

pub fn complete(prev: Step3, res: proto::SetClientDHParamsAnswer) -> Result<Completion> {
    todo!()
}

fn check_nonce(got: &[u8; 16], expected: &[u8; 16]) -> Result<()> {
    if got == expected {
        Ok(())
    } else {
        bail!(Error::InvalidNonce { got: *got, expected: *expected })
    }
}

fn pub_key_for_fingerprint(fingerprint: i64) -> Option<RsaPublicKey> {
    match fingerprint as u64 {
        // 正式服
        0xd09d1d85de64fd85 => RsaPublicKey::from_pkcs1_pem(
            "-----BEGIN RSA PUBLIC KEY-----\n\
            MIIBCgKCAQEA6LszBcC1LGzyr992NzE0ieY+BSaOW622Aa9Bd4ZHLl+TuFQ4lo4g\n\
            5nKaMBwK/BIb9xUfg0Q29/2mgIR6Zr9krM7HjuIcCzFvDtr+L0GQjae9H0pRB2OO\n\
            62cECs5HKhT5DZ98K33vmWiLowc621dQuwKWSQKjWf50XYFw42h21P2KXUGyp2y/\n\
            +aEyZ+uVgLLQbRA1dEjSDZ2iGRy12Mk5gpYc397aYp438fsJoHIgJ2lgMv5h7WY9\n\
            t6N/byY9Nw9p21Og3AoXSL2q/2IJ1WRUhebgAdGVMlV1fkuOQoEzR7EdpqtQD9Cs\n\
            5+bfo3Nhmcyvk5ftB0WkJ9z6bNZ7yxrP8wIDAQAB\n\
            -----END RSA PUBLIC KEY-----"
        ).ok(),
        // 测试服
        0xb25898df208d2603 => RsaPublicKey::from_pkcs1_pem(
            "-----BEGIN RSA PUBLIC KEY-----\n\
            MIIBCgKCAQEAyMEdY1aR+sCR3ZSJrtztKTKqigvO/vBfqACJLZtS7QMgCGXJ6XIR\n\
            yy7mx66W0/sOFa7/1mAZtEoIokDP3ShoqF4fVNb6XeqgQfaUHd8wJpDWHcR2OFwv\n\
            plUUI1PLTktZ9uW2WE23b+ixNwJjJGwBDJPQEQFBE+vfmH0JP503wr5INS1poWg/\n\
            j25sIWeYPHYeOrFp/eXaqhISP6G+q2IeTaWTXpwZj4LzXq5YOpk4bYEQ6mvRq7D1\n\
            aHWfYmlEGepfaYR8Q0YqvvhYtMte3ITnuSJs171+GDqpdKcSwHnd6FudwGO4pcCO\n\
            j4WcDuXc2CTHgH8gFTNhp/Y8/SpDOhvn9QIDAQAB\n\
            -----END RSA PUBLIC KEY-----"
        ).ok(),
        _ => None
    }
}