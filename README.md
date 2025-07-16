# Gov-Data: UK Government Dataset Metadata to AWS S3

> **Note:** This is a **lab project** intended for experimentation, learning, and demonstration purposes. It is not designed or maintained for production use. Use at your own risk.

## Project Overview

gov-data is a **professional-grade Rust project** that implements an AWS Lambda function to fetch metadata about datasets from the UK Government's CKAN data portal, serialise the metadata into a CSV file, and upload the resulting CSV to an AWS S3 bucket. This enables automated, serverless collection and storage of open government data for further analyses or downstream processing.

The project follows **Rust best practices** with a modular architecture, comprehensive error handling, type-safe deserialisation, resource optimisation, and thorough documentation.

## Architecture

The project is structured with a **modular design** for maintainability and testability:

```
src/
├── main.rs          - Lambda handler and orchestration logic
├── config.rs        - Centralised configuration management
├── error.rs         - Custom error types and conversions
├── ckan.rs          - CKAN API client with type-safe responses
├── csv_writer.rs    - CSV generation with dynamic URL columns
├── s3_upload.rs     - S3 upload with optimised buffering
└── tests.rs         - Unit tests for CKAN parsing
```

### Key Features

- **🔧 Centralised Configuration** - Environment variable overrides with sensible defaults
- **🛡️ Custom Error Types** - Proper error handling with AWS SDK compatibility
- **📦 Modular Design** - Clean separation of concerns across modules
- **🔒 Type-Safe Deserialisation** - Strongly typed CKAN API responses
- **⚡ Resource Optimisation** - Efficient HTTP client, compiled regex, connection pooling
- **✅ Configuration Validation** - Early validation with helpful error messages
- **📚 Comprehensive Documentation** - Detailed comments explaining implementation rationale

## How it Works

1. **Lambda Invocation:** The AWS Lambda function is triggered (optionally with a `test_mode` flag in the event payload).
2. **Configuration Loading:** Environment variables are loaded and validated with fallback defaults.
3. **Dataset List Fetch:** The function fetches a list of dataset IDs from the CKAN API using an optimised HTTP client.
4. **Metadata Retrieval:** For each dataset ID, it fetches detailed metadata (title, description, license, organisation, creation/modification dates, formats, and download URLs) with type-safe deserialisation.
5. **CSV Generation:** All metadata is serialised and written to a CSV file. Each download URL is written in its own column (download_url_1, download_url_2, etc.), with the number of columns determined by the dataset with the most URLs.
6. **S3 Upload:** The CSV file is uploaded to a specified S3 bucket using optimised buffering and the AWS SDK for Rust.

## Configuration

The project uses a **centralised configuration system** with environment variable overrides:

| Environment Variable | Default Value                                       | Description                  |
| -------------------- | --------------------------------------------------- | ---------------------------- |
| `BUCKET_NAME`        | `your-s3-bucket-name`                               | S3 bucket for CSV upload     |
| `CSV_FILE`           | `DataGovUK_Datasets.csv`                            | Output CSV filename          |
| `CKAN_API_BASE_URL`  | `https://ckan.publishing.service.gov.uk/api/action` | CKAN API base URL            |
| `CONCURRENCY_LIMIT`  | `10`                                                | Max concurrent HTTP requests |
| `AWS_REGION`         | `eu-west-2`                                         | AWS region (fallback)        |

### Configuration Validation

The configuration is validated at startup with helpful error messages for missing or invalid values. Invalid configurations cause the Lambda to exit early with descriptive error messages.

## Usage

### Lambda Event Example

To run in test mode (processes only the first 20 datasets for faster testing):

```json
{
  "test_mode": true
}
```

### Output

- The resulting CSV file is uploaded to the configured S3 bucket under the specified key.
- **CSV Format:** Each row contains the dataset metadata (id, title, description, license, organisation, created, modified, format), followed by one column for each download URL. The columns are named `download_url_1`, `download_url_2`, etc., up to the maximum number of URLs found in any dataset. If a dataset has fewer URLs, the extra columns are left empty.

## Dependencies

