use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub s3_region: String,
}

