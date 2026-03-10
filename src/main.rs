use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rpassword::read_password;
use serde_json::Value;

type CliResult<T> = Result<T, String>;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> CliResult<()> {
    let args: Vec<String> = env::args().collect();
    let options = parse_args(&args)?;

    if options.help {
        print_help(&args[0]);
        return Ok(());
    }

    let root =
        env::current_dir().map_err(|e| format!("Failed to resolve current directory: {e}"))?;
    let contracts_dir = if options.contracts_dir.is_absolute() {
        options.contracts_dir.clone()
    } else {
        root.join(&options.contracts_dir)
    };

    if !contracts_dir.exists() {
        return Err(format!(
            "Contracts directory '{0}' does not exist",
            contracts_dir.display()
        ));
    }

    let contracts = discover_contracts_with_parser(
        &root,
        &contracts_dir,
        parser_for_backend(options.parser_backend),
    )?;

    if contracts.is_empty() {
        println!(
            "No deployable contracts found under {}",
            options.contracts_dir.display()
        );
        return Ok(());
    }

    if options.list {
        print_contract_list(&contracts);
        return Ok(());
    }

    let selection = options.selection.as_ref().ok_or_else(|| {
        "Missing contract selection. Provide a contract name or path.".to_string()
    })?;

    let contract = select_contract(&contracts, selection)?;

    let constructor_args = if let Some(json) = options.args_json.as_ref() {
        parse_args_json(json)?
    } else if !contract.constructor_params.is_empty() {
        prompt_for_args(contract)?
    } else {
        Vec::new()
    };

    if constructor_args.len() != contract.constructor_params.len() {
        return Err(format!(
            "Constructor for '{0}' expects {1} argument(s) but {2} provided.",
            contract.name,
            contract.constructor_params.len(),
            constructor_args.len()
        ));
    }

    let private_key = get_private_key(options.private_key.as_deref())?;

    let output_dir = if options.output_dir.is_absolute() {
        options.output_dir.clone()
    } else {
        root.join(&options.output_dir)
    };

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).map_err(|e| {
            format!(
                "Failed to create output directory '{0}': {e}",
                output_dir.display()
            )
        })?;
    }

    let script_path = output_dir.join(format!("{}.s.sol", contract.name));

    if script_path.exists() && !options.force {
        let rel_path = script_path
            .strip_prefix(&root)
            .unwrap_or(&script_path)
            .display()
            .to_string();
        return Err(format!(
            "Script '{rel_path}' already exists. Use --force to overwrite."
        ));
    }

    let import_path = compute_import_path(&script_path, &contract.absolute_path, &root)
        .ok_or_else(|| "Failed to compute relative import path.".to_string())?;

    let script = render_script(&contract, &import_path, &constructor_args, &private_key);

    fs::write(&script_path, script)
        .map_err(|e| format!("Failed to write script '{0}': {e}", script_path.display()))?;

    let rel_script = script_path
        .strip_prefix(&root)
        .unwrap_or(&script_path)
        .display()
        .to_string();
    println!("Generated script at {rel_script}");

    Ok(())
}

#[derive(Default)]
struct Options {
    contracts_dir: PathBuf,
    output_dir: PathBuf,
    parser_backend: ParserBackend,
    selection: Option<String>,
    args_json: Option<String>,
    list: bool,
    help: bool,
    force: bool,
    private_key: Option<String>,
}

#[derive(Clone, Debug)]
struct ConstructorParam {
    raw: String,
    name: Option<String>,
}

#[derive(Clone, Debug)]
struct ContractInfo {
    name: String,
    absolute_path: PathBuf,
    relative_path: PathBuf,
    constructor_params: Vec<ConstructorParam>,
    pragma: Option<String>,
}

