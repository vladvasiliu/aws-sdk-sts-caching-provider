# aws-sdk-sts-caching-provider

This is a credential provider for use with [aws-sdk-rust](https://github.com/awslabs/aws-sdk-rust).
It sources the credentials from STS and caches them.

## Status

This project is in its early stages, but should be usable. It will probably be replaced by functionality in the AWS SDK
itself at some point.

## Usage

Reference the library in your `Cargo.toml`:

```toml
[dependencies]
aws-sdk-sts-caching-provider = { git = "https://github.com/vladvasiliu/aws-sdk-sts-caching-provider.git" }
```

In your code:

```rust
use aws_sdk_sts_caching_provider::STSCredentialsProvider;

async fn work() {
    let credential_provider = STSCredentialsProvider::new(
        role_arn,
        Some("external_id"),
        Some("source_identity"),
        Some("session_name"),
        Some(3600),                 // Session duration
        60,                         // Minimum remaining lifetime of the token before refresh
    );
    
    let config = aws_config::from_env().credentials_provider(credential_provider).load().await;
    let client = aws_sdk_health::client::Client::new(&config);
    client.do_something();
}
```

## Legal

The code is released under the terms of the Apache 2.0 License, which can be read in [LICENSE](LICENSE).

This project is in no way affiliated with AWS or the AWS SDK project.