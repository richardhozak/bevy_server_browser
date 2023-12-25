use bevy::{log::LogPlugin, prelude::*};
use bevy_server_browser::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default()))
        // Add the server browser plugin
        .add_plugins(ServerBrowserPlugin::new("test_id"))
        .add_systems(Startup, discover_servers)
        .add_systems(
            Update,
            print_discovered_servers.run_if(resource_changed::<DiscoveredServerList>()),
        )
        .run();
}

fn discover_servers(mut search_servers: EventWriter<SearchServers>) {
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