impl ContractInfo {
    fn constructor_signature(&self) -> String {
        if self.constructor_params.is_empty() {
            "constructor()".to_string()
        } else {
            let joined = self
                .constructor_params
                .iter()
                .map(|param| param.raw.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("constructor({joined})")
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ParserBackend {
    #[default]
    StringWalker,
}

impl ParserBackend {
    fn parse(value: &str) -> CliResult<Self> {
        match value {
            "string-walker" => Ok(Self::StringWalker),
            other => Err(format!(
                "Unknown parser backend '{other}'. Available backends: string-walker"
            )),
        }
    }
}

trait ContractParser {
    fn parse(&self, source: &str) -> CliResult<Vec<ParsedContract>>;
}

struct StringWalkerParser;

impl ContractParser for StringWalkerParser {
    fn parse(&self, source: &str) -> CliResult<Vec<ParsedContract>> {
        Ok(parse_contracts_with_string_walker(source))
    }
}

fn parse_args(args: &[String]) -> CliResult<Options> {
    let mut options = Options {
        contracts_dir: PathBuf::from("src"),
        output_dir: PathBuf::from("script"),
        parser_backend: ParserBackend::default(),
        ..Options::default()
    };

    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => options.help = true,
            "--list" => options.list = true,
            "--force" => options.force = true,
            "--contracts-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--contracts-dir requires a value".to_string())?;
                options.contracts_dir = PathBuf::from(value);
            }
            value if value.starts_with("--contracts-dir=") => {
                let dir = &value["--contracts-dir=".len()..];
                if dir.is_empty() {
                    return Err("--contracts-dir requires a value".to_string());
                }
                options.contracts_dir = PathBuf::from(dir);
            }
            "--output-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--output-dir requires a value".to_string())?;
                options.output_dir = PathBuf::from(value);
            }
            value if value.starts_with("--output-dir=") => {
                let dir = &value["--output-dir=".len()..];
                if dir.is_empty() {
                    return Err("--output-dir requires a value".to_string());
                }
                options.output_dir = PathBuf::from(dir);
            }
            "--parser" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--parser requires a value".to_string())?;
                options.parser_backend = ParserBackend::parse(value)?;
            }
            value if value.starts_with("--parser=") => {
                let parser_value = &value["--parser=".len()..];
                if parser_value.is_empty() {
                    return Err("--parser requires a value".to_string());
                }
                options.parser_backend = ParserBackend::parse(parser_value)?;
            }
            "--args" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--args requires a JSON array".to_string())?;
                options.args_json = Some(value.to_string());
            }
            value if value.starts_with("--args=") => {
                let args_value = &value["--args=".len()..];
                if args_value.is_empty() {
                    return Err("--args requires a JSON array".to_string());
                }
                options.args_json = Some(args_value.to_string());
            }
            "--private-key" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--private-key requires a value".to_string())?;
                if value.trim().is_empty() {
                    return Err("--private-key requires a value".to_string());
                }
                options.private_key = Some(value.to_string());
            }
            value if value.starts_with("--private-key=") => {
                let key_value = &value["--private-key=".len()..];
                if key_value.trim().is_empty() {
                    return Err("--private-key requires a value".to_string());
                }
                options.private_key = Some(key_value.to_string());
            }
            "--" => {
                if let Some(selection) = iter.next() {
                    options.selection = Some(selection.to_string());
                }
                break;
            }
            value if value.starts_with("--") => {
                return Err(format!("Unknown option: {value}"));
            }
            value => {
                if options.selection.is_some() {
                    return Err("Only one contract selection can be provided".to_string());
                }
                options.selection = Some(value.to_string());
            }
        }
    }

    Ok(options)
}

