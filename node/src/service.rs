
use std::sync::Arc;
use std::time::Duration;
use sc_client_api::ExecutorProvider;
use sc_consensus::LongestChain;
use node_template_runtime::{self, opaque::Block, RuntimeApi};

// use sc_service::{error::{Error as ServiceError}, AbstractService, Configuration, ServiceBuilder};
use sc_service::{
    config::Configuration,
    error::Error as ServiceError,
    RpcHandlers,
    TaskManager,
};

use sp_inherents::InherentDataProvider;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_finality_grandpa::{
	FinalityProofProvider as GrandpaFinalityProofProvider, StorageAndProofProvider, SharedVoterState,
};
// Our native executor instance.
native_executor_instance!(
	pub Executor,
	node_template_runtime::api::dispatch,
	node_template_runtime::native_version,
);
/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
	($config:expr) => {{
		use std::sync::Arc;

		let mut import_setup = None;
		let inherent_data_providers = sp_inherents::InherentDataProvider::new();
		let builder = sc_service::ServiceBuilder::new_full::<
			node_template_runtime::opaque::Block,
			node_template_runtime::RuntimeApi,
			crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_consensus::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|builder| {
				let pool_api = sc_transaction_pool::FullChainApi::new(
					builder.client().clone(),
				);
				Ok(sc_transaction_pool::BasicPool::new(
					builder.config().transaction_pool.clone(),
					std::sync::Arc::new(pool_api),
					builder.prometheus_registry(),
				))
			})?
			.with_import_queue(|
				_config,
				client,
				mut select_chain,
				_transaction_pool,
				spawn_task_handle,
				registry,
			| {
				let select_chain = select_chain.take()
					.ok_or_else(|| sc_service::Error::SelectChainRequired)?;
				let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
					client.clone(),
					&(client.clone() as Arc<_>),
					select_chain,
				)?;
				let (block_import, babe_link) = sc_consensus_babe::block_import(
					sc_consensus_babe::Config::get_or_compute(&*client)?,
					grandpa_block_import.clone(),
					client.clone(),
				)?;
				let justification_import = grandpa_block_import.clone();
				let import_queue = sc_consensus_babe::import_queue(
					babe_link.clone(),
					block_import.clone(),
					Some(Box::new(justification_import)),
					None,
					client,
					inherent_data_providers.clone(),
					spawn_task_handle,
					registry,
				)?;
				import_setup = Some((block_import, grandpa_link, babe_link));

				Ok(import_queue)
			})?;
		(builder, import_setup, inherent_data_providers)
	}}
}

/// Creates a full service from the configuration.
pub fn new_full_base(
    mut config: Configuration,
    with_startup_data: impl FnOnce(
        &sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
        &sc_consensus_babe::BabeLink<Block>,
    ),
) -> Result<NewFullBase, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: (rpc_extensions_builder, import_setup, rpc_setup),
    } = new_partial(&config)?;

    let shared_voter_state = rpc_setup;

    config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

    #[cfg(feature = "cli")]
    config.network.request_response_protocols.push(sc_finality_grandpa_warp_sync::request_response_config_for_chain(
        &config,
        task_manager.spawn_handle(),
        backend.clone(),
        import_setup.1.shared_authority_set().clone(),
    ));

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks = Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();

    let telemetry_span = TelemetrySpan::new();
    let _telemetry_span_entered = telemetry_span.enter();

    let (_rpc_handlers, telemetry_connection_notifier) = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        network: network.clone(),
        rpc_extensions_builder: Box::new(rpc_extensions_builder),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        on_demand: None,
        remote_blockchain: None,
        network_status_sinks: network_status_sinks.clone(),
        system_rpc_tx,
                // FIXME - why do i have to remove this? it appears to be required in Substrate 3.
        // telemetry_span: Some(telemetry_span.clone()),
    })?;

    let (block_import, grandpa_link, babe_link) = import_setup;

    (with_startup_data)(&block_import, &babe_link);

    if let sc_service::config::Role::Authority {
        ..
    } = &role
    {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
        );

        let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.sync_keystore(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: network.clone(),
            inherent_data_providers: inherent_data_providers.clone(),
            force_authoring,
            backoff_authoring_blocks,
            babe_link,
            can_author_with,
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;
        task_manager.spawn_essential_handle().spawn_blocking("babe-proposer", babe);
    }

        // Spawn authority discovery module.
    if role.is_authority() {
                let authority_discovery_role = sc_authority_discovery::Role::PublishAndDiscover(
            keystore_container.keystore(),
        );
                        let dht_event_stream = network.event_stream("authority-discovery")
            .filter_map(|e| async move { match e {
                Event::Dht(e) => Some(e),
                _ => None,
            }});
        let (authority_discovery_worker, _service) = sc_authority_discovery::new_worker_and_service(
            client.clone(),
                        network.clone(),
                                    Box::pin(dht_event_stream),
            authority_discovery_role,
            prometheus_registry.clone(),
        );

        task_manager.spawn_handle().spawn("authority-discovery-worker", authority_discovery_worker.run());
    }
        // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.sync_keystore())
    } else {
        None
    };

    let config = sc_finality_grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_period: 512,
        name: Some(name),
        observer_enabled: false,
        keystore,
        is_authority: role.is_authority(),
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = sc_finality_grandpa::GrandpaParams {
            config,
            link: grandpa_link,
            network: network.clone(),
            telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
            voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry,
            shared_voter_state,
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", sc_finality_grandpa::run_grandpa_voter(grandpa_config)?);
    }

    network_starter.start_network();
    Ok(NewFullBase {
        task_manager,
        inherent_data_providers,
        client,
        network,
        network_status_sinks,
        transaction_pool,
    })
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    new_full_base(config, |_, _| ()).map(
        |NewFullBase {
             task_manager,
             ..
         }| task_manager,
    )
}


