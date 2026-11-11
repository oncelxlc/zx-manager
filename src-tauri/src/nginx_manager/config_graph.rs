use super::dto::{
    NginxConfigGraph, NginxConfigGraphNode, NginxConfigGraphSource, NginxConfigNodeDetail,
    NginxConfigRevision, NginxConfiguration, NginxDirective,
};
use super::error::{NginxError, NginxResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const MAX_GRAPH_NODES: usize = 50_000;

pub fn build_config_graph(configuration: &NginxConfiguration) -> NginxResult<NginxConfigGraph> {
    let (graph, _) = build(configuration)?;
    Ok(graph)
}

pub fn read_config_node(
    configuration: &NginxConfiguration,
    node_id: &str,
) -> NginxResult<NginxConfigNodeDetail> {
    if node_id.len() != 64 || !node_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(node_not_found());
    }
    let (graph, raw_by_id) = build(configuration)?;
    let node = graph
        .nodes
        .into_iter()
        .find(|node| node.id == node_id)
        .ok_or_else(node_not_found)?;
    let raw = raw_by_id.get(node_id).cloned().ok_or_else(node_not_found)?;
    Ok(NginxConfigNodeDetail { node, raw })
}

fn build(
    configuration: &NginxConfiguration,
) -> NginxResult<(NginxConfigGraph, HashMap<String, String>)> {
    let revision = revision(configuration);
    let mut nodes = Vec::new();
    let mut raw_by_id = HashMap::new();
    for source in &configuration.sources {
        flatten_directives(
            &source.directives,
            &source.id,
            &source.include_chain,
            &[],
            &mut nodes,
            &mut raw_by_id,
        )?;
    }
    resolve_references(&mut nodes);
    let node_counts = nodes
        .iter()
        .fold(HashMap::<&str, usize>::new(), |mut counts, node| {
            *counts.entry(&node.source_id).or_default() += 1;
            counts
        });
    let sources = configuration
        .sources
        .iter()
        .map(|source| NginxConfigGraphSource {
            id: source.id.clone(),
            display_path: source.display_path.clone(),
            include_chain: source.include_chain.clone(),
            node_count: node_counts
                .get(source.id.as_str())
                .copied()
                .unwrap_or_default(),
        })
        .collect();
    Ok((
        NginxConfigGraph {
            instance_id: configuration.instance_id.clone(),
            entry_source_id: configuration.entry_source_id.clone(),
            revision,
            sources,
            nodes,
            diagnostics: configuration.diagnostics.clone(),
        },
        raw_by_id,
    ))
}

fn flatten_directives(
    directives: &[NginxDirective],
    source_id: &str,
    include_chain: &[String],
    ancestors: &[String],
    nodes: &mut Vec<NginxConfigGraphNode>,
    raw_by_id: &mut HashMap<String, String>,
) -> NginxResult<Vec<String>> {
    let mut sibling_counts = HashMap::<String, usize>::new();
    let mut ids = Vec::with_capacity(directives.len());
    for directive in directives {
        if nodes.len() >= MAX_GRAPH_NODES {
            return Err(NginxError::new(
                "NGINX_CONFIG_NODE_LIMIT",
                "configuration graph exceeds 50,000 nodes",
            ));
        }
        let signature = semantic_signature(directive);
        let occurrence = sibling_counts.entry(signature.clone()).or_default();
        let id = stable_id(source_id, ancestors, &signature, *occurrence);
        *occurrence += 1;
        let mut child_ancestors = ancestors.to_vec();
        child_ancestors.push(format!("{signature}#{}", occurrence.saturating_sub(1)));
        let child_ids = flatten_directives(
            &directive.children,
            source_id,
            include_chain,
            &child_ancestors,
            nodes,
            raw_by_id,
        )?;
        if nodes.len() >= MAX_GRAPH_NODES {
            return Err(NginxError::new(
                "NGINX_CONFIG_NODE_LIMIT",
                "configuration graph exceeds 50,000 nodes",
            ));
        }
        raw_by_id.insert(id.clone(), directive.raw.clone());
        nodes.push(NginxConfigGraphNode {
            id: id.clone(),
            kind: node_kind(&directive.name).to_owned(),
            name: directive.name.clone(),
            arguments: directive.arguments.clone(),
            label: node_label(directive),
            source_id: source_id.to_owned(),
            include_chain: include_chain.to_vec(),
            location: directive.location.clone(),
            child_ids,
            reference_ids: Vec::new(),
            known: is_known_directive(&directive.name),
        });
        ids.push(id);
    }
    Ok(ids)
}

fn revision(configuration: &NginxConfiguration) -> NginxConfigRevision {
    let mut hasher = Sha256::new();
    for source in &configuration.sources {
        hasher.update(source.text.as_bytes());
        hasher.update([0xff]);
    }
    NginxConfigRevision {
        value: format!("{:x}", hasher.finalize()),
        modified_at: configuration
            .sources
            .iter()
            .find(|source| source.id == configuration.entry_source_id)
            .and_then(|source| std::fs::metadata(&source.display_path).ok())
            .and_then(|metadata| metadata.modified().ok())
            .map(chrono::DateTime::<chrono::Utc>::from)
            .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    }
}

