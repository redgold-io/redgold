use crate::proof_support::ProofSupport;
use crate::{KeyPair, TestConstants};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::structs::{AboutNodeResponse, NodeMetadata, Proof, PublicKey};
use redgold_schema::message::Response;
use redgold_schema::message::Request;
use redgold_schema::{error_info, RgResult, SafeOption};

pub trait RequestSupport {
    fn with_auth(self, key_pair: &KeyPair) -> Request;
    fn verify_auth(&self) -> RgResult<PublicKey>;
    fn verify_auth_inner(&self) -> RgResult<PublicKey>;
}

impl RequestSupport for Request {

    fn with_auth(mut self, key_pair: &KeyPair) -> Request {
        let hash = self.calculate_hash();
        // println!("with_auth hash: {:?}", hash.hex());
        let proof = Proof::from_keypair_hash(&hash, &key_pair);
        proof.verify_signature_only(&hash).expect("immediate verify");
        self.proof = Some(proof);
        self
    }

    fn verify_auth(&self) -> RgResult<PublicKey> {
        self.verify_auth_inner()
            .add("Request authorization failure")
            .add(self.json_or())
    }
    fn verify_auth_inner(&self) -> RgResult<PublicKey> {
        let hash = self.calculate_hash();
        let proof = self.proof.safe_get_msg("Missing proof on request authentication verification")?;
        let nmd = self.node_metadata.safe_get_msg("Missing node metadata on request authentication verification")?;
        let nmd_pk = nmd.public_key.safe_get_msg("Missing public key on node metadata request authentication verification")?;
        let pk = proof.public_key.safe_get_msg("Missing public key on request authentication verification")?;
        let proof_ver = proof.verify_signature_only(&hash);
        // if proof_ver.is_err() {
        //     let sig_bytes = proof.clone().signature.unwrap().bytes;
        //     let b = sig_bytes.safe_bytes().expect("");
        //     let hh = hex::encode(b);
        //     let js = self.json_or();
        //     info!("proof verification failure on request with calculate_hash={} pk={} sig={} request={}",
        //         hash.hex(),
        //         pk.hex_or(),
        //         hh,
        //         js
        //     );
        //     hh;
        // }
        proof_ver?;

        if nmd_pk != pk {
            return Err(error_info("Node metadata public key and proof public key mismatch on request authentication verification"));
        }
        Ok(pk.clone())
    }

}

pub trait ResponseSupport {
    fn with_auth(self, key_pair: &KeyPair) -> Response;
    fn verify_auth(self, intended_pk: Option<&PublicKey>) -> RgResult<Self>
     where Self: Sized;
    fn verify_auth_inner(self, intended_pk: Option<&PublicKey>) -> RgResult<Self>
        where Self: Sized;
    fn proceed_from_proof_pk(&self, intended_pk: Option<&PublicKey>, nmd_pk: &PublicKey, proof: &Proof, proof_pk: &PublicKey) -> RgResult<()>;
}

impl ResponseSupport for Response {

    fn with_auth(mut self, key_pair: &KeyPair) -> Response {
        let hash = self.calculate_hash();
        let proof = Proof::from_keypair_hash(&hash, &key_pair);
        self.proof = Some(proof);
        self
    }

    fn verify_auth(self, intended_pk: Option<&PublicKey>) -> RgResult<Self> {
        let pk_json = intended_pk.json_or();
        self.verify_auth_inner(intended_pk)
            .add("Response authorization failure")
            .with_detail("intended_pk_json", pk_json)
    }

    fn verify_auth_inner(self, intended_pk: Option<&PublicKey>) -> RgResult<Self> {
        let nmd = self.node_metadata.safe_get_msg("Missing node metadata on response authentication verification")?;
        let nmd_pk = nmd.public_key.safe_get_msg("Missing public key on node metadata response authentication verification")?;
        let proof = self.proof.safe_get_msg("Missing proof on response authentication verification")?;
        let proof_pk = proof.public_key.safe_get_msg("Missing public key on proof on response authentication verification")?;
        self.proceed_from_proof_pk(intended_pk, nmd_pk, proof, proof_pk)
            .with_detail("nmd_pk", nmd_pk.hex())
            .with_detail("proof_pk", proof_pk.hex())
            ?;
        Ok(self)
    }
    fn proceed_from_proof_pk(&self, intended_pk: Option<&PublicKey>, nmd_pk: &PublicKey, proof: &Proof, proof_pk: &PublicKey) -> RgResult<()> {
        proof.verify_signature_only(&self.calculate_hash()).add("proof verification failure on response")?;
        if nmd_pk != proof_pk {
            return Err(error_info("Node metadata public key and proof public key mismatch on response authentication verification"));
        }
        if let Some(intended) = intended_pk {
            if proof_pk != intended {
                return
                    Err(error_info("Intended public key and proof public key mismatch on response authentication verification"
                    )).with_detail("intended_pk", intended.hex()).with_detail("proof_pk", proof_pk.hex());
            }
        }
        Ok(())
    }

}



#[test]
fn verify_request_auth() {
    let tc = TestConstants::new();
    let mut req = Request::empty();
    req.about();
    let kp = tc.key_pair();
    let mut nmd = NodeMetadata::default();
    nmd.public_key = Some(kp.public_key());
    req.node_metadata = Some(nmd);
    req.with_auth(&kp).verify_auth().unwrap();
    // println!("after with auth assign proof {}", req.calculate_hash().hex());

}

#[test]
fn verify_response_auth() {
    let tc = TestConstants::new();
    let mut req = Response::default();
    let mut anr = AboutNodeResponse::default();
    anr.num_active_peers = 1;
    req.about_node_response = Some(anr);
    let kp = tc.key_pair();
    let mut nmd = NodeMetadata::default();
    let pubk = kp.public_key();
    nmd.public_key = Some(pubk.clone());
    req.node_metadata = Some(nmd);
    req.with_auth(&kp).verify_auth(Some(&pubk)).unwrap();
    // println!("after with auth assign proof {}", req.calculate_hash().hex());

}
