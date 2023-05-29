/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

#![allow(clippy::result_large_err)]

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use aws_sdk_sesv2::{config::Region, meta::PKG_VERSION, Client, Error};
use clap::Parser;
use redgold_schema::error_info;
use redgold_schema::structs::ErrorInfo;

#[derive(Debug, Parser)]
struct Opt {
    /// The contact list containing email addresses to send the message to.
    #[structopt(short, long)]
    contact_list: String,

    /// The AWS Region.
    #[structopt(short, long)]
    region: Option<String>,

    /// The email address of the sender.
    #[structopt(short, long)]
    from_address: String,

    /// The message of the email.
    #[structopt(short, long)]
    message: String,

    /// The subject of the email.
    #[structopt(short, long)]
    subject: String,

    /// Whether to display additional information.
    #[structopt(short, long)]
    verbose: bool,
}

// Sends a message to all members of the contact list.
// snippet-start:[ses.rust.send-email]
async fn send_message(
    client: &Client,
    to: String,
    from: &str,
    subject: &str,
    message: &str,
) -> Result<(), Error> {
    // Get list of email addresses from contact list.
    // let resp = client
    //     .list_contacts()
    //     .contact_list_name(list)
    //     .send()
    //     .await?;

    // let contacts = resp.contacts().unwrap_or_default();
    //
    // let cs: String = contacts
    //     .iter()
    //     .map(|i| i.email_address().unwrap_or_default())
    //     .collect();

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

    println!("Email sent to list");

    Ok(())
}
// snippet-end:[ses.rust.send-email]

#[ignore]
#[tokio::test]
async fn debug()  {
  email("Yo", "whats up").await.expect("Failed to send email");
}

pub async fn email<S: Into<String>, Y: Into<String>>(subject: S, body: Y) -> Result<(), ErrorInfo> {

    if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
        return Ok(()); // Don't send emails if we don't have the keys
    }

    let region = Some("us-east-1");

    let region_provider = RegionProviderChain::first_try(region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-east-1"));

    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&shared_config);

    let to_address = "info@redgold.io".to_string();
    let string = subject.into();
    let string1 = body.into();
    send_message(&client, to_address.clone(), &to_address, &string, &string1)
        .await.map_err(|e| error_info(e.to_string()))
}