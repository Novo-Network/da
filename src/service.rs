use std::collections::BTreeMap;
use std::process::Command;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::DaType;
#[cfg(feature = "celestia")]
use crate::{CelestiaConfig, CelestiaService};
#[cfg(feature = "ethereum")]
use crate::{EthereumConfig, EthereumService};
#[cfg(feature = "file")]
use crate::{FileConfig, FileService};
#[cfg(feature = "greenfield")]
use crate::{GreenfieldConfig, GreenfieldService};
#[cfg(feature = "ipfs")]
use crate::{IpfsConfig, IpfsService};

#[async_trait]
pub trait DAService: Sync + Send {
    async fn set_full_tx(&self, tx: &[u8]) -> Result<Vec<u8>>;

    async fn get_tx(&self, hash: &[u8]) -> Result<Vec<u8>>;

    fn type_byte(&self) -> u8;

    async fn hash(&self, tx: &[u8]) -> Result<Vec<u8>>;

    async fn set_tx(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let hash = self.set_full_tx(tx).await?;

        let mut result = vec![self.type_byte()];

        result.extend_from_slice(&hash);

        Ok(result)
    }
}

pub struct DAServiceManager {
    services: BTreeMap<u8, Box<dyn DAService>>,
    default: u8,
}

impl DAServiceManager {
    pub async fn new(
        default: DaType,
        #[cfg(feature = "file")] file_cfg: Option<FileConfig>,
        #[cfg(feature = "ipfs")] ipfs_cfg: Option<IpfsConfig>,
        #[cfg(feature = "celestia")] celestia_cfg: Option<CelestiaConfig>,
        #[cfg(feature = "greenfield")] greenfield_cfg: Option<GreenfieldConfig>,
        #[cfg(feature = "ethereum")] ethereum_cfg: Option<EthereumConfig>,
    ) -> Result<Self> {
        match default {
            #[cfg(feature = "file")]
            DaType::File => {
                if file_cfg.is_none() {
                    return Err(anyhow!("file config not found"));
                }
            }
            #[cfg(feature = "ipfs")]
            DaType::Ipfs => {
                if ipfs_cfg.is_none() {
                    return Err(anyhow!("ipfs config not found"));
                }
            }
            #[cfg(feature = "celestia")]
            DaType::Celestia => {
                if celestia_cfg.is_none() {
                    return Err(anyhow!("celestia config not found"));
                }
            }
            #[cfg(feature = "greenfield")]
            DaType::Greenfield => {
                if greenfield_cfg.is_none() {
                    return Err(anyhow!("celestia config not found"));
                }
            }
            #[cfg(feature = "ethereum")]
            DaType::Ethereum => {
                if ethereum_cfg.is_none() {
                    return Err(anyhow!("ethereum config not found"));
                }
            }
        }
        let mut services: BTreeMap<u8, Box<dyn DAService>> = BTreeMap::new();

        #[cfg(feature = "file")]
        if let Some(cfg) = file_cfg {
            let service = FileService::new(cfg)?;
            services.insert(service.type_byte(), Box::new(service));
        }

        #[cfg(feature = "ipfs")]
        if let Some(cfg) = ipfs_cfg {
            let service = IpfsService::new(cfg)?;
            services.insert(service.type_byte(), Box::new(service));
        }

        #[cfg(feature = "celestia")]
        if let Some(cfg) = celestia_cfg {
            let service = CelestiaService::new(cfg).await?;
            services.insert(service.type_byte(), Box::new(service));
        }

        #[cfg(feature = "greenfield")]
        if let Some(cfg) = greenfield_cfg {
            let service = GreenfieldService::new(cfg.clone());
            services.insert(service.type_byte(), Box::new(service));

            // first kill if this exist
            Command::new("make")
                .args(["kill"])
                .current_dir("components/gf-sdk-server")
                .status()?;

            // start gf sdk server
            Command::new("gf-sdk-server")
                .args([
                    format!("-private_key_path={}", cfg.private_key_path),
                    format!("-host={}", cfg.gf_sdk_host),
                    format!("-chain_rpc={}", cfg.rpc_addr),
                    format!("-chain_id={}", cfg.chain_id),
                ])
                .current_dir("components/gf-sdk-server")
                .status()?;
        }

        #[cfg(feature = "ethereum")]
        if let Some(cfg) = ethereum_cfg {
            let service = EthereumService::new(cfg).await?;
            services.insert(service.type_byte(), Box::new(service));
        }

        Ok(Self {
            services,
            default: default.type_byte(),
        })
    }
    pub fn types(&self) -> Vec<u8> {
        self.services.keys().cloned().collect::<Vec<u8>>()
    }

    pub fn default_type(&self) -> u8 {
        self.default
    }

    pub async fn calc_hash(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let services = self
            .services
            .get(&self.default)
            .ok_or(anyhow!("default da not found"))?;
        services.hash(tx).await
    }

    pub async fn get_tx(&self, hash: impl Into<Vec<u8>>) -> Result<Vec<u8>> {
        let hash = hash.into();

        let type_byte = hash
            .first()
            .ok_or(anyhow!("Data length wrong, no type byte"))?;

        let service = self
            .services
            .get(type_byte)
            .ok_or(anyhow!("No target da service support"))?;

        service.get_tx(&hash[1..]).await
    }

    pub async fn set_tx(&self, tx: &[u8]) -> Result<Vec<u8>> {
        let service = self
            .services
            .get(&self.default)
            .ok_or(anyhow!("wrong service"))?;

        service.set_tx(tx).await
    }
}
