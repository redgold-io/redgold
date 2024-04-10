/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

#![allow(clippy::result_large_err)]

use crate::observability::send_email;
// snippet-end:[ses.rust.send-email]

#[ignore]
#[tokio::test]
async fn debug()  {
  send_email::email_from_to("Yo", "whats up", "info@redgold.io", "info@redgold.io").await.expect("Failed to send email");
}