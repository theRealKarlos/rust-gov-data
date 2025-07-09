# gov-data: UK Government Dataset Metadata to AWS S3

## Project Overview

gov-data is a Rust project that implements an AWS Lambda function to fetch metadata about datasets from the UK Government's CKAN data portal, serialize the metadata into a CSV file, and upload the resulting CSV to an AWS S3 bucket. This enables automated, serverless collection and storage of open government data for further analysis or downstream processing.

## How it Works

1. **Lambda Invocation:** The AWS Lambda function is triggered (optionally with a `test_mode` flag in the event payload).
2. **Dataset List Fetch:** The function fetches a list of dataset IDs from the CKAN API.
3. **Metadata Retrieval:** For each dataset ID, it fetches detailed metadata (title, description, license, organization, creation/modification dates, formats, and download URLs).
4. **CSV Generation:** All metadata is serialized and written to a CSV file.
5. **S3 Upload:** The CSV file is uploaded to a specified S3 bucket using the AWS SDK for Rust.

## Configuration

- **S3 Bucket:** The destination S3 bucket is hardcoded as `gov-data-lucky4some.com` in the source code (`src/main.rs`).
- **CSV File Name:** The output CSV file is named `DataGovUK_Datasets.csv`.
- **AWS Region:** Determined by the default AWS provider chain or falls back to `us-east-1`.
- **Test Mode:** You can limit the number of datasets processed by passing `{ "test_mode": true }` in the Lambda event payload. This is useful for local testing or CI.

## Usage

### Lambda Event Example

To run in test mode (processes only a small number of datasets):

```json
{
  "test_mode": true
}
```

### Output

- The resulting CSV file is uploaded to the configured S3 bucket under the key `DataGovUK_Datasets.csv`.

## Dependencies

- [aws-sdk-s3](https://docs.rs/aws-sdk-s3/) (AWS S3 integration)
- [aws-config](https://docs.rs/aws-config/) (AWS configuration)
- [lambda_runtime](https://docs.rs/lambda_runtime/) (AWS Lambda runtime)
- [reqwest](https://docs.rs/reqwest/) (HTTP client)
- [tokio](https://docs.rs/tokio/) (Async runtime)
- [csv](https://docs.rs/csv/) (CSV serialization)
- [serde, serde_json](https://serde.rs/) (JSON serialization)
- [regex](https://docs.rs/regex/) (HTML cleaning)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## Building

To build the project for production, run:

```bash
cargo lambda build --release
```

Remove the `--release` flag to build for development.

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`.

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
      "arn:aws:s3:::gov-data-lucky4some.com",
      "arn:aws:s3:::gov-data-lucky4some.com/*"
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
cargo lambda deploy --region eu-west-2
```

- The `--region` flag specifies the AWS region to deploy to (e.g., `eu-west-2` for London).
- To specify the function name, provide it as a positional argument:

  ```bash
  cargo lambda deploy --region eu-west-2 gov-data
  ```

- This will package your function, upload it to AWS, and create the Lambda function and IAM role if they do not exist.
- You may be prompted to select the AWS region and function name, or you can use flags and arguments to specify them.

### 3. Configure Environment Variables (Optional)

If you want to override any configuration (such as S3 bucket, CSV file name, CKAN API base URL, or concurrency), set environment variables in the AWS Lambda console or using the AWS CLI:

- Go to the AWS Lambda Console → Your Function → Configuration → Environment variables.
- Add variables like `BUCKET_NAME`, `CSV_FILE`, `CKAN_API_BASE_URL`, `CONCURRENCY_LIMIT`, etc.

### 4. Test the Lambda Function

You can test your Lambda function in two ways:

- **AWS Console:**
  - Go to the Lambda Console, select your function, and create a test event (e.g., `{ "test_mode": true }`).
- **AWS CLI:**
  - Run:
    ```bash
    aws lambda invoke --function-name <your-function-name> --payload '{"test_mode":true}' response.json
    ```
  - The output will be saved to `response.json`.

### 5. View Logs

- All logs from the function (including those from `tracing`) are available in AWS CloudWatch Logs for your Lambda function.
- Check CloudWatch for detailed execution logs and troubleshooting.

### Summary Table

| Step         | Command/Action                      |
| ------------ | ----------------------------------- |
| Build        | `cargo lambda build --release`      |
| Deploy       | `cargo lambda deploy`               |
| Set env vars | AWS Console or AWS CLI              |
| Test         | Console test or `aws lambda invoke` |
| View logs    | AWS CloudWatch Logs                 |

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).