fn print_help(program: &str) {
    println!("forge-scriptgen - Generate Foundry deployment scripts\n");
    println!("Usage: {program} [OPTIONS] <CONTRACT>\n");
    println!("Options:");
    println!("  --contracts-dir <DIR>   Directory containing Solidity sources (default: src)");
    println!("  --output-dir <DIR>      Directory for generated scripts (default: script)");
    println!("  --parser <NAME>         Contract parser backend (default: string-walker)");
    println!(
        "  --args <JSON>           Constructor arguments as JSON array; use {{\"raw\":\"...\"}} for Solidity literals"
    );
    println!("  --private-key <KEY>     Private key literal to embed in the script");
    println!("  --list                  List discoverable contracts and exit");
    println!("  --force                 Overwrite existing script when it already exists");
    println!("  -h, --help              Show this help message");
    println!("\nExamples:");
    println!("  {program} --parser string-walker --list");
    println!("  {program} Counter");
    println!("  {program} --args '[\"hello\", 10]' --private-key 0xabc src/Counter.sol");
    println!(
        "  {program} --parser string-walker --args '[{{\"raw\":\"Config({{owner: msg.sender, limits: [1, 2, 3]}})\"}},{{\"raw\":\"callback\"}},\"primary\",{{\"raw\":\"hex\\\"1234\\\"\"}}]' --private-key 0xabc123 ComplexDeployment"
    );
}

fn discover_contracts_with_parser(
    root: &Path,
    contracts_dir: &Path,
    parser: &dyn ContractParser,
) -> CliResult<Vec<ContractInfo>> {
    let mut files = Vec::new();
    collect_solidity_files(contracts_dir, &mut files)?;
    files.sort();

    let mut contracts = Vec::new();

    for file_path in files {
        let source = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read '{0}': {e}", file_path.display()))?;

        let relative = file_path
            .strip_prefix(root)
            .unwrap_or(&file_path)
            .to_path_buf();

        let pragma = extract_pragma(&source);
        let parsed = parser.parse(&source)?;

        for item in parsed {
            contracts.push(ContractInfo {
                name: item.name,
                absolute_path: file_path.clone(),
                relative_path: relative.clone(),
                constructor_params: item.constructor_params,
                pragma: pragma.clone(),
            });
        }
    }

    Ok(contracts)
}

fn parser_for_backend(backend: ParserBackend) -> &'static dyn ContractParser {
    static STRING_WALKER: StringWalkerParser = StringWalkerParser;

    match backend {
        ParserBackend::StringWalker => &STRING_WALKER,
    }
}

