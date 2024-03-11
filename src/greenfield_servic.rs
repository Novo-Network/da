use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use crate::greenfield_servic::gf_sdk_server::{create_obj, get_obj, put_obj, Req};
use crate::{DAService, DaType};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub rpc_addr: String,
    pub chain_id: String,
    pub bucket: String,
    pub private_key_path: String,
    pub gf_sdk_host: String,
}

#[allow(dead_code)]
pub struct GreenfieldService {
    rpc_addr: String,
    chain_id: String,
    bucket: String,
    private_key_path: String,
    gf_sdk_host: String,
}

impl GreenfieldService {
    pub fn new(cfg: Config) -> Self {
        Self {
            rpc_addr: cfg.rpc_addr,
            chain_id: cfg.chain_id,
            bucket: cfg.bucket,
            private_key_path: cfg.private_key_path,
            gf_sdk_host: cfg.gf_sdk_host,
        }
    }

    pub fn hash(tx: &[u8]) -> Vec<u8> {
        Keccak256::digest(tx).to_vec()
    }
}

#[async_trait]
impl DAService for GreenfieldService {
    async fn set_full_tx(&self, tx: &[u8]) -> Result<Vec<u8>> {
        // judge
        let hash = {
            let hash = self.hash(tx).await?;
            if let Ok(content) = self.get_tx(&hash).await {
                if !content.is_empty() {
                    return Ok(hash);
                }
            }
            hash
        };

        // gen url and req
        let (req, url) = {
            let obj = hex::encode(&hash);
            let url = format!("{}/object/{}/{}", self.gf_sdk_host, self.bucket, obj);
            let data = hex::encode(tx);
            let req = Req {
                data,
                content_type: "text/plain".to_string(),
                visibility: "private".to_string(),
                sync: true,
            };
            (req, url)
        };

        // create obj
        {
            let resp = create_obj(&req, &url).await?;
            println!("{:?}", resp);

            if resp.code != 0 {
                return Err(anyhow::Error::msg(format!(
                    "create obj fail: {}, id:[{}]",
                    resp.msg, resp.id
                )));
            }
        }

        // put obj
        {
            let resp = put_obj(&req, &url).await?;
            println!("{:?}", resp);

            if resp.code != 0 {
                return Err(anyhow::Error::msg(format!(
                    "put obj fail: {}, id:[{}]",
                    resp.msg, resp.id
                )));
            }
        }

        Ok(hash)
    }

    async fn get_tx(&self, hash: &[u8]) -> Result<Vec<u8>> {
        let obj = hex::encode(hash);

        // gen url
        let url = format!("{}/object/{}/{}", self.gf_sdk_host, self.bucket, obj);

        let resp = get_obj(&url).await?;

        Ok(resp)
    }

    fn type_byte(&self) -> u8 {
        DaType::Greenfield.type_byte()
    }

    async fn hash(&self, tx: &[u8]) -> Result<Vec<u8>> {
        Ok(Keccak256::digest(tx).to_vec())
    }
}

mod gf_sdk_server {
    use anyhow::Result;
    use futures::StreamExt;
    use reqwest::{Body, Client, Url};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Req {
        pub data: String,
        pub content_type: String,
        pub visibility: String,
        pub sync: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Resp {
        pub data: String,
        pub msg: String,
        pub code: i32,
        pub id: String,
    }

    pub async fn create_obj(req: &Req, url: &str) -> Result<Resp> {
        let cli = Client::new();
        let r = cli
            .post(Url::parse(url)?)
            .body(Body::from(serde_json::to_vec(req)?))
            .build()?;
        let resp = cli.execute(r).await?.json::<Resp>().await?;
        Ok(resp)
    }

    pub async fn put_obj(req: &Req, url: &str) -> Result<Resp> {
        let cli = Client::new();
        let r = cli
            .put(Url::parse(url)?)
            .body(Body::from(serde_json::to_vec(req)?))
            .build()?;
        let resp = cli.execute(r).await?.json::<Resp>().await?;
        Ok(resp)
    }

    pub async fn get_obj(url: &str) -> Result<Vec<u8>> {
        let resp = reqwest::get(Url::parse(url)?).await?;
        let mut stream = resp.bytes_stream();
        let mut data = vec![];

        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk?);
        }

        Ok(data)
    }
}
