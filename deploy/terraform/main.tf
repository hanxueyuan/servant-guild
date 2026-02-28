# ServantGuild Infrastructure - Terraform Configuration
#
# This module provisions the complete infrastructure for ServantGuild
# including compute, networking, storage, and monitoring components.

terraform {
  required_version = ">= 1.5.0"
  
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.5"
    }
  }
  
  backend "s3" {
    bucket         = "servant-guild-terraform-state"
    key            = "servant-guild/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "servant-guild-terraform-locks"
  }
}

# =============================================================================
# Variables
# =============================================================================

variable "environment" {
  description = "Environment name (dev, staging, production)"
  type        = string
  default     = "production"
}

variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "instance_type" {
  description = "EC2 instance type for the host"
  type        = string
  default     = "t3.medium"
}

variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "redis_node_type" {
  description = "ElastiCache Redis node type"
  type        = string
  default     = "cache.t3.micro"
}

variable "allowed_cidr_blocks" {
  description = "CIDR blocks allowed to access the system"
  type        = list(string)
  default     = ["0.0.0.0/0"]  # Restrict in production
}

variable "github_token_secret_arn" {
  description = "ARN of Secrets Manager secret containing GitHub token"
  type        = string
}

variable "llm_api_keys_secret_arn" {
  description = "ARN of Secrets Manager secret containing LLM API keys"
  type        = string
}

# =============================================================================
# Provider Configuration
# =============================================================================

provider "aws" {
  region = var.region
  
  default_tags {
    tags = {
      Project     = "ServantGuild"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

# =============================================================================
# VPC and Networking
# =============================================================================

module "vpc" {
  source  = "./modules/vpc"
  
  environment         = var.environment
  vpc_cidr           = "10.0.0.0/16"
  availability_zones = ["${var.region}a", "${var.region}b", "${var.region}c"]
  
  public_subnet_cidrs  = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  private_subnet_cidrs = ["10.0.10.0/24", "10.0.11.0/24", "10.0.12.0/24"]
  
  enable_nat_gateway = true
  single_nat_gateway = var.environment != "production"
}

# =============================================================================
# Security Groups
# =============================================================================

resource "aws_security_group" "host" {
  name        = "servant-guild-host-${var.environment}"
  description = "Security group for ServantGuild host"
  vpc_id      = module.vpc.vpc_id
  
  # SSH access (restrict in production)
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidr_blocks
  }
  
  # HTTP API
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidr_blocks
  }
  
  # HTTPS API
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidr_blocks
  }
  
  # Metrics endpoint (internal only)
  ingress {
    from_port   = 9090
    to_port     = 9090
    protocol    = "tcp"
    cidr_blocks = [module.vpc.vpc_cidr_block]
  }
  
  # All outbound traffic
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "servant-guild-host-sg-${var.environment}"
  }
}

resource "aws_security_group" "database" {
  name        = "servant-guild-db-${var.environment}"
  description = "Security group for ServantGuild database"
  vpc_id      = module.vpc.vpc_id
  
  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.host.id]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "servant-guild-db-sg-${var.environment}"
  }
}

resource "aws_security_group" "redis" {
  name        = "servant-guild-redis-${var.environment}"
  description = "Security group for ServantGuild Redis"
  vpc_id      = module.vpc.vpc_id
  
  ingress {
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [aws_security_group.host.id]
  }
  
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = {
    Name = "servant-guild-redis-sg-${var.environment}"
  }
}

# =============================================================================
# Database (RDS PostgreSQL)
# =============================================================================

resource "aws_db_subnet_group" "main" {
  name       = "servant-guild-${var.environment}"
  subnet_ids = module.vpc.private_subnet_ids
  
  tags = {
    Name = "servant-guild-db-subnet-${var.environment}"
  }
}

