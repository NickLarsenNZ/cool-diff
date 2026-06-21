use cool_diff::{ArrayMatchConfig, ArrayMatchMode, DiffConfig, MatchConfig, diff};
use indoc::indoc;
use serde::Deserialize;

/// A real-world Pod: a DNS server exposes port 53 on both UDP and TCP. Matching
/// container ports by `containerPort` alone is ambiguous, since two ports share
/// the same number. The (containerPort, protocol) pair is the true key. This is
/// the motivating case for composite key matching.
const DNS_POD_ACTUAL: &str = indoc! {"
    apiVersion: v1
    kind: Pod
    metadata:
      name: coredns
    spec:
      containers:
        - name: coredns
          image: coredns/coredns:1.11.1
          ports:
            - containerPort: 53
              protocol: UDP
            - containerPort: 53
              protocol: TCP
            - containerPort: 9153
              protocol: TCP
"};

/// Expected asserts the 53/TCP port carries a name that the actual port lacks,
/// so a correct (port, protocol) match produces a single Missing-field diff.
const DNS_POD_EXPECTED: &str = indoc! {"
    apiVersion: v1
    kind: Pod
    metadata:
      name: coredns
    spec:
      containers:
        - name: coredns
          ports:
            - containerPort: 53
              protocol: TCP
              name: dns-tcp
"};

fn parse(yaml: &str) -> serde_json::Value {
    serde_json::Value::deserialize(serde_yaml::Deserializer::from_str(yaml))
        .expect("valid test YAML")
}

#[test]
fn dns_pod_ports_match_by_port_and_protocol() {
    let actual = parse(DNS_POD_ACTUAL);
    let expected = parse(DNS_POD_EXPECTED);

    // Single-field key on containerPort is insufficient: ports 53/UDP and
    // 53/TCP both match containerPort 53, so this is an ambiguous match today.
    // The single container is paired up by index matching, so only the ports
    // path needs configuring.
    let config = DiffConfig::new().with_match_config(MatchConfig::new().with_config_at(
        "spec.containers.ports",
        ArrayMatchConfig::new(ArrayMatchMode::Key("containerPort".to_owned())),
    ));

    let tree = diff(&actual, &expected, &config).expect("ports should match by port and protocol");
    assert!(!tree.is_empty());
}