- [aws-sdk-s3](https://docs.rs/aws-sdk-s3/) (AWS S3 integration)
- [aws-config](https://docs.rs/aws-config/) (AWS configuration)
- [lambda_runtime](https://docs.rs/lambda_runtime/) (AWS Lambda runtime)
- [reqwest](https://docs.rs/reqwest/) (HTTP client with connection pooling)
- [tokio](https://docs.rs/tokio/) (Async runtime)
- [csv](https://docs.rs/csv/) (CSV serialisation)
- [serde, serde_json](https://serde.rs/) (JSON serialisation with type safety)
- [regex](https://docs.rs/regex/) (HTML cleaning with compiled patterns)
- [once_cell](https://docs.rs/once_cell/) (Static initialisation)
- [thiserror](https://docs.rs/thiserror/) (Error handling)
- [anyhow](https://docs.rs/anyhow/) (Additional error handling utilities)
- [futures](https://docs.rs/futures/) (Stream processing and async utilities)
- [tracing](https://docs.rs/tracing/) (Structured logging)
- [tracing-subscriber](https://docs.rs/tracing-subscriber/) (Logging configuration)
- [openssl](https://docs.rs/openssl/) (SSL/TLS support with vendored feature)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (for cross-compilation)

## Building

To build the project for AWS Lambda (production), you must use the cross compiler with Docker:

```bash
cargo lambda build --compiler cross --release
```

- This command uses Docker and the cross compiler to ensure compatibility with the Lambda environment.
- Make sure Docker Desktop is running before building.
- The cross compiler uses the Docker image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5`.

For local development (not for Lambda), you can use:

```bash
cargo lambda build
```

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`. The project includes comprehensive tests for:

- CKAN API response parsing
- Configuration validation
- Error handling scenarios

For integration tests or local Lambda invocation, use `cargo lambda watch` and `cargo lambda invoke`:

1. Start a local server:
   ```bash
   cargo lambda watch
   ```
2. Create a JSON file (e.g., `data.json`) with your test event:
   ```json
   { "test_mode": true }
   ```
3. Invoke the function locally:
   ```bash
   cargo lambda invoke --data-file ./data.json
   ```

Read more about running the local server in [the Cargo Lambda documentation for the `watch` command](https://www.cargo-lambda.info/commands/watch.html).
Read more about invoking the function in [the Cargo Lambda documentation for the `invoke` command](https://www.cargo-lambda.info/commands/invoke.html).

## Deploying

To deploy the project to AWS Lambda, follow these steps:

### Prerequisite: Docker Desktop

**Docker Desktop is required** to run the cross-compiler for AWS Lambda builds. Please ensure Docker Desktop is installed and running before building for Lambda.

- [Download Docker Desktop](https://www.docker.com/products/docker-desktop/)
- After installation, start Docker Desktop and ensure it is running in the background.
- The cross-compilation process uses the Docker image `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5` as the build environment. Docker will automatically pull this image if it is not already available locally.
- To ensure the image is available before building, you can manually pull it with:

  ```bash
  docker pull ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5
  ```

- The cross compiler is used to avoid issues compiling the OpenSSL dependency (and other native dependencies) for AWS Lambda. This ensures all dependencies are built in an environment compatible with Lambda, preventing common build and runtime errors.

### Important: S3 Bucket Region and IAM Permissions

- **Region:** Your Lambda function must be deployed in the **same AWS region** as your S3 bucket. If your bucket is in `eu-west-1`, deploy your Lambda to `eu-west-1`.
- **IAM Permissions:** The IAM role used by your Lambda function must have permissions to access your S3 bucket. At a minimum, it needs:

  - `s3:PutObject`
  - `s3:GetObject`
  - `s3:ListBucket`

  Example policy:

  ```json
  {
    "Effect": "Allow",
    "Action": ["s3:PutObject", "s3:GetObject", "s3:ListBucket"],
    "Resource": [
      "arn:aws:s3:::your-s3-bucket-name",
      "arn:aws:s3:::your-s3-bucket-name/*"
    ]
  }
  ```

- You can add this as an inline policy to your Lambda's execution role in the AWS IAM Console.

### 1. Build for AWS Lambda

First, build your project for the AWS Lambda target using the cross compiler:

```bash
cargo lambda build --compiler cross --release
```

This command uses Docker and the cross compiler to build your function for the Lambda environment and prepares it for deployment.

### 2. Deploy to AWS Lambda

Deploy your function using Cargo Lambda:

```bash
cargo lambda deploy --region eu-west-2 gov-data
```

- The `--region` flag specifies the AWS region to deploy to (e.g., `eu-west-2` for London).
- The function name `gov-data` is provided as a positional argument.
- This will package your function, upload it to AWS, and create the Lambda function and IAM role if they do not exist.

### 3. Configure Environment Variables (Optional)

If you want to override any configuration (such as S3 bucket, CSV file name, CKAN API base URL, or concurrency), set environment variables in the AWS Lambda console or using the AWS CLI:

- Go to the AWS Lambda Console → Your Function → Configuration → Environment variables.
- Add variables like `BUCKET_NAME`, `CSV_FILE`, `CKAN_API_BASE_URL`, `CONCURRENCY_LIMIT`, etc.

### 4. Test the Lambda Function

You can test your Lambda function in two ways:

- **AWS Console:**
  - Go to the Lambda Console, select your function, and create a test event (e.g., `{ "test_mode": true }`).
- **AWS CLI:**
  - Run (PowerShell, multi-line, no base64 required):
    ```powershell
    aws lambda invoke `
      --cli-binary-format raw-in-base64-out `
      --function-name "gov-data" `
      --payload '{"test_mode":true}' `
      response.json
    ```
    - Replace `gov-data` with your actual Lambda function name if different.
    - The output will be saved to `response.json`.

### Viewing Recent Logs for the Lambda (last 5 minutes)

After invoking your Lambda, you can fetch logs from the last 5 minutes using the AWS CLI:

**Bash (Linux/macOS):**

```bash
aws logs filter-log-events \
  --log-group-name "/aws/lambda/gov-data" \
  --start-time $(date -d "5 minutes ago" +%s)000 \
  --query 'events[*].{timestamp:timestamp,message:message}' \
  --output table
```

**PowerShell (Windows):**

```powershell
$startTime = [DateTimeOffset]::UtcNow.AddMinutes(-5).ToUnixTimeMilliseconds()
aws logs filter-log-events `
  --log-group-name "/aws/lambda/gov-data" `
  --start-time $startTime `
  --query 'events[*].{timestamp:timestamp,message:message}' `
  --output table
```

Or, to simply tail the last 5 minutes of logs (if you have AWS CLI v2):

```bash
aws logs tail /aws/lambda/gov-data --since 5m
```

Replace `gov-data` with your actual Lambda function name if different.

### 5. View Logs

- All logs from the function (including those from `tracing`) are available in AWS CloudWatch Logs for your Lambda function.
- Check CloudWatch for detailed execution logs and troubleshooting.

### Summary Table

| Step         | Command/Action                                    |
| ------------ | ------------------------------------------------- |
| Build        | `cargo lambda build --compiler cross --release`   |
| Deploy       | `cargo lambda deploy --region eu-west-2 gov-data` |
| Set env vars | AWS Console or AWS CLI                            |
| Test         | Console test or `aws lambda invoke`               |
| View logs    | AWS CloudWatch Logs                               |

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).

## Automated CI/CD Pipeline

This project includes a comprehensive **GitHub Actions CI/CD pipeline** that automates testing, building, deployment, and verification. The pipeline ensures code quality and reliable deployments to AWS Lambda.

### Pipeline Overview

The CI/CD pipeline consists of four sequential jobs:

1. **Quality Checks** - Unit tests, linting, formatting, and security audit
2. **Build Lambda** - Cross-compilation for AWS Lambda environment
3. **Deploy to AWS** - Automated deployment with environment configuration
4. **Post-Deploy Test** - Live Lambda function verification

### Branch-Based Deployment

| Branch        | Environment | Lambda Function    | Auto-Deploy   |
| ------------- | ----------- | ------------------ | ------------- |
| `main`        | Production  | `gov-data-prod`    | ✅ Yes        |
| `development` | Staging     | `gov-data-staging` | ✅ Yes        |
| `feature/*`   | None        | N/A                | ❌ Tests only |

### AWS OIDC Authentication

This pipeline uses **AWS OpenID Connect (OIDC)** for secure authentication without long-lived access keys.

**Required Setup:**

- AWS IAM role: `arn:aws:iam::123456789012:role/github-actions-role`
- **No AWS access keys or secrets required** - authentication handled automatically

**Optional GitHub Variables** (uses defaults if not set):

```
AWS_REGION                   # AWS region (default: eu-west-2)
LAMBDA_FUNCTION_NAME_PROD    # Production function name
LAMBDA_FUNCTION_NAME_STAGING # Staging function name
S3_BUCKET_PROD              # Production S3 bucket
S3_BUCKET_STAGING           # Staging S3 bucket
```

### Pipeline Features

- **🧪 Comprehensive Testing** - Unit tests, linting, and security scanning
- **🔒 Security First** - Dependency vulnerability scanning with `cargo audit`
- **⚡ Performance Optimised** - Caching and parallel execution
- **🚀 Automated Deployment** - Zero-touch deployment to AWS Lambda
- **✅ Post-Deploy Verification** - Live function testing with CloudWatch logs
- **📊 Full Observability** - Detailed logging and deployment tracking

### Usage

**Trigger Production Deployment:**

```bash
git push origin main
```

**Trigger Staging Deployment:**

```bash
git push origin development
```

**Run Tests Only (Pull Requests):**

```bash
# Create PR to main or development branch
# Pipeline runs quality checks but skips deployment
```

For detailed CI/CD documentation, see [CICD.md](CICD.md).

## Troubleshooting

### Windows Build Issues - Missing C++ Build Tools

If you encounter build errors on Windows related to missing C++ build tools, CMake, or Visual Studio generators, you may see errors like:

```
CMake Error: Generator Visual Studio 17 2022 could not find any instance of Visual Studio.
```

or

```
fatal error C1083: Cannot open include file: 'stdatomic.h': No such file or directory
```

**Solution:**

Install Visual Studio 2022 with C++ development tools:

1. **Download Visual Studio 2022 Build Tools or Community Edition:**

   - Build Tools: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   - Community (full IDE): https://visualstudio.microsoft.com/vs/community/

2. **During installation, ensure you select:**

   - "Desktop development with C++" workload
   - This includes the MSVC compiler, Windows SDK, and CMake tools

3. **Alternative using winget:**

   ```cmd
   winget install Microsoft.VisualStudio.2022.BuildTools
   ```

4. **After installation, restart your terminal and try building again:**
   ```bash
   cargo build
   ```

**Why this happens:**

- The AWS SDK dependencies (particularly `aws-lc-sys`) require native C++ compilation
- CMake looks for Visual Studio generators to compile native code
- Missing or outdated Visual Studio installations cause these build failures

### Other Common Issues

- **Docker not running:** Ensure Docker Desktop is running when using `cargo lambda build --compiler cross`
- **AWS credentials:** Verify AWS credentials are configured for deployment
- **Region mismatch:** Ensure Lambda and S3 bucket are in the same AWS region

## Error Handling

The project implements **comprehensive error handling** with custom error types that properly integrate with the AWS Lambda runtime and AWS SDK:

- **Configuration Errors** - Early validation with descriptive messages
- **Network Errors** - Proper handling of HTTP request failures
- **Serialisation Errors** - Graceful handling of malformed JSON responses
- **S3 Upload Errors** - Proper error propagation for upload failures

All errors are logged with appropriate context for debugging and monitoring.

## Performance Optimisations

The project includes several **performance optimisations** for the Lambda environment:

- **HTTP Connection Pooling** - Maintains up to 10 idle connections per host with 90-second timeout
- **Compiled Regex Patterns** - Pre-compiled patterns for HTML cleaning using `once_cell`
- **Optimised Buffering** - 8KB buffer size for efficient S3 uploads and memory usage
- **Concurrency Control** - Configurable limits (default: 10) to prevent resource exhaustion
- **Static Initialisation** - One-time setup of expensive resources like HTTP clients
- **Timeout Configuration** - 15-second HTTP timeouts with 10-second connection timeouts
- **TCP Keepalive** - 60-second keepalive intervals for persistent connections

## Code Quality

This project follows **Rust best practices** and professional development standards:

- **Modular Architecture** - Clean separation of concerns
- **Type Safety** - Strong typing throughout with proper deserialisation
- **Comprehensive Documentation** - Detailed comments explaining implementation rationale
- **Error Handling** - Custom error types with proper conversions
- **Testing** - Unit tests covering critical functionality
- **Resource Management** - Efficient use of Lambda resources

The codebase is production-ready and demonstrates professional Rust development practices suitable for enterprise environments.