fn collect_solidity_files(dir: &Path, files: &mut Vec<PathBuf>) -> CliResult<()> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{0}': {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect '{0}': {e}", path.display()))?;

        if file_type.is_dir() {
            collect_solidity_files(&path, files)?;
        } else if file_type.is_file() {
            if matches!(path.extension().and_then(|ext| ext.to_str()), Some(ext) if ext.eq_ignore_ascii_case("sol"))
            {
                files.push(path);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedContract {
    name: String,
    constructor_params: Vec<ConstructorParam>,
}

fn parse_contracts_with_string_walker(source: &str) -> Vec<ParsedContract> {
    let stripped = strip_comments(source);
    let mut contracts = Vec::new();
    let keyword = "contract";
    let key_len = keyword.len();
    let mut pos = 0;
    let mut tracker = StringTracker::default();

    while pos < stripped.len() {
        let current = match stripped[pos..].chars().next() {
            Some(ch) => ch,
            None => break,
        };

        if !tracker.in_string() && stripped[pos..].starts_with(keyword) {
            let end = pos + key_len;
            if is_identifier_boundary(&stripped, pos, end) {
                if is_abstract_contract(&stripped, pos) {
                    pos += key_len;
                    tracker.consume(current);
                    continue;
                }

                let cursor = skip_whitespace(&stripped, end);
                let name_end = identifier_end(&stripped, cursor);
                if name_end == cursor {
                    pos += key_len;
                    tracker.consume(current);
                    continue;
                }

                let name = stripped[cursor..name_end].to_string();

                if let Some(open_brace) = find_next_char(&stripped, name_end, '{') {
                    if let Some(close_brace) = find_matching_brace(&stripped, open_brace) {
                        let body = &stripped[(open_brace + 1)..close_brace];
                        let constructor_params = parse_constructor_params(body);
                        contracts.push(ParsedContract {
                            name: name.clone(),
                            constructor_params,
                        });
                        pos = close_brace + 1;
                        tracker.reset();
                        continue;
                    }
                }
            }
        }

        tracker.consume(current);
        pos += current.len_utf8();
    }

    contracts
}

fn parse_constructor_params(body: &str) -> Vec<ConstructorParam> {
    let keyword = "constructor";
    let mut pos = 0;
    let mut tracker = StringTracker::default();

    while pos < body.len() {
        let ch = match body[pos..].chars().next() {
            Some(ch) => ch,
            None => break,
        };

        if !tracker.in_string() && body[pos..].starts_with(keyword) {
            let after = pos + keyword.len();
            if !is_identifier_char_before(&body, pos) && !is_identifier_char_after(&body, after) {
                if let Some(open_paren) = find_next_char(body, after, '(') {
                    if let Some(close_paren) = find_matching_paren(body, open_paren) {
                        let params_text = &body[(open_paren + 1)..close_paren];
                        return split_parameters(params_text)
                            .into_iter()
                            .map(|raw| ConstructorParam {
                                name: extract_param_name(&raw),
                                raw,
                            })
                            .collect();
                    }
                }
            }
        }

        tracker.consume(ch);
        pos += ch.len_utf8();
    }

    Vec::new()
}

fn split_parameters(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut pos = 0;
    let mut tracker = BracketTracker::default();

    while pos < input.len() {
        let ch = match input[pos..].chars().next() {
            Some(ch) => ch,
            None => break,
        };

        if tracker.should_split_on_comma(ch) {
            let fragment = input[start..pos].trim();
            if !fragment.is_empty() {
                parts.push(fragment.to_string());
            }
            start = pos + ch.len_utf8();
            tracker.consume(ch);
            pos += ch.len_utf8();
            continue;
        }

        tracker.consume(ch);
        pos += ch.len_utf8();
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }

    parts
}

fn extract_param_name(param: &str) -> Option<String> {
    let trimmed = param.trim();
    if trimmed.is_empty() || !trimmed.chars().any(|c| c.is_whitespace()) {
        return None;
    }

    let mut current = String::new();
    let mut chars = trimmed.chars().rev();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }

        if current.is_empty() {
            if ch.is_whitespace()
                || matches!(ch, ')' | '(' | '[' | ']' | '{' | '}' | ',' | ';' | ':')
            {
                continue;
            }
            current.clear();
            continue;
        }

        let candidate = current.chars().rev().collect::<String>();
        if is_reserved_suffix(&candidate) {
            current.clear();
            continue;
        }
        return Some(candidate);
    }

    if !current.is_empty() {
        let candidate = current.chars().rev().collect::<String>();
        if !is_reserved_suffix(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn is_reserved_suffix(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "memory" | "calldata" | "storage" | "payable" | "indexed" | "virtual" | "override"
    )
}

fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut tracker = StringTracker::default();

    while let Some(ch) = chars.next() {
        if tracker.in_single_quote {
            result.push(ch);
            tracker.consume(ch);
            continue;
        }
        if tracker.in_double_quote {
            result.push(ch);
            tracker.consume(ch);
            continue;
        }

        if ch == '/' {
            if let Some(&next) = chars.peek() {
                if next == '/' {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                    tracker.reset();
                    continue;
                } else if next == '*' {
                    chars.next();
                    let mut prev = '\0';
                    while let Some(c) = chars.next() {
                        if c == '\n' {
                            result.push('\n');
                        }
                        if prev == '*' && c == '/' {
                            break;
                        }
                        prev = c;
                    }
                    tracker.reset();
                    continue;
                }
            }
        }

        result.push(ch);
        tracker.consume(ch);
    }

    result
}

fn select_contract<'a>(
    contracts: &'a [ContractInfo],
    selection: &str,
) -> CliResult<&'a ContractInfo> {
    let normalized = selection.replace('\\', "/");

    let mut matches: Vec<&ContractInfo> = contracts
        .iter()
        .filter(|contract| {
            contract.name == selection
                || contract.name == normalized
                || contract.relative_path.to_string_lossy().replace('\\', "/") == normalized
                || contract
                    .relative_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == selection || name == normalized)
                    .unwrap_or(false)
        })
        .collect();

    if matches.is_empty() {
        return Err(format!("No contract matching '{selection}' was found"));
    }

    matches.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    matches.dedup_by(|a, b| a.relative_path == b.relative_path && a.name == b.name);

    if matches.len() > 1 {
        let options = matches
            .iter()
            .map(|info| format!("{} ({})", info.name, info.relative_path.display()))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Multiple contracts matched '{selection}'. Be more specific: {options}"
        ));
    }

    Ok(matches[0])
}

