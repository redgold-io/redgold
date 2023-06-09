use argon2::Algorithm::Argon2d;
use argon2::{Argon2, Params, Version};
use redgold_schema::{ErrorInfoContext, RgResult, structs};
use redgold_schema::structs::ErrorInfo;


pub fn argon2_hash(salt: Vec<u8>, password: Vec<u8>, m_cost: u32, t_cost: u32, p_cost: u32)
    -> RgResult<Vec<u8>> {

    let mut params = Params::new(m_cost, t_cost, p_cost, Some(32))
        .expect("");
        //.error_info("bad parameters")?;
    let arg = Argon2::new(Argon2d, Version::V0x13, params);

    let mut output_key_material = [0u8; 32]; // Can be any desired size
    arg.hash_password_into(&*password, &*salt, &mut output_key_material)
        .expect("");
        // .error_info("hashing failed")?;
    Ok(output_key_material.to_vec())
}

#[test]
fn test_argon_params() {
    let salt = structs::Hash::from_string_calculate("asdf").vec();
    let pw = structs::Hash::from_string_calculate("asdf2").vec();
    let m = Params::DEFAULT_M_COST;
    let t = Params::DEFAULT_T_COST;
    let p = Params::DEFAULT_P_COST;
    argon2_hash(salt, pw, m, t, p).expect("argon2 hash");

}