#![warn(missing_docs)]
//! Bevy game engine plugin for creating and searching discoverable servers on local networks

use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_utils::{
    hashbrown::hash_map::{EntryRef, Values},
    prelude::*,
    tracing::debug,
    StableHashMap,
};
use mdns_sd::{DaemonEvent, Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};

pub mod prelude {
    //! Prelude containing all types you need for making discoverable server and for discovering servers.
    pub use crate::{
        DiscoverableServer, DiscoveredServer, DiscoveredServerList, SearchServers,
        ServerBrowserPlugin,
    };
}

/// Resource that when added makes server available for discovery
/// on local network.
#[derive(Resource)]
pub struct DiscoverableServer {
    /// Arbitrary port that you want to report to clients to use.
    /// This is just information for clients, no binding or connecting
    /// happens with this port.
    pub port: u16,

    /// Additional metadata to be sent to clients. You can add information such
    /// as the user-facing name of a server, current level loaded on server,
    /// current number of players, etc.
    pub metadata: HashMap<String, String>,
}

/// Contains info about discovered server on local network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredServer {
    /// Hostname or name of a computer the server runs on.
    /// Useful when trying to distinguish multiple servers
    /// with same user-facing name
    pub hostname: String,

    /// Reported port that the client should use, see [`DiscoverableServer::port`]
    pub port: u16,

    /// Addresses the server is reachable on, you can try to connect to them in
    /// order or just use the first one
    pub addresses: HashSet<IpAddr>,

    /// Additional metadata received from server, see [`DiscoverableServer::metadata`]
    pub metadata: HashMap<String, String>,
}

/// Resource containing all servers discovered on local network.
#[derive(Resource)]
pub struct DiscoveredServerList(StableHashMap<String, DiscoveredServer>);

impl DiscoveredServerList {
    /// Returns true if there are no discovered servers
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the amount of discovered servers
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterates over all discovered servers. Note that [DiscoveredServerList]
    /// implements IntoIterator, meaning you can just use iterate over this
    /// without `.iter()`:
    /// ```
    /// fn system(servers: Res<DiscoveredServerList>) {
    ///     for server in &servers {
    ///         // do something with `server`
    ///     }
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &DiscoveredServer> {
        self.0.values().into_iter()
    }
}

impl<'a> IntoIterator for &'a DiscoveredServerList {
    type Item = &'a DiscoveredServer;
    type IntoIter = Values<'a, std::string::String, DiscoveredServer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.values().into_iter()
    }
}

/// Event that when emitted clears all discovered servers and starts new server
/// discovery. You can use this to do initial server search or do server list
/// refresh. Use this from system with [bevy::prelude::EventWriter]:
/// ```
/// fn system(e: EventWriter<SearchServers>) {
///     e.send_default();
/// }
/// ```
#[derive(Event, Default)]
pub struct SearchServers;

/// Plugin for servers and clients to discover each other.
/// Add this to bevy app to use server or client functionality.
pub struct ServerBrowserPlugin(String);

impl ServerBrowserPlugin {
    /// Create ServerBrowserPlugin
    ///
    /// `name` - A unique name that identifies your app so servers and clients
    /// can identify each other, needs to have same value on client and server.
    /// There are few restrictions on what is allowed as a name identifier:
    ///  - It can only contain ascii characters a-z and A-Z, numbers 0-9,
    ///    underscores `_` and hyphens `-`
    ///  - It must be at least one character long but no more than 15 characters
    ///    long
    ///  - Cannot start or end with hyphen or underscore
    ///  - Cannot have two consecutive hyphens or underscores.
    ///
    /// Most of the time you can use your crate name as a unique name (as long
    /// as it is not longer than 15 characters) like so:
    /// ```
    /// App::new()
    ///     .add_plugins(DefaultPlugins)
    ///     .add_plugins(ServerBrowserPlugin::new(env!("CARGO_PKG_NAME")))
    ///     .run();
    /// ```
    pub fn new(name: &str) -> Self {
        Self(validate_name(name))
    }
}

#[derive(Resource)]
struct Logger(Receiver<DaemonEvent>);

impl Plugin for ServerBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Service {
            name: self.0.clone(),
            daemon: ServiceDaemon::new().expect("Could not create service daemon"),
        });
        app.insert_resource(DiscoveredServerList(default()));
        app.add_event::<SearchServers>();
        app.add_systems(Startup, setup_logger);
        app.add_systems(
            PreUpdate,
            (
                register_server.run_if(resource_exists_and_changed::<DiscoverableServer>()),
                unregister_server
                    .run_if(resource_removed::<DiscoverableServer>())
                    .run_if(resource_exists::<ServiceFullname>()),
                search_servers,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                log_daemon_events.run_if(resource_exists::<Logger>()),
                update_discovered_servers.run_if(resource_exists::<Searching>()),
            ),
        );
    }
}

