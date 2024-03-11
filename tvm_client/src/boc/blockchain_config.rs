// Copyright 2018-2021 TON Labs LTD.
//
// Licensed under the SOFTWARE EVALUATION License (the "License"); you may not
// use this file except in compliance with the License.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific TON DEV software governing permissions and
// limitations under the License.

use tvm_block::Serializable;
use tvm_types::base64_encode;

use super::internal::deserialize_object_from_boc;
use crate::boc::Error;
use crate::client::ClientContext;
use crate::error::ClientResult;

#[derive(Serialize, Deserialize, Clone, ApiType, Default)]
pub struct ParamsOfGetBlockchainConfig {
    /// Key block BOC or zerostate BOC encoded as base64
    pub block_boc: String,
}

#[derive(Serialize, Deserialize, Clone, ApiType, Default)]
pub struct ResultOfGetBlockchainConfig {
    /// Blockchain config BOC encoded as base64
    pub config_boc: String,
}

/// Extract blockchain configuration from key block and also from zerostate.
#[api_function]
pub fn get_blockchain_config(
    context: std::sync::Arc<ClientContext>,
    params: ParamsOfGetBlockchainConfig,
) -> ClientResult<ResultOfGetBlockchainConfig> {
    let config = if let Ok(block) =
        deserialize_object_from_boc::<tvm_block::Block>(&context, &params.block_boc, "block")
    {
        extract_config_from_block(&block.object)?
    } else {
        let zerostate = deserialize_object_from_boc::<tvm_block::ShardStateUnsplit>(
            &context,
            &params.block_boc,
            "zerostate",
        )?;
        extract_config_from_zerostate(&zerostate.object)?
    };

    let cell =
        config.serialize().map_err(|err| Error::serialization_error(err, "config to cells"))?;

    let bytes = tvm_types::boc::write_boc(&cell)
        .map_err(|err| Error::serialization_error(err, "config cells to bytes"))?;

    Ok(ResultOfGetBlockchainConfig { config_boc: base64_encode(bytes) })
}

pub(crate) fn extract_config_from_block(
    block: &tvm_block::Block,
) -> ClientResult<tvm_block::ConfigParams> {
    let extra = block
        .read_extra()
        .map_err(|err| Error::invalid_boc(format!("can not read `extra` from block: {}", err)))?;

    let master = extra
        .read_custom()
        .map_err(|err| Error::invalid_boc(format!("can not read `master` from block: {}", err)))?
        .ok_or(Error::inappropriate_block(
            "not a masterchain block. Only key block contains blockchain configuration",
        ))?;

    Ok(master
        .config()
        .ok_or(Error::inappropriate_block(
            "not a key block. Only key block contains blockchain configuration",
        ))?
        .clone())
}

pub(crate) fn extract_config_from_zerostate(
    zerostate: &tvm_block::ShardStateUnsplit,
) -> ClientResult<tvm_block::ConfigParams> {
    let master = zerostate
        .read_custom()
        .map_err(|err| {
            Error::invalid_boc(format!("can not read `master` from zerostate: {}", err))
        })?
        .ok_or(Error::inappropriate_block(
            "not a masterchain state. Only masterchain states contain blockchain configuration",
        ))?;

    Ok(master.config().clone())
}