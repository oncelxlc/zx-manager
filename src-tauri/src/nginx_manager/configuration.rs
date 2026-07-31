use super::dto::{
    NginxConfigDiagnostic, NginxConfigSource, NginxConfiguration, NginxDirective,
    NginxInstanceRecord, NginxSite, NginxSourceLocation, NginxTopologyEdge, NginxTopologyNode,
    NginxUpstream,
};
use super::error::{NginxError, NginxResult};
use super::manager::reject_reparse_points;
use glob::glob;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 2_048;
const MAX_INCLUDE_DEPTH: usize = 32;

#[derive(Clone, Debug)]
enum TokenKind {
    Word(String),
    LeftBrace,
    RightBrace,
    Semicolon,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

pub fn load_configuration(record: &NginxInstanceRecord) -> NginxResult<NginxConfiguration> {
    let entry = record.config_path.as_ref().ok_or_else(|| {
        NginxError::new(
            "NGINX_CONFIG_NOT_FOUND",
            "the instance has no authorized config path",
        )
    })?;
    let roots = record
        .authorized_roots
        .iter()
        .filter_map(|root| Path::new(root).canonicalize().ok())
        .collect::<Vec<_>>();
    let mut resolver = Resolver::new(roots);
    let entry_path = Path::new(entry)
        .canonicalize()
        .map_err(|error| NginxError::io("canonicalize nginx configuration entry file", error))?;
    let entry_source_id = match resolver.load(&entry_path, 0, "main", &mut Vec::new())? {
        Some(source_id) => source_id,
        None => return Err(entry_load_error(&resolver.diagnostics)),
    };
    Ok(build_configuration(
        record.id.clone(),
        entry_source_id,
        resolver.sources,
        resolver.contexts,
        resolver.diagnostics,
    ))
}

struct Resolver {
    roots: Vec<PathBuf>,
    loaded: HashMap<PathBuf, String>,
    contexts: HashMap<String, String>,
    sources: Vec<NginxConfigSource>,
    diagnostics: Vec<NginxConfigDiagnostic>,
    total_bytes: u64,
}

impl Resolver {
    fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            loaded: HashMap::new(),
            contexts: HashMap::new(),
            sources: Vec::new(),
            diagnostics: Vec::new(),
            total_bytes: 0,
        }
    }

    fn load(
        &mut self,
        requested: &Path,
        depth: usize,
        context: &str,
        stack: &mut Vec<PathBuf>,
    ) -> NginxResult<Option<String>> {
        if depth > MAX_INCLUDE_DEPTH {
            self.push_diagnostic(
                "NGINX_CONFIG_INCLUDE_DEPTH",
                "include depth exceeds 32",
                None,
            );
            return Ok(None);
        }
        let canonical = match requested.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                self.push_diagnostic(
                    "NGINX_CONFIG_INCLUDE_NOT_FOUND",
                    "included configuration file was not found",
                    None,
                );
                return Ok(None);
            }
        };
        if !self.roots.iter().any(|root| canonical.starts_with(root))
            || reject_reparse_points(&canonical).is_err()
        {
            self.push_diagnostic(
                "NGINX_CONFIG_INCLUDE_UNAUTHORIZED",
                "included file is outside the explicitly authorized roots",
                None,
            );
            return Ok(None);
        }
        if stack.contains(&canonical) {
            self.push_diagnostic("NGINX_CONFIG_INCLUDE_CYCLE", "include cycle detected", None);
            return Ok(None);
        }
        if let Some(source_id) = self.loaded.get(&canonical) {
            return Ok(Some(source_id.clone()));
        }
        if self.sources.len() >= MAX_FILES {
            self.push_diagnostic(
                "NGINX_CONFIG_FILE_LIMIT",
                "configuration contains more than 2,048 files",
                None,
            );
            return Ok(None);
        }
        let metadata = fs::metadata(&canonical)
            .map_err(|error| NginxError::io("inspect nginx configuration", error))?;
        if metadata.len() > MAX_FILE_BYTES {
            self.push_diagnostic(
                "NGINX_CONFIG_FILE_TOO_LARGE",
                "configuration file exceeds 4 MiB",
                None,
            );
            return Ok(None);
        }
        if self.total_bytes.saturating_add(metadata.len()) > MAX_TOTAL_BYTES {
            self.push_diagnostic(
                "NGINX_CONFIG_TOTAL_TOO_LARGE",
                "configuration set exceeds 32 MiB",
                None,
            );
            return Ok(None);
        }
        let bytes = fs::read(&canonical)
            .map_err(|error| NginxError::io("read nginx configuration", error))?;
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                self.push_diagnostic(
                    "NGINX_CONFIG_ENCODING_INVALID",
                    "configuration file is not valid UTF-8",
                    None,
                );
                return Ok(None);
            }
        };
        self.total_bytes += metadata.len();
        let source_id = source_id(&canonical);
        let (tokens, mut diagnostics) = lex(&source_id, &text);
        let (directives, parser_diagnostics) = parse(&source_id, &text, &tokens);
        diagnostics.extend(parser_diagnostics);
        self.diagnostics.extend(diagnostics);
        self.loaded.insert(canonical.clone(), source_id.clone());
        self.contexts
            .entry(source_id.clone())
            .or_insert_with(|| context.to_owned());
        self.sources.push(NginxConfigSource {
            id: source_id.clone(),
            display_path: canonical.to_string_lossy().into_owned(),
            text,
            directives: directives.clone(),
        });

        stack.push(canonical.clone());
        let parent = canonical.parent().unwrap_or(Path::new("."));
        for (directive, include_context) in walk_includes(&directives, context) {
            for argument in &directive.arguments {
                let requested = Path::new(argument);
                let pattern = if requested.is_absolute() {
                    requested.to_path_buf()
                } else {
                    parent.join(requested)
                };
                let pattern_text = pattern.to_string_lossy().into_owned();
                let mut matched = false;
                match glob(&pattern_text) {
                    Ok(paths) => {
                        for candidate in paths.flatten() {
                            matched = true;
                            let _ = self.load(&candidate, depth + 1, &include_context, stack)?;
                        }
                    }
                    Err(_) => self.push_diagnostic(
                        "NGINX_CONFIG_INCLUDE_GLOB_INVALID",
                        "include glob pattern is invalid",
                        Some(directive.location.clone()),
                    ),
                }
                if !matched {
                    self.push_diagnostic(
                        "NGINX_CONFIG_INCLUDE_NOT_FOUND",
                        "include pattern matched no files",
                        Some(directive.location.clone()),
                    );
                }
            }
        }
        stack.pop();
        Ok(Some(source_id))
    }

    fn push_diagnostic(
        &mut self,
        code: &str,
        message: &str,
        location: Option<NginxSourceLocation>,
    ) {
        self.diagnostics.push(NginxConfigDiagnostic {
            code: code.to_owned(),
            severity: "error".to_owned(),
            message: message.to_owned(),
            location,
        });
    }
}

