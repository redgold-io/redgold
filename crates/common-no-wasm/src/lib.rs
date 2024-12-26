pub mod data_folder_read_ext;
pub mod retry;
pub mod cmd;
pub mod tx_new;
pub mod machine_info;
pub mod ssh_like;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