fn parse_args_json(input: &str) -> CliResult<Vec<String>> {
    let value: Value = serde_json::from_str(input)
        .map_err(|e| format!("Failed to parse --args JSON array: {e}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| "--args expects a JSON array, e.g. '[42, \"0xdead\"]'".to_string())?;

    array.iter().map(parse_json_arg_value).collect()
}

fn parse_json_arg_value(value: &Value) -> CliResult<String> {
    match value {
        Value::Null => Err(
            "Null is not a valid Solidity constructor argument. Use a Solidity literal instead."
                .to_string(),
        ),
        Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            Ok(value.to_string())
        }
        Value::Object(map) => {
            let raw = map
                .get("raw")
                .or_else(|| map.get("solidity"))
                .and_then(Value::as_str);

            if let Some(raw) = raw {
                if raw.trim().is_empty() {
                    return Err("Raw Solidity literal cannot be empty.".to_string());
                }
                return Ok(raw.to_string());
            }

            Err(
                "JSON objects are only supported as raw Solidity literals, e.g. {\"raw\":\"Foo({bar: 1})\"}."
                    .to_string(),
            )
        }
    }
}

fn prompt_for_args(contract: &ContractInfo) -> CliResult<Vec<String>> {
    println!(
        "Constructor parameters for '{}': {}",
        contract.name,
        contract.constructor_signature()
    );

    let mut values = Vec::new();
    let mut input = String::new();
    let stdin = io::stdin();

    for (index, param) in contract.constructor_params.iter().enumerate() {
        loop {
            let prompt = match param.name.as_deref() {
                Some(name) => format!("  [{index}] {name} ({})", param.raw),
                None => format!("  [{index}] {}", param.raw),
            };
            print!("{prompt}: ");
            io::stdout()
                .flush()
                .map_err(|e| format!("Failed to flush stdout: {e}"))?;

            input.clear();
            stdin
                .read_line(&mut input)
                .map_err(|e| format!("Failed to read input: {e}"))?;
            let value = input.trim();
            if value.is_empty() {
                println!(
                    "    Value required. Provide the literal as it should appear in Solidity."
                );
                continue;
            }
            values.push(value.to_string());
            break;
        }
    }

    Ok(values)
}

fn get_private_key(provided: Option<&str>) -> CliResult<String> {
    if let Some(value) = provided {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("Private key cannot be empty.".to_string());
        }
        return Ok(trimmed.to_string());
    }

    prompt_for_private_key()
}

fn prompt_for_private_key() -> CliResult<String> {
    println!("Private key will be embedded directly into the generated script. Use with caution.");

    loop {
        print!("Private key: ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed to flush stdout: {e}"))?;

        let input = read_password().map_err(|e| format!("Failed to read private key: {e}"))?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            println!("    Private key is required.");
            continue;
        }

        return Ok(trimmed.to_string());
    }
}

