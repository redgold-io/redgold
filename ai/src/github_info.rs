use octocrab::models::issues::Issue;
use octocrab::{models, params};
use redgold_schema::{ErrorInfoContext, RgResult};

pub async fn issues() -> RgResult<Vec<Issue>> {
    let octocrab = octocrab::instance();
// Returns the first page of all issues.
    let mut page = octocrab
        .issues("redgold-io", "redgold")
        .list()
        // Optional Parameters
        // .creator("XAMPPRocky")
        .state(params::State::All)
        .per_page(100)
        .send()
        .await
        .error_info("Failed to get issues")?;

    let mut all = vec![];
// Go through every page of issues. Warning: There's no rate limiting so
// be careful.
    loop {
        for issue in &page {
            println!("{} {}", issue.number, issue.title);
            all.push(issue.clone());
        }
        page = match octocrab
            .get_page::<Issue>(&page.next)
            .await.error_info("Failed to get next page")?
        {
            Some(next_page) => next_page,
            None => break,
        }
    }
    Ok(all)
}

#[ignore]
#[tokio::test]
async fn test_issues() {
    issues().await.unwrap();
}