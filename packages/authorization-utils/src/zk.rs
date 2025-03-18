use ark_bn254::Bn254;
use ark_groth16::{Groth16, PreparedVerifyingKey};
use ark_serialize::CanonicalDeserialize;

// zkVMs typically produce Groth16 proof outputs. The widely utilized, audited library
// `ark-groth16` is a foundational option that will support multiple zkVMs.
// `PreparedVerifyingKey<Bn254>` here, but then we have impact on the required
// implementations for the authorization type (json schema, etc)
pub type VerifyingKey = Vec<u8>;

pub trait ProvingSystem {
    type VerifyingKey: CanonicalDeserialize;
    type PublicInputs;
    type Proof;
}

pub fn verify_proof(vk: &VerifyingKey, proof: &[u8], inputs: &[u8]) -> bool {
    let pvk: PreparedVerifyingKey<Bn254> =
        match CanonicalDeserialize::deserialize_uncompressed(vk.as_slice()) {
            Ok(k) => k,
            _ => return false,
        };

    // TODO derive the field elements from the inputs, depending on the zkVM backend
    let _ = inputs;
    let inputs = &[];
    let inputs = match Groth16::<Bn254>::prepare_inputs(&pvk, inputs) {
        Ok(i) => i,
        _ => return false,
    };

    let proof = match ark_groth16::Proof::deserialize_compressed(proof) {
        Ok(p) => p,
        _ => return false,
    };

    Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, &proof, &inputs).is_ok()
}
