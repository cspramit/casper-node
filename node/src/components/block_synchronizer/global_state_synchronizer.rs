use std::{
    collections::{btree_map::Entry, BTreeMap, HashSet},
    time::Duration,
};

use datasize::DataSize;
use derive_more::From;
use num_rational::Ratio;
use serde::Serialize;
use thiserror::Error;
use tracing::{debug, warn};

use casper_execution_engine::{core::engine_state, storage::trie::TrieRaw};
use casper_hashing::{ChunkWithProofVerificationError, Digest};

use super::{TrieAccumulator, TrieAccumulatorError, TrieAccumulatorEvent};
use crate::effect::announcements::BlocklistAnnouncement;
use crate::effect::requests::FetcherRequest;
use crate::{
    components::{fetcher::FetchResult, Component},
    effect::{
        requests::{ContractRuntimeRequest, SyncGlobalStateRequest, TrieAccumulatorRequest},
        EffectBuilder, EffectExt, Effects, Responder,
    },
    reactor,
    types::{BlockHash, BlockHashAndHeight, FetcherItem, Item, NodeId, TrieOrChunk},
    NodeRng,
};

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    TrieAccumulator(TrieAccumulatorError),
    #[error("ContractRuntime failed to put a trie into global state: {0}")]
    PutTrie(engine_state::Error),
}

#[derive(Debug, From, Serialize)]
pub(crate) enum Event {
    #[from]
    Request(SyncGlobalStateRequest),
    FetchedTrie {
        trie_hash: Digest,
        trie_accumulator_result: Result<Box<TrieRaw>, TrieAccumulatorError>,
    },
    PutTrieResult {
        trie_hash: Digest,
        #[serde(skip)]
        put_trie_result: Result<Vec<Digest>, engine_state::Error>,
    },
    #[from]
    TrieAccumulatorEvent(TrieAccumulatorEvent),
}

#[derive(Debug, DataSize)]
struct RequestState {
    missing_descendants: HashSet<Digest>,
    // TODO: Have one such set for all request states?
    in_flight: HashSet<Digest>,
    peers: HashSet<NodeId>,
    responder: Responder<Result<(), Error>>,
}

impl RequestState {
    fn new(request: SyncGlobalStateRequest) -> Self {
        let mut missing_descendants_for_current_block = HashSet::new();
        missing_descendants_for_current_block.insert(request.state_root_hash);
        Self {
            missing_descendants: missing_descendants_for_current_block,
            in_flight: HashSet::new(),
            peers: request.peers,
            responder: request.responder,
        }
    }

    fn add_missing_descendants(&mut self, missing_descendants: Vec<Digest>) {
        let descendants_to_add: Vec<_> = missing_descendants
            .into_iter()
            .filter(|descendant_hash| !self.in_flight.contains(descendant_hash))
            .collect();
        self.missing_descendants.extend(descendants_to_add);
    }

    /// Returns `true` if the given hash is known to be a missing descendant or an in flight
    /// request for this state.
    fn is_relevant(&self, trie_hash: &Digest) -> bool {
        self.in_flight.contains(&trie_hash) || self.missing_descendants.contains(&trie_hash)
    }
}

#[derive(Debug, DataSize)]
pub(super) struct GlobalStateSynchronizer {
    max_parallel_trie_fetches: usize,
    trie_accumulator: TrieAccumulator,
    request_states: BTreeMap<BlockHash, RequestState>,
}

impl GlobalStateSynchronizer {
    pub(super) fn new(max_parallel_trie_fetches: usize) -> Self {
        Self {
            max_parallel_trie_fetches,
            trie_accumulator: TrieAccumulator::new(),
            request_states: Default::default(),
        }
    }

    fn handle_request<REv>(
        &mut self,
        request: SyncGlobalStateRequest,
        effect_builder: EffectBuilder<REv>,
    ) -> Effects<Event>
    where
        REv: From<TrieAccumulatorRequest> + Send,
    {
        match self.request_states.entry(request.block_hash) {
            Entry::Vacant(entry) => {
                entry.insert(RequestState::new(request));
            }
            Entry::Occupied(entry) => {
                entry.into_mut().peers.extend(request.peers);
            }
        }

        self.parallel_fetch(effect_builder)
    }

