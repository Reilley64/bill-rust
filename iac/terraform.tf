terraform {
  backend "s3" {
    bucket = "billrustterraformtest"
    key = "terraform.state"
    region = "ap-southeast-2"
    encrypt = true
    use_lockfile = true
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.92"
    }
  }

  required_version = ">= 1.2"
}
