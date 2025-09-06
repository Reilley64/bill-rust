variable "function_name" {
  type = string
}

variable "source_dir" {
  type = string
}

variable "environment_variables" {
  type = map(string)
  default = {}
}