fn lex(source_id: &str, text: &str) -> (Vec<Token>, Vec<NginxConfigDiagnostic>) {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            byte if byte.is_ascii_whitespace() => index += 1,
            b'#' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'{' | b'}' | b';' => {
                let kind = match bytes[index] {
                    b'{' => TokenKind::LeftBrace,
                    b'}' => TokenKind::RightBrace,
                    _ => TokenKind::Semicolon,
                };
                tokens.push(Token {
                    kind,
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            quote @ (b'\'' | b'"') => {
                let start = index;
                index += 1;
                let value_start = index;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                let value_end = index;
                if closed {
                    index += 1;
                } else {
                    diagnostics.push(diagnostic(
                        "NGINX_CONFIG_UNTERMINATED_QUOTE",
                        "unterminated quoted argument",
                        Some(location(source_id, text, start, bytes.len())),
                    ));
                }
                tokens.push(Token {
                    kind: TokenKind::Word(text[value_start..value_end].to_owned()),
                    start,
                    end: index,
                });
            }
            _ => {
                let start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && !matches!(bytes[index], b'{' | b'}' | b';' | b'#')
                {
                    if bytes[index] == b'\\' && index + 1 < bytes.len() {
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Word(text[start..index].to_owned()),
                    start,
                    end: index,
                });
            }
        }
    }
    (tokens, diagnostics)
}

fn parse(
    source_id: &str,
    text: &str,
    tokens: &[Token],
) -> (Vec<NginxDirective>, Vec<NginxConfigDiagnostic>) {
    let mut index = 0;
    let mut diagnostics = Vec::new();
    let directives = parse_scope(source_id, text, tokens, &mut index, false, &mut diagnostics);
    (directives, diagnostics)
}

fn parse_scope(
    source_id: &str,
    text: &str,
    tokens: &[Token],
    index: &mut usize,
    nested: bool,
    diagnostics: &mut Vec<NginxConfigDiagnostic>,
) -> Vec<NginxDirective> {
    let mut directives = Vec::new();
    while *index < tokens.len() {
        if matches!(tokens[*index].kind, TokenKind::RightBrace) {
            if nested {
                *index += 1;
                return directives;
            }
            diagnostics.push(diagnostic(
                "NGINX_CONFIG_UNEXPECTED_BRACE",
                "unexpected closing brace",
                Some(location(
                    source_id,
                    text,
                    tokens[*index].start,
                    tokens[*index].end,
                )),
            ));
            *index += 1;
            continue;
        }
        let start = tokens[*index].start;
        let TokenKind::Word(name) = &tokens[*index].kind else {
            diagnostics.push(diagnostic(
                "NGINX_CONFIG_DIRECTIVE_EXPECTED",
                "directive name expected",
                Some(location(source_id, text, start, tokens[*index].end)),
            ));
            *index += 1;
            continue;
        };
        let name = name.clone();
        *index += 1;
        let mut arguments = Vec::new();
        while *index < tokens.len() {
            match &tokens[*index].kind {
                TokenKind::Word(value) => {
                    arguments.push(value.clone());
                    *index += 1;
                }
                TokenKind::Semicolon => {
                    let end = tokens[*index].end;
                    *index += 1;
                    directives.push(NginxDirective {
                        name,
                        arguments,
                        raw: text[start..end].to_owned(),
                        location: location(source_id, text, start, end),
                        children: Vec::new(),
                    });
                    break;
                }
                TokenKind::LeftBrace => {
                    *index += 1;
                    let children = parse_scope(source_id, text, tokens, index, true, diagnostics);
                    let end = tokens
                        .get(index.saturating_sub(1))
                        .map_or(text.len(), |token| token.end);
                    directives.push(NginxDirective {
                        name,
                        arguments,
                        raw: text[start..end].to_owned(),
                        location: location(source_id, text, start, end),
                        children,
                    });
                    break;
                }
                TokenKind::RightBrace => {
                    diagnostics.push(diagnostic(
                        "NGINX_CONFIG_MISSING_SEMICOLON",
                        "directive is missing a semicolon",
                        Some(location(source_id, text, start, tokens[*index].start)),
                    ));
                    break;
                }
            }
        }
        if *index >= tokens.len()
            && !directives
                .iter()
                .any(|directive| directive.location.line == line_column(text, start).0)
        {
            diagnostics.push(diagnostic(
                "NGINX_CONFIG_UNEXPECTED_EOF",
                "configuration ended before the directive was complete",
                Some(location(source_id, text, start, text.len())),
            ));
        }
    }
    if nested {
        diagnostics.push(diagnostic(
            "NGINX_CONFIG_UNCLOSED_BLOCK",
            "configuration block is missing a closing brace",
            None,
        ));
    }
    directives
}

fn build_configuration(
    instance_id: String,
    entry_source_id: String,
    sources: Vec<NginxConfigSource>,
    contexts: HashMap<String, String>,
    diagnostics: Vec<NginxConfigDiagnostic>,
) -> NginxConfiguration {
    let mut sites = Vec::new();
    let mut upstreams = Vec::new();
    let mut pid_path = None;
    let mut access_logs = Vec::new();
    let mut error_logs = Vec::new();
    for source in &sources {
        let context = contexts.get(&source.id).map_or("main", String::as_str);
        extract_model(
            &source.directives,
            context,
            &mut sites,
            &mut upstreams,
            &mut pid_path,
            &mut access_logs,
            &mut error_logs,
        );
    }
    let (topology_nodes, topology_edges) = build_topology(&sites, &upstreams);
    NginxConfiguration {
        instance_id,
        entry_source_id,
        sources,
        diagnostics,
        sites,
        upstreams,
        topology_nodes,
        topology_edges,
        pid_path,
        access_logs,
        error_logs,
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_model(
    directives: &[NginxDirective],
    context: &str,
    sites: &mut Vec<NginxSite>,
    upstreams: &mut Vec<NginxUpstream>,
    pid_path: &mut Option<String>,
    access_logs: &mut Vec<String>,
    error_logs: &mut Vec<String>,
) {
    for directive in directives {
        match directive.name.as_str() {
            "pid" if pid_path.is_none() => *pid_path = directive.arguments.first().cloned(),
            "access_log" => access_logs.extend(directive.arguments.first().cloned()),
            "error_log" => error_logs.extend(directive.arguments.first().cloned()),
            "server" if context == "http" || context == "stream" => {
                sites.push(site_from_directive(directive, context));
            }
            "upstream" => {
                upstreams.push(NginxUpstream {
                    name: directive.arguments.first().cloned().unwrap_or_default(),
                    servers: direct_values(&directive.children, "server"),
                    source: directive.location.clone(),
                });
            }
            _ => {}
        }
        let child_context = if directive.name == "http" || directive.name == "stream" {
            directive.name.as_str()
        } else {
            context
        };
        if directive.name != "upstream" {
            extract_model(
                &directive.children,
                child_context,
                sites,
                upstreams,
                pid_path,
                access_logs,
                error_logs,
            );
        }
    }
}

fn site_from_directive(directive: &NginxDirective, context: &str) -> NginxSite {
    let listens = direct_values(&directive.children, "listen");
    let server_names = direct_arguments(&directive.children, "server_name");
    let root = walk_directives(&directive.children)
        .into_iter()
        .find(|child| child.name == "root")
        .and_then(|child| child.arguments.first().cloned());
    let proxy_pass = walk_directives(&directive.children)
        .into_iter()
        .filter(|child| child.name == "proxy_pass")
        .filter_map(|child| child.arguments.first().cloned())
        .collect();
    let locations = directive
        .children
        .iter()
        .filter(|child| child.name == "location")
        .map(|child| child.arguments.join(" "))
        .collect();
    NginxSite {
        id: format!(
            "{}:{}:{}",
            directive.location.source_id, directive.location.line, directive.location.column
        ),
        context: context.to_owned(),
        listens,
        server_names,
        root,
        proxy_pass,
        locations,
        source: directive.location.clone(),
    }
}

fn build_topology(
    sites: &[NginxSite],
    upstreams: &[NginxUpstream],
) -> (Vec<NginxTopologyNode>, Vec<NginxTopologyEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let upstream_names = upstreams
        .iter()
        .map(|item| item.name.as_str())
        .collect::<HashSet<_>>();
    for upstream in upstreams {
        nodes.push(NginxTopologyNode {
            id: format!("upstream:{}", upstream.name),
            kind: "upstream".to_owned(),
            label: upstream.name.clone(),
        });
    }
    for (index, site) in sites.iter().enumerate() {
        let node_id = format!("site:{index}");
        nodes.push(NginxTopologyNode {
            id: node_id.clone(),
            kind: "site".to_owned(),
            label: site
                .server_names
                .first()
                .cloned()
                .unwrap_or_else(|| "_".to_owned()),
        });
        for target in &site.proxy_pass {
            let name = target
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(['/', ':'])
                .next()
                .unwrap_or_default();
            if upstream_names.contains(name) {
                edges.push(NginxTopologyEdge {
                    from: node_id.clone(),
                    to: format!("upstream:{name}"),
                    label: "proxy_pass".to_owned(),
                });
            }
        }
    }
    (nodes, edges)
}

fn direct_values(directives: &[NginxDirective], name: &str) -> Vec<String> {
    directives
        .iter()
        .filter(|directive| directive.name == name)
        .map(|directive| directive.arguments.join(" "))
        .collect()
}

fn direct_arguments(directives: &[NginxDirective], name: &str) -> Vec<String> {
    directives
        .iter()
        .filter(|directive| directive.name == name)
        .flat_map(|directive| directive.arguments.clone())
        .collect()
}

fn walk_directives(directives: &[NginxDirective]) -> Vec<&NginxDirective> {
    let mut result = Vec::new();
    for directive in directives {
        result.push(directive);
        result.extend(walk_directives(&directive.children));
    }
    result
}

fn walk_includes<'a>(
    directives: &'a [NginxDirective],
    context: &str,
) -> Vec<(&'a NginxDirective, String)> {
    let mut result = Vec::new();
    for directive in directives {
        let child_context = if directive.name == "http" || directive.name == "stream" {
            directive.name.as_str()
        } else {
            context
        };
        if directive.name == "include" {
            result.push((directive, context.to_owned()));
        }
        result.extend(walk_includes(&directive.children, child_context));
    }
    result
}

fn source_id(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()))
}

