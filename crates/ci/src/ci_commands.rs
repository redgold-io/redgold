use redgold_common_no_wasm::cmd::run_command_os;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::RgResult;

pub async fn get_branch() -> RgResult<String> {
    let (stdout, stderr) = run_command_os("git branch --show-current".to_string()).await?;
    Ok(stdout.replace("\n", ""))
}

pub async fn is_public_cluster_branch() -> RgResult<bool> {
    let branch = get_branch().await?;
    let branches =
        NetworkEnvironment::status_networks().iter()
            .map(|n| n.to_std_string())
            .collect::<Vec<String>>();
    Ok(branches.contains(&branch))
}

#[ignore]
#[tokio::test]
async fn test_get_branch() {
    get_branch().await.expect("test").print();
}