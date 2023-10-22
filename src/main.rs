use std::time::Duration;

use libp2p::{
    core::{upgrade, Transport},
    futures::StreamExt,
    mdns, noise,
    swarm::SwarmEvent,
    tcp,
};

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

async fn start_peer(listen: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut swarm = build_swarm()?;
    if listen {
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    }

    loop {
        let event = swarm.select_next_some().await;
        match event {
            SwarmEvent::Behaviour(mdns::Event::Discovered(list)) => {
                for (peer, addr) in list {
                    println!("Discovered peer: {} with address {}", peer, addr);
                }
            }
            SwarmEvent::NewListenAddr { .. } => {}
            _ => {
                eprintln!("\n*** Unhandled Event: {:?} ***\n", event);
            }
        }
    }
}

fn build_swarm() -> Result<libp2p::Swarm<mdns::tokio::Behaviour>, Box<dyn std::error::Error>> {
    let id_keys = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = libp2p::PeerId::from(id_keys.public());
    println!("Local peer id: {local_peer_id}");
    let trns = tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(&id_keys)?)
        .multiplex(libp2p::yamux::Config::default())
        .boxed();
    let behaviour = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
    let config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(u64::MAX));
    let swarm = libp2p::Swarm::new(trns, behaviour, local_peer_id, config);
    Ok(swarm)
}
