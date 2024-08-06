use futures_util::StreamExt;
use reth::{
<<<<<<< HEAD
    network::{NetworkEvent, NetworkEvents, NetworkHandle, PeersInfo},
=======
    network::{NetworkEvent, NetworkEventListenerProvider, PeersHandleProvider, PeersInfo},
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
    rpc::types::PeerId,
};
use reth_network_peers::NodeRecord;
use reth_tokio_util::EventStream;
use reth_tracing::tracing::info;

/// Helper for network operations
#[derive(Debug)]
pub struct NetworkTestContext<Network> {
    network_events: EventStream<NetworkEvent>,
    network: Network,
}

impl<Network> NetworkTestContext<Network>
where
    Network: NetworkEventListenerProvider + PeersInfo + PeersHandleProvider,
{
    /// Creates a new network helper
    pub fn new(network: Network) -> Self {
        let network_events = network.event_listener();
        Self { network_events, network }
    }

    /// Adds a peer to the network node via network handle
    pub async fn add_peer(&mut self, node_record: NodeRecord) {
        self.network.peers_handle().add_peer(node_record.id, node_record.tcp_addr());

        match self.network_events.next().await {
            Some(NetworkEvent::PeerAdded(_)) => (),
            ev => panic!("Expected a peer added event, got: {ev:?}"),
        }
    }

    /// Returns the network node record
    pub fn record(&self) -> NodeRecord {
        self.network.local_node_record()
    }

    /// Awaits the next event for an established session.
    pub async fn next_session_established(&mut self) -> Option<PeerId> {
        while let Some(ev) = self.network_events.next().await {
            match ev {
                NetworkEvent::SessionEstablished { peer_id, .. } => {
                    info!("Session established with peer: {:?}", peer_id);
<<<<<<< HEAD
                    return Some(peer_id);
=======
                    return Some(peer_id)
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
                }
                _ => continue,
            }
        }
        None
    }
}
