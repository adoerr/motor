//! Simplex
//!
//! A simple consensus engine for block production and finalization
//!

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::{collections::HashMap, marker::PhantomData};

use codec::{Decode, Encode};
use derive_more::{AsRef, From, Into};
use futures::{future, FutureExt, StreamExt};
use log::{debug, info, warn};
use parking_lot::Mutex;

use sc_client_api::{Backend, BlockchainEvents, Finalizer};
use sc_network::PeerId;
use sc_network_gossip::{GossipEngine, Network, ValidationResult, Validator, ValidatorContext};

use sp_api::{ProvideRuntimeApi, TransactionFor};
use sp_application_crypto::RuntimePublic;
use sp_consensus::{
    import_queue::{BasicQueue, CacheKeyId, Verifier},
    BlockCheckParams, BlockImport, BlockImportParams, BlockOrigin, Environment,
    Error as ConsensusError, ForkChoiceStrategy, ImportResult, Proposal, Proposer, RecordProof,
    SelectChain, SyncOracle,
};
use sp_core::{sr25519, Pair};

use sp_runtime::{
    generic::DigestItem,
    traits::{Block as BlockT, Hash, Header},
    ConsensusEngineId, Justification,
};

pub const SIMPLEX_ENGINE_ID: ConsensusEngineId = *b"SPLX";
pub const SIMPLEX_PROTOCOL_NAME: &'static str = "/consensus/simplex/1";

#[derive(AsRef, Clone, From, Into)]
pub struct SimplexBlockAuthority(sr25519::Public);

#[derive(AsRef, From, Into)]
pub struct SimplexBlockAuthorityPair(sr25519::Pair);

#[derive(AsRef, Clone, From, Into)]
pub struct SimplexFinalityAuthority(sr25519::Public);

#[derive(AsRef, From, Into)]
pub struct SimplexFinalityAuthorityPair(sr25519::Pair);

/// Justification is basically a finality proof
#[derive(AsRef, Decode, Encode, From)]
struct SimplexJustification(sr25519::Signature);

/// Seal is basically a claim of origin
#[derive(AsRef, Decode, Encode, From)]
struct SimplexSeal(sr25519::Signature);

impl<Block> From<SimplexSeal> for DigestItem<Block> {
    fn from(seal: SimplexSeal) -> Self {
        DigestItem::Seal(SIMPLEX_ENGINE_ID, seal.encode())
    }
}

struct SimplexVerifier<B> {
    authority: SimplexBlockAuthority,
    _phantom: PhantomData<B>,
}

impl<B> SimplexVerifier<B>
where
    B: BlockT,
{
    fn check_header(&self, header: &mut B::Header) -> Result<SimplexSeal, String> {
        let seal = match header.digest_mut().pop() {
            Some(DigestItem::Seal(id, seal)) => {
                if id == SIMPLEX_ENGINE_ID {
                    SimplexSeal::decode(&mut &seal[..])
                        .map_err(|_| "Header with invalid seal".to_string())?
                } else {
                    return Err("Header seal wrong engine id".into());
                }
            }
            _ => return Err("Unsealed header".into()),
        };

        let pre_hash = header.hash();

        if !self.authority.as_ref().verify(&pre_hash, seal.as_ref()) {
            return Err("Invalid seal signature".into());
        }

        Ok(seal)
    }
}

impl<B> Verifier<B> for SimplexVerifier<B>
where
    B: BlockT,
{
    fn verify(
        &mut self,
        origin: BlockOrigin,
        mut header: B::Header,
        justification: Option<Justification>,
        body: Option<Vec<B::Extrinsic>>,
    ) -> Result<(BlockImportParams<B, ()>, Option<Vec<(CacheKeyId, Vec<u8>)>>), String> {
        let hash = header.hash();
        let seal = self.check_header(&mut header)?;

        let mut params = BlockImportParams::new(origin, header);
        params.body = body;
        params.post_digests.push(seal.into());
        params.post_hash = Some(hash);
        params.justification = justification;
        params.finalized = false;
        params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

        Ok((params, None))
    }
}

struct SimplexBlockImport<I, C> {
    import: I,
    finality_authority: SimplexFinalityAuthority,
    _phantom: PhantomData<C>,
}

