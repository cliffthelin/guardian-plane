use guardian_core::error::{GuardianDbusError, GuardianErrorCategory};
use guardian_daemon::{GuardianContract, INTERFACE_NAME, OBJECT_PATH};
use guardian_testkit::PrivateSessionBus;
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use zbus::blocking::{Connection, Proxy, connection};

const EXPECTED_XML: &str =
    include_str!("../../../dbus/interfaces/org.guardianproject.Development.Guardian1.xml");
const GUARDIAN_INTERFACE_PREFIX: &str = "org.guardianproject.Development.";
const ERROR_PROBE_INTERFACE: &str = "org.guardianproject.Development.ErrorProbe1";
const ERROR_PROBE_PATH: &str = "/org/guardianproject/Development/ErrorProbe1";

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArgumentContract {
    name: Option<String>,
    signature: String,
    direction: Option<String>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MethodContract {
    name: String,
    arguments: Vec<ArgumentContract>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PropertyContract {
    name: String,
    signature: String,
    access: String,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SignalContract {
    name: String,
    arguments: Vec<ArgumentContract>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct InterfaceContract {
    name: String,
    methods: Vec<MethodContract>,
    properties: Vec<PropertyContract>,
    signals: Vec<SignalContract>,
}

type ContractTree = BTreeMap<String, Vec<InterfaceContract>>;

struct ErrorProbe {
    message: &'static str,
}

#[zbus::interface(name = "org.guardianproject.Development.ErrorProbe1")]
impl ErrorProbe {
    fn representative_failure(&self) -> Result<(), GuardianDbusError> {
        Err(GuardianErrorCategory::ProviderChanged.with_message(self.message))
    }
}

fn with_private_connection(test: impl FnOnce(&Connection)) {
    let bus = PrivateSessionBus::launch().expect("private D-Bus must launch");
    let connection = connection::Builder::address(bus.address())
        .expect("parse private D-Bus address")
        .build()
        .expect("connect to private D-Bus");
    test(&connection);
}

fn introspect(connection: &Connection, path: &str) -> String {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().expect("unique bus name").as_str(),
        path,
        "org.freedesktop.DBus.Introspectable",
    )
    .expect("create introspection proxy");
    proxy
        .call("Introspect", &())
        .expect("live introspection works")
}

fn required_attribute(node: Node<'_, '_>, attribute: &str) -> String {
    node.attribute(attribute)
        .unwrap_or_else(|| panic!("{} requires {attribute}", node.tag_name().name()))
        .to_owned()
}

fn argument_contract(node: Node<'_, '_>) -> ArgumentContract {
    ArgumentContract {
        name: node.attribute("name").map(str::to_owned),
        signature: required_attribute(node, "type"),
        direction: node.attribute("direction").map(str::to_owned),
    }
}

fn guardian_interfaces(node: Node<'_, '_>) -> Vec<InterfaceContract> {
    let mut interfaces = node
        .children()
        .filter(|child| child.has_tag_name("interface"))
        .filter(|interface| {
            interface
                .attribute("name")
                .is_some_and(|name| name.starts_with(GUARDIAN_INTERFACE_PREFIX))
        })
        .map(|interface| {
            let mut methods = interface
                .children()
                .filter(|child| child.has_tag_name("method"))
                .map(|method| MethodContract {
                    name: required_attribute(method, "name"),
                    arguments: method
                        .children()
                        .filter(|child| child.has_tag_name("arg"))
                        .map(argument_contract)
                        .collect(),
                })
                .collect::<Vec<_>>();
            let mut properties = interface
                .children()
                .filter(|child| child.has_tag_name("property"))
                .map(|property| PropertyContract {
                    name: required_attribute(property, "name"),
                    signature: required_attribute(property, "type"),
                    access: required_attribute(property, "access"),
                })
                .collect::<Vec<_>>();
            let mut signals = interface
                .children()
                .filter(|child| child.has_tag_name("signal"))
                .map(|signal| SignalContract {
                    name: required_attribute(signal, "name"),
                    arguments: signal
                        .children()
                        .filter(|child| child.has_tag_name("arg"))
                        .map(argument_contract)
                        .collect(),
                })
                .collect::<Vec<_>>();
            methods.sort();
            properties.sort();
            signals.sort();
            InterfaceContract {
                name: required_attribute(interface, "name"),
                methods,
                properties,
                signals,
            }
        })
        .collect::<Vec<_>>();
    interfaces.sort();
    interfaces
}

fn child_path(parent: &str, child: &str) -> String {
    if child.starts_with('/') {
        child.to_owned()
    } else {
        format!("{}/{child}", parent.trim_end_matches('/'))
    }
}

fn parse_expected_node(node: Node<'_, '_>, path: &str, tree: &mut ContractTree) {
    tree.insert(path.to_owned(), guardian_interfaces(node));
    for child in node.children().filter(|child| child.has_tag_name("node")) {
        let name = required_attribute(child, "name");
        let path = child_path(path, &name);
        parse_expected_node(child, &path, tree);
    }
}

fn expected_contract_tree() -> ContractTree {
    let document = Document::parse(EXPECTED_XML).expect("expected XML must parse");
    let root = document.root_element();
    assert_eq!(root.attribute("name"), Some(OBJECT_PATH));
    let mut tree = ContractTree::new();
    parse_expected_node(root, OBJECT_PATH, &mut tree);
    tree
}

fn introspection_node_xml(xml: &str) -> &str {
    &xml[xml
        .find("<node")
        .expect("introspection XML must contain a root node")..]
}

fn inspect_live_node(
    connection: &Connection,
    path: &str,
    visited: &mut HashSet<String>,
    tree: &mut ContractTree,
) {
    assert!(
        visited.insert(path.to_owned()),
        "object-tree cycle at {path}"
    );
    let xml = introspect(connection, path);
    let document =
        Document::parse(introspection_node_xml(&xml)).expect("live introspection XML must parse");
    let root = document.root_element();
    tree.insert(path.to_owned(), guardian_interfaces(root));
    for child in root.children().filter(|child| child.has_tag_name("node")) {
        let name = required_attribute(child, "name");
        inspect_live_node(connection, &child_path(path, &name), visited, tree);
    }
}

fn live_contract_tree(connection: &Connection) -> ContractTree {
    let mut tree = ContractTree::new();
    inspect_live_node(connection, OBJECT_PATH, &mut HashSet::new(), &mut tree);
    tree
}

fn method_error_name(error: &zbus::Error) -> &str {
    match error {
        zbus::Error::MethodError(name, _, _) => name.as_str(),
        other => panic!("expected structured D-Bus method error, got {other:?}"),
    }
}

fn assert_p0_dbus_001_live_export_matches_complete_expected_contract(connection: &Connection) {
    assert_eq!(live_contract_tree(connection), expected_contract_tree());
}

fn assert_p0_dbus_002_every_guardian_interface_has_terminal_major_one(connection: &Connection) {
    let tree = live_contract_tree(connection);
    let interfaces = tree.values().flatten().collect::<Vec<_>>();
    assert!(!interfaces.is_empty(), "Guardian must export an interface");
    for interface in interfaces {
        let terminal_component = interface.name.rsplit('.').next().unwrap();
        let first_digit = terminal_component
            .find(|character: char| character.is_ascii_digit())
            .expect("Guardian interface must end in an explicit major");
        assert_eq!(&terminal_component[first_digit..], "1");
        assert!(first_digit > 0, "major requires a named interface stem");
    }
}

fn assert_p0_dbus_003_entire_live_tree_has_exact_g0_method_allowlist(connection: &Connection) {
    let methods = live_contract_tree(connection)
        .into_values()
        .flatten()
        .flat_map(|interface| interface.methods)
        .map(|method| method.name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        methods,
        BTreeSet::from(["ContractVersion".to_owned(), "ServiceState".to_owned()])
    );
}

fn assert_p0_dbus_004_representative_typed_error_crosses_private_bus(connection: &Connection) {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().unwrap().as_str(),
        ERROR_PROBE_PATH,
        ERROR_PROBE_INTERFACE,
    )
    .unwrap();
    let error = proxy
        .call::<_, _, ()>("RepresentativeFailure", &())
        .unwrap_err();
    assert_eq!(
        method_error_name(&error),
        "org.guardianproject.Development.Guardian1.Error.ProviderChanged"
    );
}

fn assert_p0_dbus_005_unknown_method_is_structured_and_service_survives(connection: &Connection) {
    let proxy = Proxy::new(
        connection,
        connection.unique_name().unwrap().as_str(),
        OBJECT_PATH,
        INTERFACE_NAME,
    )
    .unwrap();
    let error = proxy
        .call::<_, _, ()>("DefinitelyUnknown", &())
        .unwrap_err();
    assert_eq!(
        method_error_name(&error),
        "org.freedesktop.DBus.Error.UnknownMethod"
    );
    let version: String = proxy.call("ContractVersion", &()).unwrap();
    assert_eq!(version, "1.0");
}

#[test]
fn p0_dbus_001_through_005_live_private_bus_contract_suite() {
    with_private_connection(|connection| {
        connection
            .object_server()
            .at(OBJECT_PATH, GuardianContract::default())
            .expect("register Guardian contract object");
        connection
            .object_server()
            .at(
                ERROR_PROBE_PATH,
                ErrorProbe {
                    message: "provider contract changed",
                },
            )
            .expect("register test-only typed error probe");

        assert_p0_dbus_002_every_guardian_interface_has_terminal_major_one(connection);
        assert_p0_dbus_003_entire_live_tree_has_exact_g0_method_allowlist(connection);
        assert_p0_dbus_001_live_export_matches_complete_expected_contract(connection);
        assert_p0_dbus_004_representative_typed_error_crosses_private_bus(connection);
        assert_p0_dbus_005_unknown_method_is_structured_and_service_survives(connection);
    });
}
