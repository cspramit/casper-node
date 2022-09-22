use crate::components::blocks_accumulator::LeapInstruction;
use crate::components::sync_leaper;
use crate::types::{ActivationPoint, Block, BlockHash, Chainspec, ChainspecRawBytes, NodeId};
use crate::{
    components::InitializedComponent,
    effect::{EffectBuilder, EffectExt, Effects},
    reactor::main_reactor::MainEvent,
};
use casper_execution_engine::core::engine_state::{ChainspecRegistry, UpgradeConfig};
use casper_hashing::Digest;
use casper_types::{EraId, Key, ProtocolVersion, StoredValue, Timestamp};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn initialize_component(
    effect_builder: EffectBuilder<MainEvent>,
    component: &mut impl InitializedComponent<MainEvent>,
    component_name: String,
    initiating_event: MainEvent,
) -> Option<Effects<MainEvent>> {
    if component.is_uninitialized() {
        let mut effects = effect_builder.immediately().event(|()| initiating_event);
        effects.extend(
            effect_builder
                .immediately()
                .event(|()| MainEvent::CheckStatus),
        );
        return Some(effects);
    }
    if component.is_fatal() {
        return Some(effect_builder.immediately().event(move |()| {
            MainEvent::Shutdown(format!("{} failed to initialize", component_name))
        }));
    }
    None
}

/// Check if protocol upgrade is necessary.
pub(super) fn maybe_upgrade(
    effect_builder: EffectBuilder<MainEvent>,
    block: &Block,
    chainspec: Arc<Chainspec>,
    chainspec_raw_bytes: Arc<ChainspecRawBytes>,
) -> Result<Option<Effects<MainEvent>>, String> {
    match chainspec.protocol_config.activation_point {
        ActivationPoint::Genesis(_) => Ok(None),
        ActivationPoint::EraId(era_id) => {
            if era_id != block.header().next_block_era_id() {
                return Ok(None);
            }
            match chainspec.ee_upgrade_config(
                *block.header().state_root_hash(),
                block.header().protocol_version(),
                era_id,
                chainspec_raw_bytes,
            ) {
                Ok(cfg) => {
                    let previous_block_header = Box::new(block.header().clone());
                    Ok(Some(
                        effect_builder
                            .upgrade_contract_runtime(Box::new(cfg))
                            .event(|result| MainEvent::UpgradeResult {
                                previous_block_header,
                                result,
                            }),
                    ))
                }
                Err(msg) => Err(msg),
            }
        }
    }
}

/// Check if running genesis is necessary.
pub(super) fn maybe_pre_genesis(
    effect_builder: EffectBuilder<MainEvent>,
    chainspec: Arc<Chainspec>,
    chainspec_raw_bytes: Arc<ChainspecRawBytes>,
) -> Result<Effects<MainEvent>, String> {
    match chainspec.protocol_config.activation_point {
        ActivationPoint::Genesis(timestamp) => {
            if Timestamp::now() > timestamp {
                Err("we are not pre-genesis".to_string())
            } else {
                Ok(effect_builder
                    .commit_genesis(chainspec, chainspec_raw_bytes)
                    .event(MainEvent::GenesisResult))
            }
        }
        ActivationPoint::EraId(_) => {
            Err("should not attempt genesis if not on a genesis chainspec".to_string())
        }
    }
}
