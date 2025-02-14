use redgold_keys::xpub_wrapper::XpubWrapper;
use redgold_schema::RgResult;
use crate::hardware::trezor;

pub fn get_cold_xpub(dp: String) -> RgResult<String> {
    let node = trezor::get_public_node(dp)?;
    let w = XpubWrapper::new(node.xpub);
    w.public_at(0, 0)?;
    Ok(w.xpub)
}