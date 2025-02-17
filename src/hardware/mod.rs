use redgold_schema::structs::{Address, ErrorInfo};

pub mod trezor;
// pub mod debug;
// pub mod trezor_unchecked;


trait HardwareWallet {
    fn get_address<S: Into<String>>(&self, path: S) -> Result<Address, ErrorInfo>;
    fn sign<S: Into<String>, M: Into<String>>(&self, path: S, message: M) -> Result<String, ErrorInfo>;
}