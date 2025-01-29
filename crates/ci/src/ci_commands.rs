use redgold_common_no_wasm::cmd::run_command_os;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::RgResult;

#[allow(dead_code)]
pub async fn get_branch() -> RgResult<String> {
    let (stdout, _) = run_command_os("git branch --show-current".to_string()).await?;
    Ok(stdout.replace("\n", ""))
}

#[allow(dead_code)]
pub async fn is_public_cluster_branch() -> RgResult<bool> {
    let branch = get_branch().await?;
    let branches =
        NetworkEnvironment::status_networks().iter()
            .map(|n| n.to_std_string())
            .collect::<Vec<String>>();
    Ok(branches.contains(&branch))
}

#[cfg(test)]
mod test {
    use redgold_schema::util::lang_util::AnyPrinter;

    use crate::ci_commands::get_branch;

    #[ignore]
    #[tokio::test]
    async fn test_get_branch() {
        get_branch().await.expect("test").print();
    }
}