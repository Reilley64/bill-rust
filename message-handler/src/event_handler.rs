use std::str::FromStr;
use lambda_runtime::{tracing, Error, LambdaEvent};
use aws_lambda_events::event::sqs::SqsEvent;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordAuthor {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordEmbed {
    author: DiscordAuthor,
    title: String,
    description: String,
    fields: Vec<DiscordField>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordMessage {
    username: String,
    embeds: Vec<DiscordEmbed>,
}

pub(crate)async fn function_handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    let payload = event.payload;
    tracing::info!("payload: {:?}", payload);

    let discord_webhook_url = std::env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL not set");

    let reqwest_client = reqwest::Client::new();

    for record in payload.records {
        let message = record.body;
        let Some(message) = message else {
            return Err("no message found".into());
        };

        let json = serde_json::Value::from_str(&message)?;

        let amount = json["amount"].as_f64().expect("no amount");

        let body = DiscordMessage {
            username: "Bill".to_string(),
            embeds: vec![
                DiscordEmbed {
                    author: DiscordAuthor { name: "New Bill".to_string() },
                    title: json["company"].as_str().expect("no company").to_string(),
                    description: json["subject"].as_str().expect("no subject").to_string(),
                    fields: vec![
                        DiscordField { name: "Due Date".to_string(), value: json["date"].as_str().expect("no date").to_string(), inline: true },
                        DiscordField { name: "Amount".to_string(), value: format!("${:.2}", amount), inline: true },
                        DiscordField { name: "Split".to_string(), value: format!("${:.2}", amount / 2.0), inline: true }
                    ],
                }
            ],
        };

        reqwest_client
            .post(discord_webhook_url.clone())
            .json(&body)
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
        let event = LambdaEvent::new(SqsEvent::default(), Context::default());
        function_handler(event).await.unwrap();
    }
}
