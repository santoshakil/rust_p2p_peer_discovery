use libp2p::{
    core::{upgrade, Transport},
    futures::StreamExt,
    mdns, noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let thread1 = tokio::task::spawn(async {
        if let Err(e) = start_peer().await {
            eprintln!("Error1: {}", e);
        }
    });

    let thread2 = tokio::task::spawn(async {
        if let Err(e) = start_peer().await {
            eprintln!("Error2: {}", e);
        }
    });

    tokio::try_join!(thread1, thread2)?;

    Ok(())
}

async fn start_peer() -> Result<(), Box<dyn std::error::Error>> {
    let id_keys = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = libp2p::PeerId::from(id_keys.public());
    println!("Local peer id: {local_peer_id}");

    let noise_config = noise::Config::new(&id_keys)?;
    let trns = tcp::tokio::Transport::default()
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(libp2p::yamux::Config::default())
        .boxed();
    let mut swarm = SwarmBuilder::with_tokio_executor(
        trns,
        mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
        local_peer_id,
    )
    .build();

    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    loop {
        let event = swarm.select_next_some().await;
        match event {
            SwarmEvent::Behaviour(mdns::Event::Discovered(list)) => {
                for (peer, _) in list {
                    println!("Discovered peer: {:?}", peer);
                }
            }
            _ => {
                eprintln!("Unhandled Swarm Event: {:?}", event);
            }
        }
    }
}
