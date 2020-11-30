use std::fmt::{self, Display, Formatter};

use datasize::DataSize;
use serde::{Deserialize, Serialize};

use super::{BlockHeight, DeployCollection, FinalizationQueue};
use crate::{types::Timestamp, Chainspec};

/// Stores the internal state of the BlockProposer.
#[derive(Clone, DataSize, Debug, Default, Deserialize, Serialize)]
pub struct BlockProposerDeploySets {
    /// The collection of deploys pending for inclusion in a block.
    pub(super) pending: DeployCollection,
    /// The deploys that have already been included in a finalized block.
    pub(super) finalized_deploys: DeployCollection,
    /// The next block height we expect to be finalized.
    /// If we receive a notification of finalization of a later block, we will store it in
    /// finalization_queue.
    /// If we receive a request that contains a later next_finalized, we will store it in
    /// request_queue.
    pub(super) next_finalized: BlockHeight,
    /// The queue of finalized block contents awaiting inclusion in `self.finalized_deploys`.
    pub(super) finalization_queue: FinalizationQueue,
}

impl Display for BlockProposerDeploySets {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "(pending:{}, finalized:{})",
            self.pending.len(),
            self.finalized_deploys.len()
        )
    }
}

/// Create a state storage key for block proposer deploy sets based on a chainspec.
///
/// We namespace based on a chainspec to prevent validators from loading data for a different chain
/// if they forget to clear their state.
pub fn create_storage_key(chainspec: &Chainspec) -> Vec<u8> {
    format!(
        "block_proposer_deploy_sets:version={},chain_name={}",
        chainspec.genesis.protocol_version, chainspec.genesis.name
    )
    .into()
}

impl BlockProposerDeploySets {
    /// Prunes expired deploy information from the BlockProposerState, returns the total deploys
    /// pruned
    pub(crate) fn prune(&mut self, current_instant: Timestamp) -> usize {
        let pending = prune_deploys(&mut self.pending, current_instant);
        let finalized = prune_deploys(&mut self.finalized_deploys, current_instant);
        pending + finalized
    }
}

/// Prunes expired deploy information from an individual DeployCollection, returns the total
/// deploys pruned
pub(super) fn prune_deploys(deploys: &mut DeployCollection, current_instant: Timestamp) -> usize {
    let initial_len = deploys.len();
    deploys.retain(|_hash, header| !header.expired(current_instant));
    initial_len - deploys.len()
}
