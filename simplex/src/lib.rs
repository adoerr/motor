//! Simplex
//!
//! A simple consensus engine for block production and finalization
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use codec::{Decode, Encode};
use derive_more::{AsRef, From, Into};
use log::{debug, info, warn};

use sp_api::{ProvideRuntimeApi, TransactionFor};
use sp_consensus::{
    import_queue::BasicQueue, BlockImport, BlockImportParams, BlockOrigin, Environment,
    ForkChoiceStrategy, Proposal, Proposer, RecordProof, SelectChain, SyncOracle,
};
use sp_core::{sr25519, Pair};
use sp_runtime::{
    generic::DigestItem,
    traits::{Block as BlockT, Header},
    ConsensusEngineId,
};

pub const SIMPLEX_ENGINE_ID: ConsensusEngineId = *b"SPLX";
pub const SIMPLEX_PROTOCOL_NAME: &[u8] = b"/consensus/simplex/1";

#[derive(AsRef, Clone, From, Into)]
pub struct BlockAuthority(sr25519::Public);

#[derive(AsRef, From, Into)]
pub struct BlockAuthorityPair(sr25519::Pair);

#[derive(AsRef, Clone, From, Into)]
pub struct FinalityAuthority(sr25519::Public);

#[derive(AsRef, From, Into)]
pub struct FinalityAuthorityPair(sr25519::Pair);

/// Justification is basically a finality proof
#[derive(AsRef, Decode, Encode, From)]
struct Justification(sr25519::Signature);

/// Seal is basically a claim of origin
#[derive(AsRef, Debug, Encode, From)]
struct Seal(sr25519::Signature);

impl<Block> From<Seal> for DigestItem<Block> {
    fn from(seal: Seal) -> Self {
        DigestItem::Seal(SIMPLEX_ENGINE_ID, seal.encode())
    }
}

#[derive(Clone)]
pub struct Config {
    pub block_authority: sr25519::Public,
    pub finality_authority: sr25519::Public,
}

pub type ImportQueue<Block, Client> = BasicQueue<Block, TransactionFor<Client, Block>>;

pub fn import_queue<B, I, C>(
    _config: Config,
    _import: I,
    _client: Arc<C>,
    _spawner: &dyn sp_core::traits::SpawnNamed,
) -> ImportQueue<B, C>
where
    B: BlockT,
    I: BlockImport<B, Transaction = TransactionFor<C, B>> + Send + Sync + 'static,
    I::Error: Into<sp_consensus::Error>,
    C: ProvideRuntimeApi<B> + Send + Sync + 'static,
{
    unimplemented!()
}

/// Start `simplex` block authoring and import.
/// A new block will be authored and imported every 10 seconds.
pub fn start_simplex<B, C, SC, I, E, SO>(
    _client: Arc<C>,
    select_chain: SC,
    mut import: I,
    mut env: E,
    mut sync_oracle: SO,
    authority_key: BlockAuthorityPair,
) where
    B: BlockT,
    C: ProvideRuntimeApi<B> + Send + Sync + 'static,
    SC: SelectChain<B> + 'static,
    I: BlockImport<B, Transaction = TransactionFor<C, B>> + Send + Sync + 'static,
    I::Error: Into<sp_consensus::Error>,
    E: Environment<B> + Send + 'static,
    E::Proposer: Proposer<B, Transaction = TransactionFor<C, B>>,
    E::Error: std::fmt::Debug,
    SO: SyncOracle + Send + 'static,
{
    const BLOCK_TIME_SECS: u64 = 10;

    info!(target: "simplex", "🎭 start simplex block authoring");

    let mut propose_block = move || -> Result<Proposal<B, TransactionFor<C, B>>, String> {
        let best_header = select_chain
            .best_chain()
            .map_err(|err| format!("failed to pick best chain: {:?}", err))?;

        let proposer = futures::executor::block_on(env.init(&best_header))
            .map_err(|err| format!("failed to initialize proposer: {:?}", err))?;

        let inherent_data = Default::default();
        let inherent_digest = Default::default();

        let proposal = futures::executor::block_on(proposer.propose(
            inherent_data,
            inherent_digest,
            Duration::from_secs(BLOCK_TIME_SECS),
            RecordProof::No,
        ))
        .map_err(|err| format!("block proposal failed: {:?}", err))?;

        Ok(proposal)
    };

    let seal_block = move |header: &mut B::Header| {
        let seal = {
            let hash = header.hash();
            let seal = authority_key.as_ref().sign(hash.as_ref());
            DigestItem::<B::Hash>::Seal(SIMPLEX_ENGINE_ID, seal.encode())
        };

        header.digest_mut().push(seal);
        let post_hash = header.hash();
        let seal = header
            .digest_mut()
            .pop()
            .expect("pushed seal above; length greater than zero; qed");

        (post_hash, seal)
    };

    let mut author_block = move || -> Result<(), String> {
        if sync_oracle.is_major_syncing() {
            debug!(target: "singelton", "🔷 skip block proposal due to sync.");
        }

        let proposal = propose_block()?;
        let (mut header, body) = proposal.block.deconstruct();
        let (post_hash, seal) = seal_block(&mut header);

        let mut bip = BlockImportParams::new(BlockOrigin::Own, header);
        bip.post_digests.push(seal);
        bip.body = Some(body);
        bip.storage_changes = Some(proposal.storage_changes);
        bip.post_hash = Some(post_hash);
        bip.fork_choice = Some(ForkChoiceStrategy::LongestChain);

        import
            .import_block(bip, HashMap::default())
            .map_err(|err| format!("authored block import failed: {:?}", err))
            .map(|_| ())
    };

    thread::spawn(move || loop {
        if let Err(err) = author_block() {
            warn!(target: "singelton", "🔷 failed to author block: {:?}", err);
        }

        thread::sleep(Duration::from_secs(BLOCK_TIME_SECS));
    });
}