#[derive(Resource)]
struct Service {
    name: String,
    daemon: ServiceDaemon,
}

#[derive(Resource)]
struct ServiceFullname(String);

#[derive(Resource)]
struct Searching {
    browse: Receiver<ServiceEvent>,
}

fn update_discovered_servers(
    browsing: Res<Searching>,
    mut discovered_servers: ResMut<DiscoveredServerList>,
) {
    // this functions does comlicated mutation by inserting and merging found
    // servers that would trigger change detection even on accesses, we bypass
    // change change detection to be more accurate and do not mark it as changed
    // if it is not needed
    let servers = discovered_servers.bypass_change_detection();
    let mut changed = false;

    for event in browsing.browse.try_iter() {
        debug!("{:?}", event);

        match event {
            ServiceEvent::ServiceResolved(info) => {
                let hostname = info.get_hostname();

                let mut metadata = HashMap::new();
                for property in info.get_properties().iter() {
                    metadata.insert(property.key().to_string(), property.val_str().to_string());
                }

                let server = DiscoveredServer {
                    hostname: hostname
                        .strip_suffix(".local.")
                        .unwrap_or(hostname)
                        .to_string(),
                    port: info.get_port(),
                    addresses: info.get_addresses().to_owned(),
                    metadata,
                };

                match servers.0.entry_ref(info.get_fullname()) {
                    EntryRef::Occupied(mut entry) => {
                        if entry.get() != &server {
                            changed = true;
                            entry.insert(server);
                        }
                    }
                    EntryRef::Vacant(entry) => {
                        changed = true;
                        entry.insert(server);
                    }
                }
            }
            ServiceEvent::ServiceRemoved(_, fullname) => {
                changed = true;
                servers.0.remove(&fullname);
            }
            _ => {}
        }
    }

    if changed {
        discovered_servers.set_changed();
    }
}

fn search_servers(
    mut commands: Commands,
    service: Res<Service>,
    mut discovered_servers: ResMut<DiscoveredServerList>,
    mut search_servers_event: EventReader<SearchServers>,
) {
    if search_servers_event.is_empty() {
        return;
    }

    search_servers_event.clear();

    if !discovered_servers.is_empty() {
        discovered_servers.0.clear();
    }

    let service_type = format!("_{}._udp.local.", service.name);
    let browse = service
        .daemon
        .browse(&service_type)
        .expect("Failed to browse");
    commands.remove_resource::<Searching>();
    commands.insert_resource(Searching { browse });
}

fn setup_logger(mut commands: Commands, service: Res<Service>) {
    let monitor = service
        .daemon
        .monitor()
        .expect("Failed to monitor the daemon");

    commands.insert_resource(Logger(monitor));
}

fn log_daemon_events(logger: Res<Logger>) {
    for event in logger.0.try_iter() {
        debug!("{:?}", event);
    }
}

fn unregister_server(service_fullname: Res<ServiceFullname>, service: Res<Service>) {
    service
        .daemon
        .unregister(&service_fullname.0)
        .expect("Could not unregister service");
}

fn register_server(mut commands: Commands, server: Res<DiscoverableServer>, service: Res<Service>) {
    let service_type = format!("_{}._udp.local.", service.name);
    let instance_name = format!("{}", std::process::id());
    let service_hostname = format!("{}.local.", gethostname::gethostname().to_string_lossy());

    let service_info = ServiceInfo::new(
        &service_type,
        &instance_name,
        &service_hostname,
        "",
        server.port,
        server.metadata.clone(),
    )
    .expect("valid service info")
    .enable_addr_auto();

    let service_fullname = service_info.get_fullname().to_string();
    service
        .daemon
        .register(service_info)
        .expect("Failed to register mDNS service");

    commands.insert_resource(ServiceFullname(service_fullname));
}

fn validate_name(name: &str) -> String {
    let name = name.replace('_', "-");

    assert!(
        !name.starts_with('-'),
        "Name cannot start with hyphen or underscore"
    );

    assert!(
        !name.ends_with('-'),
        "Name cannot end with hyphen or underscore"
    );

    assert!(
        !name.contains("--"),
        "Name cannot contains double hyphens or double underscores"
    );

    assert!(name.len() <= 15, "Name cannot be longer than 15 bytes");
    assert!(name.len() > 0, "Name cannot be empty");

    // underscore is technically not allowed, but we allow it and convert it to hyphen
    // as it is common to use this character in unique id when provided id is crate name
    assert!(
        name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "Name can only contain a-zA-Z, hyphens ('-') and underscores ('_')"
    );

    assert!(
        name.bytes().any(|c| c.is_ascii_alphabetic()),
        "Name must contains at least one letter (a-zA-Z)"
    );

    name
}
