use aws_config::BehaviorVersion;
use lambda_runtime::{tracing, Error, LambdaEvent};
use aws_lambda_events::event::s3::S3Event;
use aws_sdk_bedrockruntime::primitives::Blob;
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, ConverseOutput, DocumentBlock, DocumentFormat, DocumentSource, Message};

const AGENT_RESPONSE_SCHEMA: &str = include_str!("agent-response-schema.json");

pub(crate)async fn function_handler(event: LambdaEvent<S3Event>) -> Result<(), Error> {
    let payload = event.payload;
    tracing::info!("payload: {:?}", payload);

    let bedrock_model = std::env::var("AWS_BEDROCK_MODEL").expect("AWS_BEDROCK_MODEL not set");
    let sqs_queue_url = std::env::var("AWS_SQS_QUEUE_URL").expect("AWS_SQS_QUEUE_URL not set");

    let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let bedrock_client = aws_sdk_bedrockruntime::Client::new(&aws_config);
    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config);

    let prompt = format!("Analyze this bill/invoice PDF and extract information from each one.

Return your response as a JSON object that conforms to this JSON schema:
{AGENT_RESPONSE_SCHEMA}

Rules:
- If you cannot determine the amount for a document DO NOT try to guess use 0
- If you cannot determine a date field for the document DO NOT try to guess use today's date
- If you cannot determine the value for any other property DO NOT try to guess use 'Unknown'
- Return ONLY a single-line, minified JSON object with no whitespace, no newlines, no indentation, and no additional text");

    for record in payload.records {
        let bucket = record.s3.bucket.name.expect("no s3 bucket name");
        let key = record.s3.object.key.expect("no s3 object key");

        let get_object_output = s3_client
            .get_object()
            .bucket(bucket)
            .key(key.clone())
            .send()
            .await?;

        let pdf_stream = get_object_output.body;
        let aggregated_bytes = pdf_stream.collect().await?;
        let raw_pdf = aggregated_bytes.into_bytes();

        let document_source = DocumentSource::Bytes(Blob::new(raw_pdf));

        let document_block = DocumentBlock::builder()
            .format(DocumentFormat::Pdf)
            .name(key)
            .source(document_source)
            .build()?;

        let document_content_block = ContentBlock::Document(document_block);

        let text_content_block = ContentBlock::Text(prompt.clone());

        let user_message = Message::builder()
            .role(ConversationRole::User)
            .content(text_content_block)
            .content(document_content_block)
            .build()?;

        let converse_output = bedrock_client
            .converse()
            .model_id(bedrock_model.clone())
            .messages(user_message)
            .send()
            .await?;

        let Some(output) = converse_output.output else {
            return Err("no response from bedrock".into());
        };

        let ConverseOutput::Message(message) = output else {
            return Err("unknown response from bedrock".into());
        };

        let Some(content) = message.content.first() else {
            return Err("no content from bedrock".into());
        };

        let ContentBlock::Text(schema) = content else {
            return Err("unknown content from bedrock".into());
        };

        sqs_client
            .send_message()
            .queue_url(sqs_queue_url.clone())
            .message_body(schema)
            .send()
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_runtime::{Context, LambdaEvent};

    #[tokio::test]
    async fn test_event_handler() {
        tracing::init_default_subscriber();
        let event = LambdaEvent::new(S3Event::default(), Context::default());
        function_handler(event).await.unwrap();
    }
}
