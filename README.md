# bevy_server_browser

[![crates.io](https://img.shields.io/crates/v/bevy_server_browser)](https://crates.io/crates/bevy_server_browser)
[![docs.rs](https://docs.rs/bevy_server_browser/badge.svg)](https://docs.rs/bevy_server_browser)

Bevy game engine plugin for creating and searching discoverable servers on local networks.

This plugin does not provide any connection between server and clients, you need to pair it with network library, for example [bevy_matchbox](https://crates.io/crates/bevy_matchbox). This plugin only allows clients to discover servers and its info on local network, so you do not have to type ip addresses of servers into clients.

## Usage
See usage below or [examples](https://github.com/richardhozak/bevy_server_browser/tree/main/examples) for more comprehensive usage.

This example shows both server and client in one single app, meaning the client will discover itself, you can use both functionalities or just client or server.

```rust
use bevy::{log::LogPlugin, prelude::*};
use bevy_server_browser::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the server browser plugin
        .add_plugins(ServerBrowserPlugin::new("test_id"))
        .add_systems(
            Startup,
            // run discover servers after setup
            (setup_discoverable_server, discover_servers).chain(),
        )
        .add_systems(
            Update,
            print_discovered_servers.run_if(resource_changed::<DiscoveredServerList>()),
        )
        .run();
}

fn setup_discoverable_server(mut commands: Commands) {
    // add discoverable server as a resource which makes it available for discovery
    // on local network

    info!("Adding discoverable server");
    commands.insert_resource(DiscoverableServer {
        name: "Test Server".to_string(),
        port: 1234,
    });
}

fn discover_servers(mut search_servers: EventWriter<SearchServers>) {
    // send SearchServers event which will trigger search of discoverable servers
    // and update Res<DiscoverableServerList> accordingly
    search_servers.send_default();
}

fn print_discovered_servers(servers: Res<DiscoveredServerList>) {
    if servers.is_empty() {
        info!("No servers discovered");
        return;
    }

    info!("Discovered {} servers:", servers.len());
    for server in &servers {
        info!("{:?}", server);
    }
}

```

| bevy | bevy_server_browser |
| ---- | --------------------|
| 0.12 | 0.1.0               |