fn stable_id(source_id: &str, ancestors: &[String], signature: &str, occurrence: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_id.as_bytes());
    for ancestor in ancestors {
        hasher.update([0]);
        hasher.update(ancestor.as_bytes());
    }
    hasher.update([0xfe]);
    hasher.update(signature.as_bytes());
    hasher.update(occurrence.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

fn semantic_signature(directive: &NginxDirective) -> String {
    format!(
        "{}\u{1f}{}",
        directive.name,
        directive.arguments.join("\u{1f}")
    )
}

fn node_label(directive: &NginxDirective) -> String {
    if directive.arguments.is_empty() {
        directive.name.clone()
    } else {
        format!("{} {}", directive.name, directive.arguments.join(" "))
    }
}

fn node_kind(name: &str) -> &str {
    match name {
        "events" | "http" | "stream" | "server" | "location" | "upstream" | "map" => name,
        "include" => "include",
        _ => "directive",
    }
}

fn resolve_references(nodes: &mut [NginxConfigGraphNode]) {
    let upstreams = nodes
        .iter()
        .filter(|node| node.name == "upstream")
        .filter_map(|node| {
            node.arguments
                .first()
                .map(|name| (name.clone(), node.id.clone()))
        })
        .collect::<HashMap<_, _>>();
    for node in nodes {
        if node.name != "proxy_pass" {
            continue;
        }
        if let Some(target) = node
            .arguments
            .first()
            .and_then(|value| upstream_name(value))
        {
            if let Some(id) = upstreams.get(target) {
                node.reference_ids.push(id.clone());
            }
        }
    }
}

fn upstream_name(value: &str) -> Option<&str> {
    let value = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))?;
    value
        .split(['/', ':'])
        .next()
        .filter(|value| !value.is_empty())
}

fn is_known_directive(name: &str) -> bool {
    matches!(
        name,
        "accept_mutex"
            | "access_log"
            | "daemon"
            | "error_log"
            | "events"
            | "http"
            | "include"
            | "keepalive_timeout"
            | "listen"
            | "location"
            | "map"
            | "pid"
            | "proxy_pass"
            | "root"
            | "sendfile"
            | "server"
            | "server_name"
            | "stream"
            | "upstream"
            | "user"
            | "worker_connections"
            | "worker_processes"
    )
}

fn node_not_found() -> NginxError {
    NginxError::new(
        "NGINX_CONFIG_NODE_NOT_FOUND",
        "the configuration node does not exist in the current revision",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{NginxConfigSource, NginxSourceLocation};

    fn location() -> NginxSourceLocation {
        NginxSourceLocation {
            source_id: "source".to_owned(),
            byte_start: 0,
            byte_end: 10,
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 10,
        }
    }

    fn configuration(raw: &str) -> NginxConfiguration {
        NginxConfiguration {
            instance_id: "instance".to_owned(),
            entry_source_id: "source".to_owned(),
            sources: vec![NginxConfigSource {
                id: "source".to_owned(),
                display_path: "nginx.conf".to_owned(),
                include_chain: Vec::new(),
                text: raw.to_owned(),
                directives: vec![NginxDirective {
                    name: "worker_processes".to_owned(),
                    arguments: vec!["auto".to_owned()],
                    raw: "worker_processes auto;".to_owned(),
                    location: location(),
                    children: Vec::new(),
                }],
            }],
            diagnostics: Vec::new(),
            sites: Vec::new(),
            upstreams: Vec::new(),
            topology_nodes: Vec::new(),
            topology_edges: Vec::new(),
            pid_path: None,
            access_logs: Vec::new(),
            error_logs: Vec::new(),
        }
    }

    #[test]
    fn stable_id_does_not_depend_on_source_line_numbers() {
        let first =
            build_config_graph(&configuration("worker_processes auto;")).expect("first graph");
        let mut second_config = configuration("\n\nworker_processes auto;");
        second_config.sources[0].directives[0].location.line = 3;
        let second = build_config_graph(&second_config).expect("second graph");
        assert_eq!(first.nodes[0].id, second.nodes[0].id);
        assert_ne!(first.revision.value, second.revision.value);
    }

    #[test]
    fn rejects_unvalidated_node_ids() {
        let error = read_config_node(&configuration("events {}"), "../nginx.conf")
            .expect_err("invalid node id");
        assert_eq!(error.code, "NGINX_CONFIG_NODE_NOT_FOUND");
    }

    #[test]
    fn resolves_upstream_references() {
        let mut fixture = configuration("upstream api {} proxy_pass http://api;");
        fixture.sources[0].directives = vec![
            NginxDirective {
                name: "upstream".to_owned(),
                arguments: vec!["api".to_owned()],
                raw: "upstream api {}".to_owned(),
                location: location(),
                children: Vec::new(),
            },
            NginxDirective {
                name: "proxy_pass".to_owned(),
                arguments: vec!["http://api".to_owned()],
                raw: "proxy_pass http://api;".to_owned(),
                location: location(),
                children: Vec::new(),
            },
        ];
        let graph = build_config_graph(&fixture).expect("graph");
        let upstream = graph
            .nodes
            .iter()
            .find(|node| node.name == "upstream")
            .unwrap();
        let proxy = graph
            .nodes
            .iter()
            .find(|node| node.name == "proxy_pass")
            .unwrap();
        assert_eq!(
            proxy.reference_ids.as_slice(),
            std::slice::from_ref(&upstream.id)
        );
    }

    #[test]
    fn enforces_graph_node_limit() {
        let mut fixture = configuration("worker_processes auto;");
        let directive = fixture.sources[0].directives[0].clone();
        fixture.sources[0].directives = vec![directive; MAX_GRAPH_NODES + 1];
        let error = build_config_graph(&fixture).expect_err("node limit");
        assert_eq!(error.code, "NGINX_CONFIG_NODE_LIMIT");
    }
}
