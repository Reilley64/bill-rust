provider "aws" {
  region = "us-east-1"
}

resource "aws_s3_bucket" "mail_attachments" {}

resource "aws_sqs_queue" "messages" {
  visibility_timeout_seconds = 120
}

module "mail_handler" {
  source = "./modules/lambda"

  function_name = "mail-handler"
  source_dir    = "${path.module}/../mail-handler/target/lambda/mail-handler/"

  environment_variables = {
    AWS_S3_BUCKET = aws_s3_bucket.mail_attachments.bucket
  }
}

resource "aws_iam_policy" "mail_handler" {
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
          "s3:PutObjectAcl"
        ]
        Resource = "${aws_s3_bucket.mail_attachments.arn}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "workmailmessageflow:GetRawMessageContent"
        ]
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "mail_handler" {
  policy_arn = aws_iam_policy.mail_handler.arn
  role       = module.mail_handler.iam_role.name
}

resource "aws_lambda_permission" "mail_handler_workmail" {
  statement_id   = "AllowWorkMailInvoke"
  action         = "lambda:InvokeFunction"
  function_name  = module.mail_handler.function.function_name
  principal      = "workmail.us-east-1.amazonaws.com"
  source_account = "633193633564"
}

module "attachment_handler" {
  source = "./modules/lambda"

  function_name = "attachment-handler"
  source_dir    = "${path.module}/../attachment-handler/target/lambda/attachment-handler/"

  environment_variables = {
    AWS_BEDROCK_MODEL = "us.anthropic.claude-3-5-sonnet-20241022-v2:0",
    AWS_SQS_QUEUE_URL = aws_sqs_queue.messages.url
  }
}

resource "aws_iam_policy" "attachment_handler" {
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:GetObjectAcl"
        ]
        Resource = "${aws_s3_bucket.mail_attachments.arn}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "bedrock:InvokeModel"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "sqs:SendMessage"
        ]
        Resource = aws_sqs_queue.messages.arn
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "attachment_handler" {
  policy_arn = aws_iam_policy.attachment_handler.arn
  role       = module.attachment_handler.iam_role.name
}

resource "aws_lambda_permission" "attachment_handler_s3" {
  statement_id = "AllowS3Invoke"
  action = "lambda:InvokeFunction"
  function_name = module.attachment_handler.function.function_name
  principal = "s3.amazonaws.com"
  source_arn = aws_s3_bucket.mail_attachments.arn
}

resource "aws_s3_bucket_notification" "attachment_handler" {
  bucket = aws_s3_bucket.mail_attachments.id

  lambda_function {
    lambda_function_arn = module.attachment_handler.function.arn
    events = ["s3:ObjectCreated:*"]
  }

  depends_on = [aws_lambda_permission.attachment_handler_s3]
}

module "message_handler" {
  source = "./modules/lambda"

  function_name = "message-handler"
  source_dir    = "${path.module}/../message-handler/target/lambda/message-handler/"

  environment_variables = {
    DISCORD_WEBHOOK_URL = var.discord_webhook_url
  }
}

resource "aws_iam_policy" "message_handler" {
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sqs:ReceiveMessage",
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes"
        ]
        Resource = aws_sqs_queue.messages.arn
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "message_handler" {
  policy_arn = aws_iam_policy.message_handler.arn
  role       = module.message_handler.iam_role.name
}

resource "aws_lambda_event_source_mapping" "message_handler" {
  event_source_arn = aws_sqs_queue.messages.arn
  function_name    = module.message_handler.function.function_name
}