fn entry_load_error(diagnostics: &[NginxConfigDiagnostic]) -> NginxError {
    let code = diagnostics.last().map(|item| item.code.as_str());
    match code {
        Some("NGINX_CONFIG_ENCODING_INVALID") => NginxError::new(
            "NGINX_CONFIG_ENCODING_INVALID",
            "the configuration entry is not valid UTF-8",
        ),
        Some("NGINX_CONFIG_FILE_TOO_LARGE") => NginxError::new(
            "NGINX_CONFIG_FILE_TOO_LARGE",
            "the configuration entry exceeds 4 MiB",
        ),
        Some("NGINX_CONFIG_INCLUDE_UNAUTHORIZED") => NginxError::new(
            "NGINX_CONFIG_INCLUDE_UNAUTHORIZED",
            "the configuration entry is outside authorized roots",
        ),
        _ => NginxError::new(
            "NGINX_CONFIG_NOT_FOUND",
            "the configuration entry could not be loaded",
        ),
    }
}

fn diagnostic(
    code: &str,
    message: &str,
    location: Option<NginxSourceLocation>,
) -> NginxConfigDiagnostic {
    NginxConfigDiagnostic {
        code: code.to_owned(),
        severity: "error".to_owned(),
        message: message.to_owned(),
        location,
    }
}

