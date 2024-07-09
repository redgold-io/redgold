use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;
use aws_sdk_sesv2::{Client, Error};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use redgold_schema::{error_info, RgResult};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::ErrorInfo;
use crate::node_config::NodeConfig;


const DEFAULT_EMAIL: &str = "info@redgold.io";

#[async_trait]
pub trait EmailOnError<T> {
    async fn email_on_error(self) -> RgResult<T> where T : Send ;
}

#[async_trait]
impl<T> EmailOnError<T> for RgResult<T> {
    async fn email_on_error(self) -> RgResult<T>
    where T : Send {
        match self {
            Ok(s) => { Ok(s) }
            Err(e) => {
                if std::env::var("REDGOLD_MAIN_DEVELOPMENT_MODE").is_ok() {
                    email_default("Error in Redgold", e.json_or()).await.mark_abort()?;
                }
                Err(e)
            }
        }
    }
}

pub async fn email_default(
    subject: impl Into<String>,
    body: impl Into<String>,
) -> Result<(), ErrorInfo> {
    email_from_to(subject, body, DEFAULT_EMAIL, DEFAULT_EMAIL).await
}

pub async fn email_cfg(
    subject: impl Into<String>,
    body: impl Into<String>,
    nc: &NodeConfig
) -> RgResult<()> {
    if let (Some(from), Some(to)) = (nc.opts.from_email.as_ref(), nc.opts.to_email.as_ref()) {
        email_from_to(subject, body, from, to).await
    } else {
        return Ok(())
    }

}

pub async fn email_from_to(
    subject: impl Into<String>,
    body: impl Into<String>,
    from: impl Into<String>,
    to: impl Into<String>
) -> Result<(), ErrorInfo> {

    if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
        return Ok(()); // Don't send emails if we don't have the keys
    }

    let region = Some("us-east-1");

    let region_provider = RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-east-1"));

    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&shared_config);
    let string = subject.into();
    let body_string = body.into();
    let from = from.into();
    send_message(&client, to.into(), &from, &string, &body_string)
        .await.map_err(|e| error_info(e.to_string()))
}

// Sends a message to all members of the contact list.
// snippet-start:[ses.rust.send-email]
pub async fn send_message(
    client: &Client,
    to: String,
    from: &str,
    subject: &str,
    message: &str,
) -> Result<(), Error> {

    let dest = Destination::builder().to_addresses(to).build();
    let subject_content = Content::builder().data(subject).charset("UTF-8").build();
    let body_content = Content::builder().data(message).charset("UTF-8").build();
    let body = Body::builder().text(body_content).build();

    let msg = Message::builder()
        .subject(subject_content)
        .body(body)
        .build();

    let email_content = EmailContent::builder().simple(msg).build();

    client
        .send_email()
        .from_email_address(from)
        .destination(dest)
        .content(email_content)
        .send()
        .await?;

    Ok(())
}

#[ignore]
#[tokio::test]
async fn debug_send() {
    email_default("test", "test").await.expect("Failed to send email");
}