    fn parallel_fetch<REv>(&mut self, effect_builder: EffectBuilder<REv>) -> Effects<Event>
    where
        REv: From<TrieAccumulatorRequest> + Send,
    {
        let mut effects = Effects::new();
        let mut finished = vec![];
        for (block_hash, request_state) in &mut self.request_states {
            // if there are no missing descendants and no tries in flight, we're finished
            if request_state.missing_descendants.is_empty() && request_state.in_flight.is_empty() {
                finished.push(*block_hash);
                continue;
            }

            // if we're not finished, figure out how many new fetching tasks we can start
            let num_fetches_to_start = self
                .max_parallel_trie_fetches
                .saturating_sub(request_state.in_flight.len());
            let mut requested_hashes = HashSet::new();

            for trie_hash in request_state
                .missing_descendants
                .iter()
                .filter(|trie_hash| !request_state.in_flight.contains(*trie_hash))
                .take(num_fetches_to_start)
                .cloned()
            {
                effects.extend(
                    effect_builder
                        .fetch_trie(trie_hash, request_state.peers.iter().copied().collect())
                        .event(move |trie_accumulator_result| Event::FetchedTrie {
                            trie_hash,
                            trie_accumulator_result,
                        }),
                );
                requested_hashes.insert(trie_hash);
            }

            request_state.in_flight.extend(requested_hashes);
            request_state.missing_descendants = request_state
                .missing_descendants
                .difference(&request_state.in_flight)
                .copied()
                .collect();
        }
        for block_hash in finished {
            effects.extend(self.finish_request(block_hash));
        }
        effects
    }

    fn handle_fetched_trie<REv>(
        &mut self,
        trie_hash: Digest,
        trie_accumulator_result: Result<Box<TrieRaw>, TrieAccumulatorError>,
        effect_builder: EffectBuilder<REv>,
    ) -> Effects<Event>
    where
        REv: From<TrieAccumulatorRequest> + From<ContractRuntimeRequest> + Send,
    {
        let trie_raw = match trie_accumulator_result {
            Ok(trie_raw) => trie_raw,
            Err(error) => {
                debug!(%error, "error fetching a trie");
                let canceled = self.affected_block_hashes(&trie_hash);
                return canceled
                    .into_iter()
                    .flat_map(|block_hash| {
                        self.cancel_request(block_hash, Error::TrieAccumulator(error.clone()))
                    })
                    .collect();
            }
        };

        // TODO - what if we got this from storage - should we rewrite it?
        effect_builder
            .put_trie_and_find_missing_descendant_trie_keys(*trie_raw)
            .event(move |put_trie_result| Event::PutTrieResult {
                trie_hash,
                put_trie_result,
            })
    }

    fn cancel_request(&mut self, block_hash: BlockHash, error: Error) -> Effects<Event> {
        match self.request_states.remove(&block_hash) {
            Some(request_state) => request_state.responder.respond(Err(error)).ignore(),
            None => Effects::new(),
        }
    }

    fn finish_request(&mut self, block_hash: BlockHash) -> Effects<Event> {
        match self.request_states.remove(&block_hash) {
            Some(request_state) => request_state.responder.respond(Ok(())).ignore(),
            None => Effects::new(),
        }
    }

    fn handle_put_trie_result<REv>(
        &mut self,
        trie_hash: Digest,
        put_trie_result: Result<Vec<Digest>, engine_state::Error>,
        effect_builder: EffectBuilder<REv>,
    ) -> Effects<Event>
    where
        REv: From<TrieAccumulatorRequest> + Send,
    {
        let mut effects = Effects::new();
        let block_hashes = self.affected_block_hashes(&trie_hash);
        match put_trie_result {
            Ok(missing_descendants) => {
                for block_hash in block_hashes {
                    if let Some(request_state) = self.request_states.get_mut(&block_hash) {
                        request_state.add_missing_descendants(missing_descendants.clone());
                        request_state.in_flight.remove(&trie_hash);
                        request_state.missing_descendants.remove(&trie_hash);
                    }
                }
            }
            Err(error) => {
                warn!(%trie_hash, %error, "couldn't put trie into global state");
                for block_hash in block_hashes {
                    effects.extend(self.cancel_request(block_hash, Error::PutTrie(error.clone())));
                }
            }
        }
        effects.extend(self.parallel_fetch(effect_builder));
        effects
    }

    fn affected_block_hashes(&self, trie_hash: &Digest) -> Vec<BlockHash> {
        self.request_states
            .iter()
            .filter(|(_, state)| state.is_relevant(&trie_hash))
            .map(|(block_hash, _)| *block_hash)
            .collect()
    }
}

impl<REv> Component<REv> for GlobalStateSynchronizer
where
    REv: From<TrieAccumulatorRequest>
        + From<ContractRuntimeRequest>
        + From<FetcherRequest<TrieOrChunk>>
        + From<BlocklistAnnouncement>
        + Send,
{
    type Event = Event;

    fn handle_event(
        &mut self,
        effect_builder: EffectBuilder<REv>,
        rng: &mut NodeRng,
        event: Self::Event,
    ) -> Effects<Self::Event> {
        match event {
            Event::Request(request) => self.handle_request(request, effect_builder),
            Event::FetchedTrie {
                trie_hash,
                trie_accumulator_result,
            } => self.handle_fetched_trie(trie_hash, trie_accumulator_result, effect_builder),
            Event::PutTrieResult {
                trie_hash,
                put_trie_result,
            } => self.handle_put_trie_result(trie_hash, put_trie_result, effect_builder),
            Event::TrieAccumulatorEvent(event) => reactor::wrap_effects(
                Event::TrieAccumulatorEvent,
                self.trie_accumulator
                    .handle_event(effect_builder, rng, event),
            ),
        }
    }
}
