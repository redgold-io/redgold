use crate::btc::btc_wallet;
use bdk::bitcoin::blockdata::opcodes;
use bdk::bitcoin::blockdata::script::Builder as ScriptBuilder;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::secp256k1::{All, Secp256k1, Signature};
use bdk::bitcoin::util::sighash;
use bdk::bitcoin::{psbt, EcdsaSighashType, Script, Sighash};
use bdk::signer::{InputSigner, SignerCommon, SignerError, SignerId};
use bdk::{bitcoin, SignOptions, TransactionDetails};
use redgold_schema::structs::{ErrorInfo, Proof};
use redgold_schema::{error_info, structs, ErrorInfoContext, SafeOption};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct MultipartySigner {
    pub public_key: structs::PublicKey,
    pub proofs: Arc<RwLock<HashMap<usize, Proof>>>,
    pub err: Arc<RwLock<Option<ErrorInfo>>>
}

impl MultipartySigner {
    pub fn new(public_key: structs::PublicKey) -> Self {
        Self {
            public_key,
            proofs: Default::default(),
            err: Arc::new(RwLock::new(None)),
        }
    }
    pub fn sign_input(&self,
                      psbt: &mut PartiallySignedTransaction,
                      input_index: usize,
                      hash_ty: EcdsaSighashType,
                      _sign_options: &SignOptions
    ) -> Result<(), ErrorInfo> {
        let arc = self.proofs.clone();
        let guard = arc.read().unwrap();
        let proof = guard.get(&input_index).ok_or(error_info("No proof found"))?;
        let signature = proof.signature.safe_get_msg("Missing signature in proof")?;
        let sig = Signature::from_compact(&*signature.raw_bytes()?).error_msg(
            structs::ErrorCode::IncorrectSignature,
            "Decoded signature construction failure",
        )?;

        let final_signature = bitcoin::EcdsaSig { sig, hash_ty };

        let public_key = proof.public_key.safe_get_msg("Missing public key")?.raw_bytes()?;
        let public_key = bdk::bitcoin::util::key::PublicKey::from_slice(&*public_key)
            .error_info("Public key failure")?;

        let input = psbt.inputs.get_mut(input_index).ok_or(error_info("No psbt found"))?;
        input
            .partial_sigs
            .insert(public_key, final_signature);

        Ok(())
    }
}

impl SignerCommon for MultipartySigner {
    fn id(&self, _secp: &Secp256k1<All>) -> SignerId {
        let pk = btc_wallet::struct_public_to_bdk_pubkey(&self.public_key).unwrap();
        SignerId::PkHash(pk.pubkey_hash().as_hash())
    }
}

impl InputSigner for MultipartySigner {
    fn sign_input(&self,
                  psbt: &mut PartiallySignedTransaction,
                  input_index: usize,
                  sign_options: &SignOptions, _secp: &Secp256k1<All>
    ) -> Result<(), SignerError> {
        let (_, sighash_type) = segwit_sighash(psbt, input_index, ())?;
        match self.sign_input(psbt, input_index, sighash_type, sign_options) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                *self.err.write().unwrap() = Some(e);
                Err(SignerError::UserCanceled)
            }
        }
    }
}

pub fn p2wpkh_script_code(script: &Script) -> Script {
    ScriptBuilder::new()
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(&script[2..])
        .push_opcode(opcodes::all::OP_EQUALVERIFY)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script()
}

pub fn segwit_sighash(
    psbt: &psbt::PartiallySignedTransaction,
    input_index: usize,
    _extra: (),
) -> Result<(Sighash, EcdsaSighashType), SignerError> {
    if input_index >= psbt.inputs.len() || input_index >= psbt.unsigned_tx.input.len() {
        return Err(SignerError::InputIndexOutOfRange);
    }

    let psbt_input = &psbt.inputs[input_index];
    let tx_input = &psbt.unsigned_tx.input[input_index];

    let sighash = psbt_input
        .sighash_type
        .unwrap_or_else(|| EcdsaSighashType::All.into())
        .ecdsa_hash_ty()
        .map_err(|_| SignerError::InvalidSighash)?;

    // Always try first with the non-witness utxo
    let utxo = if let Some(prev_tx) = &psbt_input.non_witness_utxo {
        // Check the provided prev-tx
        if prev_tx.txid() != tx_input.previous_output.txid {
            return Err(SignerError::InvalidNonWitnessUtxo);
        }

        // The output should be present, if it's missing the `non_witness_utxo` is invalid
        prev_tx
            .output
            .get(tx_input.previous_output.vout as usize)
            .ok_or(SignerError::InvalidNonWitnessUtxo)?
    } else if let Some(witness_utxo) = &psbt_input.witness_utxo {
        // Fallback to the witness_utxo. If we aren't allowed to use it, signing should fail
        // before we get to this point
        witness_utxo
    } else {
        // Nothing has been provided
        return Err(SignerError::MissingNonWitnessUtxo);
    };
    let value = utxo.value;

    let script = match psbt_input.witness_script {
        Some(ref witness_script) => witness_script.clone(),
        None => {
            if utxo.script_pubkey.is_v0_p2wpkh() {
                p2wpkh_script_code(&utxo.script_pubkey)
            } else if psbt_input
                .redeem_script
                .as_ref()
                .map(Script::is_v0_p2wpkh)
                .unwrap_or(false)
            {
                p2wpkh_script_code(psbt_input.redeem_script.as_ref().unwrap())
            } else {
                return Err(SignerError::MissingWitnessScript);
            }
        }
    };

    Ok((
        sighash::SighashCache::new(&psbt.unsigned_tx).segwit_signature_hash(
            input_index,
            &script,
            value,
            sighash,
        )?,
        sighash,
    ))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub psbt: Option<PartiallySignedTransaction>,
    pub transaction_details: Option<TransactionDetails>,
}