resource "aws_db_instance" "main" {
  identifier = "servant-guild-${var.environment}"
  
  engine         = "postgres"
  engine_version = "15.4"
  instance_class = var.db_instance_class
  
  allocated_storage     = 100
  max_allocated_storage = 500
  storage_type          = "gp3"
  storage_encrypted     = true
  
  db_name  = "servant_guild"
  username = "servant_admin"
  password = random_password.db_password.result
  
  vpc_security_group_ids = [aws_security_group.database.id]
  db_subnet_group_name   = aws_db_subnet_group.main.name
  
  backup_retention_period = 7
  backup_window          = "03:00-04:00"
  maintenance_window     = "Mon:04:00-Mon:05:00"
  
  multi_az               = var.environment == "production"
  skip_final_snapshot    = var.environment != "production"
  final_snapshot_identifier = var.environment == "production" ? "servant-guild-${var.environment}-final" : null
  
  performance_insights_enabled = var.environment == "production"
  
  tags = {
    Name = "servant-guild-db-${var.environment}"
  }
}

resource "random_password" "db_password" {
  length  = 32
  special = false
}

# =============================================================================
# Redis (ElastiCache)
# =============================================================================

resource "aws_elasticache_subnet_group" "main" {
  name       = "servant-guild-${var.environment}"
  subnet_ids = module.vpc.private_subnet_ids
}

resource "aws_elasticache_replication_group" "main" {
  replication_group_id = "servant-guild-${var.environment}"
  description          = "ServantGuild Redis cluster"
  
  node_type            = var.redis_node_type
  num_cache_clusters   = var.environment == "production" ? 3 : 1
  subnet_group_name    = aws_elasticache_subnet_group.main.name
  security_group_ids   = [aws_security_group.redis.id]
  
  engine               = "redis"
  engine_version       = "7.0"
  parameter_group_name = "default.redis7"
  
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  
  automatic_failover_enabled = var.environment == "production"
  multi_az_enabled          = var.environment == "production"
  
  tags = {
    Name = "servant-guild-redis-${var.environment}"
  }
}

# =============================================================================
# Compute (EC2)
# =============================================================================

data "aws_ami" "amazon_linux" {
  most_recent = true
  owners      = ["amazon"]
  
  filter {
    name   = "name"
    values = ["amzn2-ami-hvm-*-x86_64-gp2"]
  }
  
  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_iam_role" "host" {
  name = "servant-guild-host-${var.environment}"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ec2.amazonaws.com"
      }
    }]
  })
}

resource "aws_iam_role_policy" "host" {
  name = "servant-guild-host-policy-${var.environment}"
  role = aws_iam_role.host.id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue",
          "secretsmanager:DescribeSecret"
        ]
        Resource = [
          var.github_token_secret_arn,
          var.llm_api_keys_secret_arn
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.artifacts.arn,
          "${aws_s3_bucket.artifacts.arn}/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents",
          "logs:DescribeLogStreams"
        ]
        Resource = "${aws_cloudwatch_log_group.main.arn}:*"
      },
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:PutMetricData"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "cloudwatch:namespace" = "ServantGuild/${var.environment}"
          }
        }
      }
    ]
  })
}

resource "aws_iam_instance_profile" "host" {
  name = "servant-guild-host-${var.environment}"
  role = aws_iam_role.host.name
}

resource "aws_instance" "host" {
  ami           = data.aws_ami.amazon_linux.id
  instance_type = var.instance_type
  
  subnet_id                   = module.vpc.public_subnet_ids[0]
  vpc_security_group_ids      = [aws_security_group.host.id]
  associate_public_ip_address = true
  
  iam_instance_profile = aws_iam_instance_profile.host.name
  
  root_block_device {
    volume_size           = 50
    volume_type          = "gp3"
    encrypted            = true
    delete_on_termination = true
  }
  
  user_data = base64encode(templatefile("${path.module}/user_data.sh", {
    environment        = var.environment
    db_endpoint       = aws_db_instance.main.endpoint
    redis_endpoint    = aws_elasticache_replication_group.main.primary_endpoint_address
    log_group_name    = aws_cloudwatch_log_group.main.name
    github_secret_arn = var.github_token_secret_arn
    llm_secret_arn    = var.llm_api_keys_secret_arn
  }))
  
  tags = {
    Name = "servant-guild-host-${var.environment}"
  }
  
  lifecycle {
    create_before_destroy = true
  }
}

# =============================================================================
# Storage (S3)
# =============================================================================

resource "aws_s3_bucket" "artifacts" {
  bucket = "servant-guild-artifacts-${var.environment}-${random_id.bucket_suffix.hex}"
  
  tags = {
    Name = "servant-guild-artifacts-${var.environment}"
  }
}

