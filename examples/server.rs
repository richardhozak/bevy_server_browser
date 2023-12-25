use bevy::{log::LogPlugin, prelude::*};
use bevy_server_browser::prelude::*;

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default()))
        // Add the server browser plugin
        .add_plugins(ServerBrowserPlugin::new("test_id"))
        .add_systems(Startup, setup_discoverable_server)
        .run();
}

fn setup_discoverable_server(mut commands: Commands) {
    info!("Adding discoverable server");
    commands.insert_resource(DiscoverableServer {
        name: "Test Server".to_string(),
        port: 1234,
    });
}