fn location(source_id: &str, text: &str, start: usize, end: usize) -> NginxSourceLocation {
    let (line, column) = line_column(text, start);
    let (end_line, end_column) = line_column(text, end);
    NginxSourceLocation {
        source_id: source_id.to_owned(),
        line,
        column,
        end_line,
        end_column,
    }
}

fn line_column(text: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &text[..byte_offset.min(text.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count(), |(_, tail)| tail.chars().count())
        + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{
        NginxAuthorizationLevel, NginxControlBackend, NginxLifecycleState, NginxProviderIdentity,
    };

    fn record(root: &Path, config: &Path) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: "fixture".to_owned(),
            name: "Fixture".to_owned(),
            kind: "external".to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            binary_path: root.join("nginx").to_string_lossy().into_owned(),
            config_path: Some(config.to_string_lossy().into_owned()),
            authorized_roots: vec![root.to_string_lossy().into_owned()],
            authorization_level: NginxAuthorizationLevel::ReadOnly,
            version: "1.28.0".to_owned(),
            configure_arguments: Vec::new(),
            binary_fingerprint: "fingerprint".to_owned(),
            provider_identity: NginxProviderIdentity {
                provider: "portable".to_owned(),
                external_id: None,
            },
            control_backend: NginxControlBackend::Portable,
            lifecycle_state: NginxLifecycleState::Available,
            created_at: "2026-07-31T00:00:00Z".to_owned(),
            updated_at: "2026-07-31T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn preserves_source_and_extracts_sites_and_topology() {
        let directory = tempfile::tempdir().expect("tempdir");
        let conf = directory.path().join("nginx.conf");
        let included = directory.path().join("site.conf");
        fs::write(
            &conf,
            "# retained\nhttp { include site.conf; upstream api { server 127.0.0.1:3000; } }",
        )
        .expect("write root");
        fs::write(
            &included,
            "server { listen 8080; server_name local.test; location / { proxy_pass http://api; custom on; } }",
        )
        .expect("write include");

        let result = load_configuration(&record(directory.path(), &conf)).expect("configuration");

        assert_eq!(result.sources.len(), 2);
        assert!(result.sources[0].text.starts_with("# retained"));
        assert!(result.sources.iter().any(|source| {
            walk_directives(&source.directives)
                .iter()
                .any(|directive| directive.name == "custom" && directive.raw == "custom on;")
        }));
        assert_eq!(result.sites[0].server_names, ["local.test"]);
        assert_eq!(result.upstreams[0].name, "api");
        assert_eq!(result.topology_edges.len(), 1);
    }

    #[test]
    fn reports_include_cycle_and_unauthorized_file() {
        let directory = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let conf = directory.path().join("nginx.conf");
        let child = directory.path().join("child.conf");
        let escaped = outside.path().join("outside.conf");
        fs::write(&escaped, "events {}\n").expect("write outside");
        fs::write(
            &conf,
            format!("include child.conf; include {};", escaped.display()),
        )
        .expect("write root");
        fs::write(&child, "include nginx.conf;").expect("write child");

        let result = load_configuration(&record(directory.path(), &conf)).expect("configuration");
        let codes = result
            .diagnostics
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&"NGINX_CONFIG_INCLUDE_CYCLE"));
        assert!(codes.contains(&"NGINX_CONFIG_INCLUDE_UNAUTHORIZED"));
    }

    #[test]
    fn reports_invalid_utf8_without_exposing_bytes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let conf = directory.path().join("nginx.conf");
        fs::write(&conf, [0xff, 0xfe]).expect("write invalid utf8");
        let error = load_configuration(&record(directory.path(), &conf)).expect_err("entry fails");
        assert_eq!(error.code, "NGINX_CONFIG_ENCODING_INVALID");
    }

    #[test]
    fn enforces_per_file_and_include_depth_limits() {
        let directory = tempfile::tempdir().expect("tempdir");
        let oversized = directory.path().join("oversized.conf");
        let file = fs::File::create(&oversized).expect("create oversized");
        file.set_len(MAX_FILE_BYTES + 1).expect("extend oversized");
        let error = load_configuration(&record(directory.path(), &oversized))
            .expect_err("oversized entry fails");
        assert_eq!(error.code, "NGINX_CONFIG_FILE_TOO_LARGE");

        let valid = directory.path().join("valid.conf");
        fs::write(&valid, "events {}\n").expect("write valid");
        let mut resolver = Resolver::new(vec![directory
            .path()
            .canonicalize()
            .expect("canonical root")]);
        let loaded = resolver
            .load(&valid, MAX_INCLUDE_DEPTH + 1, "main", &mut Vec::new())
            .expect("bounded load");
        assert!(loaded.is_none());
        assert_eq!(
            resolver.diagnostics.last().map(|item| item.code.as_str()),
            Some("NGINX_CONFIG_INCLUDE_DEPTH")
        );
    }
}