fn compute_import_path(script_path: &Path, contract_path: &Path, root: &Path) -> Option<String> {
    let script_dir = script_path.parent()?;
    let base = if script_dir.is_absolute() {
        script_dir.to_path_buf()
    } else {
        root.join(script_dir)
    };
    let target = if contract_path.is_absolute() {
        contract_path.to_path_buf()
    } else {
        root.join(contract_path)
    };

    let relative = relative_path_between(&base, &target)?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn relative_path_between(from: &Path, to: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut from_iter = from.components();
    let mut to_iter = to.components();

    let mut from_prefix = None;
    let mut to_prefix = None;

    if let Some(Component::Prefix(prefix)) = from_iter.clone().next() {
        from_prefix = Some(prefix);
        from_iter.next();
    }
    if let Some(Component::Prefix(prefix)) = to_iter.clone().next() {
        to_prefix = Some(prefix);
        to_iter.next();
    }

    if from_prefix != to_prefix {
        return None;
    }

    if matches!(from_iter.clone().next(), Some(Component::RootDir)) {
        from_iter.next();
    }
    if matches!(to_iter.clone().next(), Some(Component::RootDir)) {
        to_iter.next();
    }

    let from_components: Vec<_> = from_iter.collect();
    let to_components: Vec<_> = to_iter.collect();

    let mut common_len = 0;
    while common_len < from_components.len()
        && common_len < to_components.len()
        && from_components[common_len] == to_components[common_len]
    {
        common_len += 1;
    }

    let mut result = PathBuf::new();

    for comp in from_components.iter().skip(common_len) {
        match comp {
            Component::Normal(_) | Component::CurDir | Component::ParentDir => result.push(".."),
            _ => {}
        }
    }

    for comp in to_components.iter().skip(common_len) {
        result.push(comp.as_os_str());
    }

    if result.as_os_str().is_empty() {
        result.push(".");
    }

    Some(result)
}

fn render_script(
    contract: &ContractInfo,
    import_path: &str,
    args: &[String],
    private_key_literal: &str,
) -> String {
    let pragma = contract
        .pragma
        .as_deref()
        .filter(|line| line.starts_with("pragma solidity"))
        .map(|line| line.trim_end_matches(';').to_string())
        .unwrap_or_else(|| "pragma solidity ^0.8.23".to_string());

    let constructor_call = if args.is_empty() {
        format!("        {0} instance = new {0}();\n", contract.name)
    } else {
        format!(
            "        {0} instance = new {0}({1});\n",
            contract.name,
            args.join(", ")
        )
    };

    format!(
        "// SPDX-License-Identifier: UNLICENSED\n// Generated by forge-scriptgen.\n{pragma};\n\nimport \"forge-std/Script.sol\";\nimport \"{import_path}\";\n\ncontract {name}Script is Script {{\n    function run() public {{\n        uint256 deployerPrivateKey = {private_key};\n        vm.startBroadcast(deployerPrivateKey);\n\n{constructor_call}        vm.stopBroadcast();\n    }}\n}}\n",
        name = contract.name,
        pragma = pragma,
        import_path = import_path,
        constructor_call = constructor_call,
        private_key = private_key_literal
    )
}

fn print_contract_list(contracts: &[ContractInfo]) {
    for contract in contracts {
        println!("- {} ({})", contract.name, contract.relative_path.display());
        if contract.constructor_params.is_empty() {
            println!("  constructor(): no parameters");
        } else {
            println!("  {}", contract.constructor_signature());
            for (index, param) in contract.constructor_params.iter().enumerate() {
                println!("    [{index}] {}", param.raw);
            }
        }
    }
}

fn find_next_char(source: &str, mut pos: usize, target: char) -> Option<usize> {
    while pos < source.len() {
        let ch = source[pos..].chars().next()?;
        if ch == target {
            return Some(pos);
        }
        if !ch.is_whitespace() {
            if target == '{' {
                // Continue searching even across other characters (e.g. inheritance lists)
            }
        }
        pos += ch.len_utf8();
    }
    None
}

fn find_matching_brace(source: &str, open_index: usize) -> Option<usize> {
    find_matching_delimiter(source, open_index, '{', '}')
}

fn find_matching_paren(source: &str, open_index: usize) -> Option<usize> {
    find_matching_delimiter(source, open_index, '(', ')')
}

fn find_matching_delimiter(
    source: &str,
    open_index: usize,
    open: char,
    close: char,
) -> Option<usize> {
    let mut depth = 0;
    let mut pos = open_index;
    let mut tracker = StringTracker::default();

    while pos < source.len() {
        let ch = source[pos..].chars().next()?;
        if !tracker.in_string() {
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
        }
        tracker.consume(ch);
        pos += ch.len_utf8();
    }

    None
}

fn skip_whitespace(source: &str, mut index: usize) -> usize {
    while index < source.len() {
        let ch = match source[index..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        if ch.is_whitespace() {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    index
}

fn identifier_end(source: &str, mut index: usize) -> usize {
    while index < source.len() {
        let ch = match source[index..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        if is_identifier_char(ch) {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    index
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_identifier_boundary(source: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        None
    } else {
        source[..start].chars().rev().next()
    };
    let after = source[end..].chars().next();

    before.map_or(true, |ch| !is_identifier_char(ch))
        && after.map_or(true, |ch| !is_identifier_char(ch))
}

fn is_identifier_char_before(source: &str, start: usize) -> bool {
    if start == 0 {
        return false;
    }
    source[..start]
        .chars()
        .rev()
        .next()
        .map_or(false, |ch| is_identifier_char(ch))
}

fn is_identifier_char_after(source: &str, index: usize) -> bool {
    source[index..]
        .chars()
        .next()
        .map_or(false, |ch| is_identifier_char(ch))
}

fn is_abstract_contract(source: &str, position: usize) -> bool {
    let prefix = &source[..position];
    prefix
        .trim_end()
        .split_whitespace()
        .last()
        .map_or(false, |word| word == "abstract")
}

fn extract_pragma(source: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pragma solidity") {
            return Some(trimmed.trim_end_matches(';').to_string());
        }
    }
    None
}

#[derive(Default)]
struct StringTracker {
    in_single_quote: bool,
    in_double_quote: bool,
    escape: bool,
}

impl StringTracker {
    fn consume(&mut self, ch: char) {
        if self.in_single_quote {
            if self.escape {
                self.escape = false;
            } else {
                if ch == '\\' {
                    self.escape = true;
                } else if ch == '\'' {
                    self.in_single_quote = false;
                }
            }
            return;
        }

        if self.in_double_quote {
            if self.escape {
                self.escape = false;
            } else {
                if ch == '\\' {
                    self.escape = true;
                } else if ch == '"' {
                    self.in_double_quote = false;
                }
            }
            return;
        }

        if ch == '\'' {
            self.in_single_quote = true;
            self.escape = false;
        } else if ch == '"' {
            self.in_double_quote = true;
            self.escape = false;
        } else {
            self.escape = false;
        }
    }

    fn in_string(&self) -> bool {
        self.in_single_quote || self.in_double_quote
    }

    fn reset(&mut self) {
        self.in_single_quote = false;
        self.in_double_quote = false;
        self.escape = false;
    }
}

#[derive(Default)]
struct BracketTracker {
    paren_depth: usize,
    bracket_depth: usize,
    brace_depth: usize,
    string_tracker: StringTracker,
}

impl BracketTracker {
    fn consume(&mut self, ch: char) {
        let was_in_string = self.string_tracker.in_string();
        self.string_tracker.consume(ch);
        if was_in_string || self.string_tracker.in_string() {
            return;
        }

        match ch {
            '(' => self.paren_depth += 1,
            ')' => self.paren_depth = self.paren_depth.saturating_sub(1),
            '[' => self.bracket_depth += 1,
            ']' => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            '{' => self.brace_depth += 1,
            '}' => self.brace_depth = self.brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    fn should_split_on_comma(&self, ch: char) -> bool {
        if ch == ','
            && !self.string_tracker.in_string()
            && self.paren_depth == 0
            && self.bracket_depth == 0
            && self.brace_depth == 0
        {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_parameters_with_nested_tuples() {
        let input = "uint256 amount, (uint256, address) memory pair, string memory name";
        let parts = split_parameters(input);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "uint256 amount");
        assert_eq!(parts[1], "(uint256, address) memory pair");
        assert_eq!(parts[2], "string memory name");
    }

    #[test]
    fn parses_constructor_params() {
        let body = "constructor(address owner, uint256 initialSupply) public { }";
        let params = parse_constructor_params(body);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name.as_deref(), Some("owner"));
        assert_eq!(params[1].name.as_deref(), Some("initialSupply"));
    }

    #[test]
    fn ignores_abstract_contracts() {
        let source = r#"
            abstract contract AbstractExample { }
            contract Concrete {
                constructor() {}
            }
        "#;
        let contracts = parse_contracts_with_string_walker(source);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].name, "Concrete");
    }

    #[test]
    fn parses_multiple_contracts_in_one_file() {
        let source = r#"
            contract First {
                constructor(uint256 value) {}
            }
            contract Second {
                constructor() {}
            }
        "#;
        let contracts = parse_contracts_with_string_walker(source);
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts[0].name, "First");
        assert_eq!(contracts[1].name, "Second");
    }

    #[test]
    fn parses_complex_contract_with_modifiers_structs_and_assembly() {
        let source = r#"
            pragma solidity ^0.8.24;

            interface ICounter {
                function increment() external;
            }

            library MathLib {
                function scale(uint256 value) internal pure returns (uint256) {
                    return value * 2;
                }
            }

            abstract contract BaseDeployer {
                constructor(address admin) {}
            }

            contract AdvancedCounter is BaseDeployer {
                struct Config {
                    address owner;
                    uint256[] limits;
                }

                error InvalidConfig(string reason);

                constructor(
                    Config memory config,
                    function(address, uint256[] memory) external returns (bytes32) callback,
                    string memory label
                )
                    BaseDeployer(config.owner)
                    payable
                {
                    if (bytes(label).length == 0) {
                        revert InvalidConfig("missing label");
                    }

                    assembly {
                        let slot := mload(0x40)
                        mstore(slot, 1)
                    }
                }
            }
        "#;

        let contracts = parse_contracts_with_string_walker(source);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].name, "AdvancedCounter");
        assert_eq!(contracts[0].constructor_params.len(), 3);
        assert_eq!(
            contracts[0].constructor_params[0].raw,
            "Config memory config"
        );
        assert_eq!(
            contracts[0].constructor_params[1].name.as_deref(),
            Some("callback")
        );
        assert_eq!(
            contracts[0].constructor_params[2].name.as_deref(),
            Some("label")
        );
    }

    #[test]
    fn extracts_param_name_with_symbols() {
        assert_eq!(
            extract_param_name("uint256 indexed amount"),
            Some("amount".to_string())
        );
        assert_eq!(
            extract_param_name("tuple(uint256 a, uint256 b) memory pair"),
            Some("pair".to_string())
        );
    }

    #[test]
    fn strip_comments_preserves_strings() {
        let source = r#"
            // comment
            string constant sample = "http://example"; /* block comment */
            contract Test { string constant with_comment = "value // not comment"; }
        "#;
        let stripped = strip_comments(source);
        assert!(stripped.contains("http://example"));
        assert!(stripped.contains("// not comment"));
        assert!(!stripped.contains("// comment"));
        assert!(!stripped.contains("block comment"));
    }

    #[test]
    fn parser_backend_accepts_known_value() {
        assert_eq!(
            ParserBackend::parse("string-walker").unwrap(),
            ParserBackend::StringWalker
        );
    }

    #[test]
    fn parse_args_json_supports_raw_solidity_literals() {
        let args = parse_args_json(
            r#"[{"raw":"Config({owner: msg.sender, limits: [1, 2]})"},{"solidity":"callback"},"label"]"#,
        )
        .unwrap();

        assert_eq!(args[0], "Config({owner: msg.sender, limits: [1, 2]})");
        assert_eq!(args[1], "callback");
        assert_eq!(args[2], "\"label\"");
    }
}
