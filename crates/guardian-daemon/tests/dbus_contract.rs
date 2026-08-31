use guardian_daemon::{GuardianContract, INTERFACE_NAME, OBJECT_PATH};
use guardian_testkit::PrivateSessionBus;
use std::sync::{LazyLock, Mutex};
use zbus::blocking::{Connection, Proxy, connection};

static PRIVATE_BUS_TEST: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn with_exported_contract(test: impl FnOnce(&Connection)) {
    // zbus creates executor threads per connection. Serializing these tests avoids
    // transient thread exhaustion in constrained CI while preserving bus isolation.
    let _guard = PRIVATE_BUS_TEST.lock().expect("private bus test lock");
    let bus = PrivateSessionBus::launch().expect("private D-Bus must launch");
    let connection = connection::Builder::address(bus.address())
        .expect("parse private D-Bus address")
        .build()
        .expect("connect to private D-Bus");
    connection
        .object_server()
        .at(OBJECT_PATH, GuardianContract::default())
        .expect("register Guardian contract object");
    test(&connection);
}

#[test]
fn p0_dbus_001_live_export_has_introspection() {
    with_exported_contract(|connection| {
        let proxy = Proxy::new(
            connection,
            connection.unique_name().expect("unique bus name").as_str(),
            OBJECT_PATH,
            "org.freedesktop.DBus.Introspectable",
        )
        .expect("create introspection proxy");
        let xml: String = proxy
            .call("Introspect", &())
            .expect("live introspection works");
        println!("{xml}");
        assert!(xml.contains(INTERFACE_NAME));
        assert!(xml.contains("ContractVersion"));
    });
}

#[test]
fn p0_dbus_002_every_guardian_interface_has_major_version() {
    with_exported_contract(|connection| {
        let proxy = Proxy::new(
            connection,
            connection.unique_name().unwrap().as_str(),
            OBJECT_PATH,
            "org.freedesktop.DBus.Introspectable",
        )
        .unwrap();
        let xml: String = proxy.call("Introspect", &()).unwrap();
        for line in xml
            .lines()
            .filter(|line| line.contains("org.guardianproject"))
        {
            assert!(
                line.contains("Guardian1"),
                "unversioned public interface: {line}"
            );
        }
    });
}

#[test]
fn p0_dbus_003_live_export_has_no_generic_execution_endpoint() {
    with_exported_contract(|connection| {
        let proxy = Proxy::new(
            connection,
            connection.unique_name().unwrap().as_str(),
            OBJECT_PATH,
            "org.freedesktop.DBus.Introspectable",
        )
        .unwrap();
        let xml: String = proxy.call("Introspect", &()).unwrap();
        let guardian_interface = xml
            .split(&format!("<interface name=\"{INTERFACE_NAME}\">"))
            .nth(1)
            .and_then(|tail| tail.split("</interface>").next())
            .expect("Guardian interface appears in live introspection");
        for forbidden in [
            "RunCommand",
            "RunShell",
            "ExecuteArbitrary",
            "ExecuteCommand",
            "argv",
            "shell",
            "command",
        ] {
            assert!(
                !guardian_interface
                    .to_ascii_lowercase()
                    .contains(&forbidden.to_ascii_lowercase()),
                "forbidden generic execution surface exported: {forbidden}"
            );
        }
        assert_eq!(guardian_interface.matches("<method name=").count(), 2);
    });
}

#[test]
fn p0_dbus_005_unknown_method_returns_error_and_service_survives() {
    with_exported_contract(|connection| {
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
        assert!(error.to_string().contains("UnknownMethod"));
        let version: String = proxy.call("ContractVersion", &()).unwrap();
        assert_eq!(version, "1.0");
    });
}
