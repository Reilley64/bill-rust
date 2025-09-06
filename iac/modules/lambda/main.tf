resource "aws_iam_role" "iam_role" {
  name = "${var.function_name}-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "basic_execution" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
  role       = aws_iam_role.iam_role.name
}

data "archive_file" "build" {
  type = "zip"
  source_dir = var.source_dir
  output_path = "${path.module}/${var.function_name}.zip"
}

resource "aws_lambda_function" "function" {
  filename = data.archive_file.build.output_path
  function_name = var.function_name
  role = aws_iam_role.iam_role.arn
  handler = "bootstrap"
  source_code_hash = data.archive_file.build.output_base64sha256
  timeout = "60"
  runtime = "provided.al2023"

  environment {
    variables = var.environment_variables
  }
}
