use std::time::Duration;

use libp2p::{
    core::{upgrade, Transport},
    futures::StreamExt,
    mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp,
};

#[derive(NetworkBehaviour)]
struct AppBehaviour {
    mdns: mdns::tokio::Behaviour,
    reqres: libp2p::request_response::cbor::Behaviour<String, String>,
}

async fn start_peer(listen: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut swarm = build_swarm()?;
    if listen {
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    }

    loop {
        let event = swarm.select_next_some().await;
        match event {
            SwarmEvent::Behaviour(AppBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer, addr) in list {
                    println!("\nDiscovered peer: {} with address {}", peer, addr);
                    swarm
                        .behaviour_mut()
                        .reqres
                        .send_request(&peer, "isMaster".to_string());
                }
            }
            SwarmEvent::Behaviour(AppBehaviourEvent::Reqres(v)) => match v {
                libp2p::request_response::Event::Message { peer, message } => match message {
                    libp2p::request_response::Message::Request {
                        request, channel, ..
                    } => {
                        println!("\nRequest from {:?}: {:?}", peer, request);
                        if request == "isMaster" {
                            _ = swarm
                                .behaviour_mut()
                                .reqres
                                .send_response(channel, "true".to_string());
                        }
                    }
                    libp2p::request_response::Message::Response { response, .. } => {
                        println!("\nResponse from {:?}: {:?}", peer, response);
                    }
                },
                _ => {}
            },
            SwarmEvent::NewListenAddr { .. } => {}
            _ => {
                eprintln!("\n*** Unhandled Event ***\n{:?}\n", event);
            }
        }
    }
}

fn build_swarm() -> Result<libp2p::Swarm<AppBehaviour>, Box<dyn std::error::Error>> {
    let id_keys = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = id_keys.public().to_peer_id();
    println!("\nkey: {:?}\nPeer: {local_peer_id}", id_keys.public());
    let trns = tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&id_keys)?)
        .multiplex(libp2p::yamux::Config::default())
        .boxed();
    let behaviour = AppBehaviour {
        mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
        reqres: libp2p::request_response::cbor::Behaviour::new(
            [(
                libp2p::StreamProtocol::new("/reqres/cbor/1.0.0"),
                libp2p::request_response::ProtocolSupport::Full,
            )],
            libp2p::request_response::Config::default(),
        ),
    };
    let config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(u64::MAX));
    let swarm = libp2p::Swarm::new(trns, behaviour, local_peer_id, config);
    Ok(swarm)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let thread1 = tokio::task::spawn(async {
        if let Err(e) = start_peer(true).await {
            eprintln!("Error1: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let thread2 = tokio::task::spawn(async {
        if let Err(e) = start_peer(false).await {
            eprintln!("Error2: {}", e);
        }
    });

    tokio::try_join!(thread1, thread2)?;

    Ok(())
}
