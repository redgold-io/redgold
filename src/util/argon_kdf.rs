use crate::util::current_time_millis;
use argon2::Algorithm::Argon2d;
use argon2::{Argon2, Params, Version};
use redgold_schema::{structs, RgResult};


pub fn argon2d_hash(salt: Vec<u8>, password: Vec<u8>, m_cost: u32, t_cost: u32, p_cost: u32)
                    -> RgResult<Vec<u8>> {

    let params = Params::new(m_cost, t_cost, p_cost, Some(32))
        .expect("");
        //.error_info("bad parameters")?;
    let arg = Argon2::new(Argon2d, Version::V0x13, params);

    let mut output_key_material = [0u8; 32]; // Can be any desired size
    arg.hash_password_into(&*password, &*salt, &mut output_key_material)
        .expect("");
        // .error_info("hashing failed")?;
    Ok(output_key_material.to_vec())
}

#[ignore]
#[test]
fn test_argon_params() {
    let salt = structs::Hash::from_string_calculate("asdf").vec();
    let pw = structs::Hash::from_string_calculate("asdf2").vec();
    let _m = Params::DEFAULT_M_COST;
    // let m = 19*1024*10;
    let m = 64*1024;
    let _t = Params::DEFAULT_T_COST;
    let t = 10;
    let _p = Params::DEFAULT_P_COST;
    let p = 2;
    let start = current_time_millis();
    let result = argon2d_hash(salt, pw, m, t, p).expect("argon2 hash");
    let end = current_time_millis();
    println!("argon2d_hash took {} ms", end - start);
    println!("hex result {}", hex::encode(result));

}