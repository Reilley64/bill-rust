use aws_config::BehaviorVersion;
use lambda_runtime::{tracing, Error, LambdaEvent};
use aws_sdk_s3::primitives::ByteStream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkMailEvent {
    summary_version: String,
    subject: String,
    message_id: String,
    invocation_id: String,
    flow_direction: String,
}

pub(crate)async fn function_handler(event: LambdaEvent<WorkMailEvent>) -> Result<(), Error> {
    let payload = event.payload;
    tracing::info!("payload: {:?}", payload);

    let s3_bucket = std::env::var("AWS_S3_BUCKET").expect("AWS_S3_BUCKET not set");

    let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let workmail_client = aws_sdk_workmailmessageflow::Client::new(&aws_config);

    let message_content_output = workmail_client
        .get_raw_message_content()
        .message_id(payload.message_id)
        .send()
        .await?;

    let message_stream = message_content_output.message_content;
    let aggregated_bytes = message_stream.collect().await?;
    let raw_message = aggregated_bytes.into_bytes();
    let parsed = mailparse::parse_mail(&raw_message)?;

    let attachment = parsed.subparts
        .iter()
        .find(|x| x.headers
            .iter()
            .any(|x| x.get_key() == "Content-Type" && x.get_value().contains("application/pdf")));
    let Some(attachment) = attachment else {
        return Err("no attachment found".into());
    };

    let attachment_body = attachment.get_body_raw()?;
    let byte_stream = ByteStream::from(attachment_body);

    s3_client
        .put_object()
        .bucket(s3_bucket)
        .key(Uuid::now_v7().to_string())
        .content_type("application/pdf")
        .body(byte_stream)
        .send()
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_runtime::{Context, LambdaEvent};

    #[tokio::test]
    async fn test_event_handler() {
        tracing::init_default_subscriber();
        let event = LambdaEvent::new(WorkMailEvent::default(), Context::default());
        function_handler(event).await.unwrap();
    }
}