impl<B, I, C> BlockImport<B> for SimplexBlockImport<I, C>
where
    B: BlockT,
    C: ProvideRuntimeApi<B>,
    I: BlockImport<B, Transaction = TransactionFor<C, B>>,
    I::Error: Into<ConsensusError>,
{
    type Error = ConsensusError;
    type Transaction = TransactionFor<C, B>;

    fn check_block(&mut self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
        self.import.check_block(block).map_err(Into::into)
    }

    fn import_block(
        &mut self,
        mut block: BlockImportParams<B, Self::Transaction>,
        cache: HashMap<CacheKeyId, Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        let justification = block
            .justification
            .take()
            .and_then(|j| SimplexJustification::decode(&mut &j[..]).ok());

        if let Some(justification) = justification {
            let hash = block
                .post_hash
                .as_ref()
                .expect("header has a seal; must have a post hash; qed");

            if self
                .finality_authority
                .as_ref()
                .verify(hash, justification.as_ref())
            {
                block.justification = Some(justification.encode());
                block.finalized = true;
            } else {
                warn!(target: "simplex", "📘 Invalid justification provided with block: {:?}", hash)
            }
        }

        self.import.import_block(block, cache).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct SimplexConfig {
    pub block_authority: SimplexBlockAuthority,
    pub finality_authority: SimplexFinalityAuthority,
}

pub type SimplexImportQueue<Block, Client> = BasicQueue<Block, TransactionFor<Client, Block>>;

pub fn import_queue<B, I, C>(
    config: SimplexConfig,
    import: I,
    _client: Arc<C>,
    spawner: &impl sp_core::traits::SpawnNamed,
) -> SimplexImportQueue<B, C>
where
    B: BlockT,
    I: BlockImport<B, Transaction = TransactionFor<C, B>> + Send + Sync + 'static,
    I::Error: Into<sp_consensus::Error>,
    C: ProvideRuntimeApi<B> + Send + Sync + 'static,
{
    let block_import = Box::new(SimplexBlockImport {
        import,
        finality_authority: config.finality_authority,
        _phantom: PhantomData::<C>,
    });

    let verifier = SimplexVerifier {
        authority: config.block_authority,
        _phantom: PhantomData,
    };

    BasicQueue::new(verifier, block_import, None, spawner, None)
}

/// Start `simplex` block authoring and import.
/// A new block will be authored and imported every 10 seconds.
pub fn start_simplex<B, C, SC, I, E, SO>(
    _client: Arc<C>,
    select_chain: SC,
    mut import: I,
    mut env: E,
    mut sync_oracle: SO,
    authority_key: SimplexBlockAuthorityPair,
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

    info!(target: "simplex", "📘 start simplex block authoring");

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
            debug!(target: "simplex", "📘 skip block proposal due to sync.");
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
            warn!(target: "simplex", "📘 failed to author block: {:?}", err);
        }

        thread::sleep(Duration::from_secs(BLOCK_TIME_SECS));
    });
}

struct AllowAll<Hash> {
    topic: Hash,
}

impl<B> Validator<B> for AllowAll<B::Hash>
where
    B: BlockT,
{
    fn validate(
        &self,
        _context: &mut dyn ValidatorContext<B>,
        _sender: &PeerId,
        _data: &[u8],
    ) -> ValidationResult<B::Hash> {
        ValidationResult::ProcessAndKeep(self.topic)
    }
}

#[derive(Decode, Encode)]
struct SimplexFinalityMessage<Hash> {
    block_hash: Hash,
    poof: SimplexJustification,
}

pub async fn start_simplex_finality_gadget<B, BE, C, N, SO>(
    _config: SimplexConfig,
    client: Arc<C>,
    network: N,
    mut sync_oracle: SO,
    _authority_key: Option<SimplexFinalityAuthorityPair>,
) where
    B: BlockT,
    BE: Backend<B>,
    C: BlockchainEvents<B> + Finalizer<B, BE> + Send + Sync,
    N: Network<B> + Clone + Send + 'static,
    SO: SyncOracle + Send + 'static,
{
    let topic = <<B::Header as Header>::Hashing as Hash>::hash("simplex".as_bytes());

    let gossip_engine = Arc::new(Mutex::new(GossipEngine::new(
        network,
        SIMPLEX_PROTOCOL_NAME,
        Arc::new(AllowAll { topic }),
        None,
    )));

    let mut listener = {
        let client = client.clone();

        gossip_engine
            .lock()
            .messages_for(topic)
            .for_each(move |notification| {
                if sync_oracle.is_major_syncing() {
                    debug!(target: "simplex", "📘 Ignoring finality notification due to ongoing sync.")
                }

                let message: SimplexFinalityMessage<B:Hash> = match Decode::decode(&mut &notification.message[..],) {
                    Ok(n) => n,
                    Err(err) => {
                        warn!(target: "simplex", "📘 Failed to decode gossip message: {:?}", err);
                        return future::ready(());
                    }
                };

                future::ready(())
            })
    };

    let mut gossip_engine = future::poll_fn(move |cx| gossip_engine.lock().poll_unpin(cx)).fuse();

    futures::select! {
        () = gossip_engine => {},
    }
}
