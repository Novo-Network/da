use std::io::Cursor;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base58::{FromBase58, ToBase58};
use futures::TryStreamExt;
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use ipfs_api_backend_hyper as _;
use serde::{Deserialize, Serialize};

use crate::{service::DAService, DaType};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub url: String,
}

pub struct IpfsService {
    ipfs: IpfsClient,
}
impl IpfsService {
    pub fn new(cfg: Config) -> Result<Self> {
        Ok(Self {
            ipfs: IpfsClient::from_str(&cfg.url)?,
        })
    }
}

#[async_trait]
impl DAService for IpfsService {
    async fn set_full_tx(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let hash = self
            .ipfs
            .add(Cursor::new(tx.to_vec()))
            .await
            .map(|v| v.hash)?;
        self.ipfs.pin_add(&hash, true).await?;
        let hash = hash.from_base58().map_err(|e| anyhow!("{:?}", e))?;

        Ok(hash)
    }

    async fn get_tx(&self, hash: &[u8]) -> Result<Vec<u8>> {
        let key = hash.to_base58();
        let content = self
            .ipfs
            .cat(&key)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?;

        Ok(content)
    }

    fn type_byte(&self) -> u8 {
        DaType::Ipfs.type_byte()
    }
}
