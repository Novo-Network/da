use std::str::FromStr;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Bytes, TransactionRequest, H160, H256},
    utils::keccak256,
};
use serde::{Deserialize, Serialize};

use crate::{service::DAService, DaType};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub url: String,
    pub to: String,
    pub sk: String,
}

pub struct EthereumService {
    provider: Provider<Http>,
    to: H160,
    wallet: LocalWallet,
}

impl EthereumService {
    pub async fn new(cfg: Config) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&cfg.url)?;
        let to = H160::from_str(&cfg.to)?;
        let wallet = {
            let chain_id = provider.get_chainid().await?.as_u64();
            let sk = cfg.sk.as_str().strip_prefix("0x").unwrap_or(&cfg.sk);
            let bytes = hex::decode(sk)?;
            LocalWallet::from_bytes(&bytes).map(|wallet| wallet.with_chain_id(chain_id))?
        };

        Ok(Self {
            provider,
            to,
            wallet,
        })
    }

    async fn build_tx(&self, data: &[u8]) -> Result<Bytes> {
        let nonce = self
            .provider
            .get_transaction_count(self.wallet.address(), None)
            .await?;

        let tx = TransactionRequest::new()
            .value(0)
            .from(self.wallet.address())
            .to(self.to)
            .nonce(nonce)
            .data(Bytes::from_iter(data));

        let mut tx = tx.into();
        self.provider.fill_transaction(&mut tx, None).await?;

        let signature = self.wallet.sign_transaction(&tx).await?;

        Ok(tx.rlp_signed(&signature))
    }
}

#[async_trait]
impl DAService for EthereumService {
    async fn hash(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let tx = self.build_tx(tx).await?;
        Ok(keccak256(tx).to_vec())
    }

    async fn set_full_tx(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let tx = self.build_tx(tx).await?;
        let hash = self.provider.send_raw_transaction(tx).await?.tx_hash();
        Ok(hash.0.to_vec())
    }

    async fn get_tx(&self, hash: &[u8]) -> Result<Vec<u8>> {
        let hash = H256::from_slice(hash);
        let tx = self
            .provider
            .get_transaction(hash)
            .await?
            .ok_or(anyhow!("data not found"))?;
        Ok(tx.input.to_vec())
    }

    fn type_byte(&self) -> u8 {
        DaType::Ethereum.type_byte()
    }
}