pub fn new_light_base(
    mut config: Configuration,
) -> Result<
    (
        TaskManager,
        RpcHandlers,
        Option<TelemetryConnectionNotifier>,
        Arc<LightClient>,
        Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
        Arc<sc_transaction_pool::LightPool<Block, LightClient, sc_network::config::OnDemand<Block>>>,
    ),
    ServiceError,
> {
    let (client, backend, keystore_container, mut task_manager, on_demand) =
        sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

    config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
        config.transaction_pool.clone(),
        config.prometheus_registry(),
        task_manager.spawn_handle(),
        client.clone(),
        on_demand.clone(),
    ));

    let (grandpa_block_import, _) =
        sc_finality_grandpa::block_import(client.clone(), &(client.clone() as Arc<_>), select_chain.clone())?;
    let justification_import = grandpa_block_import.clone();

    let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::Config::get_or_compute(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let import_queue = sc_consensus_babe::import_queue(
        babe_link,
        babe_block_import,
        Some(Box::new(justification_import)),
        client.clone(),
        select_chain.clone(),
        inherent_data_providers.clone(),
        // TODO - find out why in Substrate 3 they use
        // &task_manager.spawn_essential_handle(), but it doesn't work
        &task_manager.spawn_handle(),
        config.prometheus_registry(),
        sp_consensus::NeverCanAuthor,
    )?;

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: Some(on_demand.clone()),
            block_announce_validator_builder: None,
        })?;
    network_starter.start_network();

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let light_deps = node_template_rpc::LightDeps {
        remote_blockchain: backend.remote_blockchain(),
        fetcher: on_demand.clone(),
        client: client.clone(),
        pool: transaction_pool.clone(),
    };

    let rpc_extensions = node_template_rpc::create_light(light_deps);

    let telemetry_span = TelemetrySpan::new();
    let _telemetry_span_entered = telemetry_span.enter();

    let (rpc_handlers, telemetry_connection_notifier) = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        on_demand: Some(on_demand),
        remote_blockchain: Some(backend.remote_blockchain()),
        rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        keystore: keystore_container.sync_keystore(),
        config,
        backend,
        network_status_sinks,
        system_rpc_tx,
        network: network.clone(),
        task_manager: &mut task_manager,
        // FIXME - why do i have to remove this? it appears to be required in Substrate 3.
        // telemetry_span: Some(telemetry_span.clone()),
    })?;

    Ok((task_manager, rpc_handlers, telemetry_connection_notifier, client, network, transaction_pool))
}


/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
    new_light_base(config).map(|(task_manager, ..)| task_manager)
}