resource "random_id" "bucket_suffix" {
  byte_length = 4
}

resource "aws_s3_bucket_versioning" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  
  rule {
    id     = "cleanup-old-artifacts"
    status = "Enabled"
    
    expiration {
      days = 90
    }
    
    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }
}

# =============================================================================
# Monitoring (CloudWatch)
# =============================================================================

resource "aws_cloudwatch_log_group" "main" {
  name              = "/servant-guild/${var.environment}"
  retention_in_days = var.environment == "production" ? 30 : 7
}

resource "aws_cloudwatch_dashboard" "main" {
  dashboard_name = "servant-guild-${var.environment}"
  
  dashboard_body = jsonencode({
    widgets = [
      {
        type   = "metric"
        x      = 0
        y      = 0
        width  = 12
        height = 6
        
        properties = {
          title = "System Health"
          view  = "timeSeries"
          stacked = false
          metrics = [
            ["AWS/EC2", "CPUUtilization", { stat = "Average", period = 300 }],
            [".", "MemoryUtilization", { stat = "Average", period = 300 }],
            [".", "NetworkIn", { stat = "Sum", period = 300 }],
            [".", "NetworkOut", { stat = "Sum", period = 300 }]
          ]
        }
      },
      {
        type   = "metric"
        x      = 0
        y      = 6
        width  = 12
        height = 6
        
        properties = {
          title = "Database Performance"
          view  = "timeSeries"
          stacked = false
          metrics = [
            ["AWS/RDS", "CPUUtilization", { stat = "Average", period = 300 }],
            [".", "FreeableMemory", { stat = "Average", period = 300 }],
            [".", "ReadIOPS", { stat = "Sum", period = 300 }],
            [".", "WriteIOPS", { stat = "Sum", period = 300 }]
          ]
        }
      }
    ]
  })
}

resource "aws_cloudwatch_metric_alarm" "cpu_high" {
  alarm_name          = "servant-guild-cpu-high-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "CPUUtilization"
  namespace           = "AWS/EC2"
  period              = 300
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "This metric monitors EC2 CPU utilization"
  
  dimensions = {
    InstanceId = aws_instance.host.id
  }
  
  alarm_actions = [aws_sns_topic.alerts.arn]
}

resource "aws_cloudwatch_metric_alarm" "memory_high" {
  alarm_name          = "servant-guild-memory-high-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "MemoryUtilization"
  namespace           = "System/Linux"
  period              = 300
  statistic           = "Average"
  threshold           = 85
  alarm_description   = "This metric monitors EC2 memory utilization"
  
  dimensions = {
    InstanceId = aws_instance.host.id
  }
  
  alarm_actions = [aws_sns_topic.alerts.arn]
}

# =============================================================================
# Alerting (SNS)
# =============================================================================

resource "aws_sns_topic" "alerts" {
  name = "servant-guild-alerts-${var.environment}"
}

resource "aws_sns_topic_subscription" "email" {
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = "alerts@servantguild.dev"  # Configure appropriately
}

# =============================================================================
# Secrets (for reference, create externally)
# =============================================================================

# Note: These secrets should be created externally and passed via variables
# This is just for reference

# resource "aws_secretsmanager_secret" "github_token" {
#   name = "servant-guild/github-token"
# }

# resource "aws_secretsmanager_secret" "llm_api_keys" {
#   name = "servant-guild/llm-api-keys"
# }

# =============================================================================
# Outputs
# =============================================================================

output "host_public_ip" {
  description = "Public IP of the ServantGuild host"
  value       = aws_instance.host.public_ip
}

output "db_endpoint" {
  description = "RDS PostgreSQL endpoint"
  value       = aws_db_instance.main.endpoint
}

output "redis_endpoint" {
  description = "ElastiCache Redis endpoint"
  value       = aws_elasticache_replication_group.main.primary_endpoint_address
}

output "artifacts_bucket" {
  description = "S3 bucket for artifacts"
  value       = aws_s3_bucket.artifacts.bucket
}

output "sns_alerts_topic" {
  description = "SNS topic for alerts"
  value       = aws_sns_topic.alerts.arn
}
