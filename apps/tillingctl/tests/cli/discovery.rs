use tilling_ipc::endpoint::SOCKET_VARIABLE;
use tilling_ipc::{Outcome, PROTOCOL_VERSION, Reply, Request, ServerMessage, VersionInfo};

use crate::support::{Runtime, reply, standard_error, standard_output};

fn named(request: &Request, compositor: &str) -> Vec<ServerMessage> {
    let result = Reply::Version(VersionInfo {
        compositor: compositor.to_owned(),
        protocol: PROTOCOL_VERSION,
    });
    vec![reply(request, Outcome::Success { result })]
}

fn alpha(request: &Request) -> Vec<ServerMessage> {
    named(request, "alpha")
}

fn beta(request: &Request) -> Vec<ServerMessage> {
    named(request, "beta")
}

#[test]
fn missing_instance_is_reported_without_guessing() {
    let runtime = Runtime::new();

    let output = runtime
        .tillingctl()
        .args(["get", "version"])
        .output()
        .unwrap();

    assert!(standard_error(&output).contains("no instance is running"));
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn several_instances_are_refused_until_one_is_chosen() {
    let runtime = Runtime::new();
    let _alpha = runtime.start("alpha", alpha);
    let beta = runtime.start("beta", beta);

    let ambiguous = runtime
        .tillingctl()
        .args(["get", "version"])
        .output()
        .unwrap();
    let by_name = runtime
        .tillingctl()
        .args(["--instance", "alpha", "get", "version"])
        .output()
        .unwrap();
    let by_socket = runtime
        .tillingctl()
        .arg("--socket")
        .arg(beta.socket())
        .args(["get", "version"])
        .output()
        .unwrap();
    let by_environment = runtime
        .tillingctl()
        .env(SOCKET_VARIABLE, beta.socket())
        .args(["get", "version"])
        .output()
        .unwrap();

    assert!(standard_error(&ambiguous).contains("several instances are running (alpha, beta)"));
    assert_eq!(ambiguous.status.code(), Some(3));
    assert_eq!(standard_output(&by_name), "alpha (protocol 1.0)\n");
    assert_eq!(standard_output(&by_socket), "beta (protocol 1.0)\n");
    assert_eq!(standard_output(&by_environment), "beta (protocol 1.0)\n");
}

#[test]
fn running_instances_are_listed_by_name() {
    let runtime = Runtime::new();
    let _alpha = runtime.start("alpha", alpha);
    let _beta = runtime.start("beta", beta);

    let text = runtime.tillingctl().arg("instances").output().unwrap();
    let json = runtime
        .tillingctl()
        .args(["instances", "--json"])
        .output()
        .unwrap();

    assert_eq!(standard_output(&text), "alpha\nbeta\n");
    assert_eq!(standard_output(&json), "[\"alpha\",\"beta\"]\n");
}

#[test]
fn unreachable_socket_names_the_path_it_tried() {
    let runtime = Runtime::new();

    let output = runtime
        .tillingctl()
        .args(["--socket", "/nonexistent/tilling/ipc.sock", "quit"])
        .output()
        .unwrap();

    assert!(standard_error(&output).contains("connect to /nonexistent/tilling/ipc.sock"));
    assert_eq!(output.status.code(), Some(3));
}
