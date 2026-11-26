//! The three-engine differential runner: the same randomized inputs
//! through Ruby, Rust, and TS (where reachable), the verdicts compared,
//! the drifts emitted as a machine-readable report the dashboard
//! consumes.
//!
//! Inputs are mined from the vendored fixture corpus through the shared
//! generators (sampled rows, mutated rows, facet recombinations) with a
//! deterministic seed, so a run replays byte-identically:
//!
//!   differential --all --ts-root /path/to/pubid-ts
//!
//! External engines run through bridge scripts
//! (`bridges/ruby_bridge.rb`, `bridges/ts_bridge.mjs`): the cases go in
//! as one JSON file, the verdicts come back as one JSON object per
//! line, an engine header first. An engine that cannot run (no gem, no
//! npm tree, no interpreter) is reported unavailable — recorded, never
//! a silent skip. Drifts never gate: this is a dashboard data
//! generator. The report schema lives at
//! `schema/differential-report.schema.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};

use pubid_core::conformance::{flavor_impl, implemented_flavors};
use pubid_core::generators::{attested_universe, mutate_row};

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

struct Options {
    flavors: Vec<&'static str>,
    rows: usize,
    mutants: usize,
    recombinations: usize,
    seed: u64,
    inputs: Option<PathBuf>,
    ruby_command: String,
    ruby_bridge: PathBuf,
    ts_command: String,
    ts_root: Option<PathBuf>,
    ts_bridge: PathBuf,
    out: Option<PathBuf>,
    stamp: Option<String>,
    include_agree: bool,
}

fn default_bridge(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bridges")
        .join(name)
}

fn parse_options() -> Options {
    let mut options = Options {
        flavors: Vec::new(),
        rows: 8,
        mutants: 8,
        recombinations: 8,
        seed: 0,
        inputs: None,
        ruby_command: "ruby".to_string(),
        ruby_bridge: default_bridge("ruby_bridge.rb"),
        ts_command: "node".to_string(),
        ts_root: None,
        ts_bridge: default_bridge("ts_bridge.mjs"),
        out: None,
        stamp: None,
        include_agree: false,
    };
    let mut args = std::env::args().skip(1);
    let mut all = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--all" => all = true,
            "--flavor" => {
                let name = args.next().unwrap_or_else(|| die("--flavor needs a name"));
                match implemented_flavors()
                    .into_iter()
                    .find(|known| *known == name)
                {
                    Some(known) => options.flavors.push(known),
                    None => die(&format!("unknown flavor {name:?} (not ported here)")),
                }
            }
            "--rows" => options.rows = parse_usize(&mut args, "--rows"),
            "--mutants" => options.mutants = parse_usize(&mut args, "--mutants"),
            "--recombinations" => {
                options.recombinations = parse_usize(&mut args, "--recombinations")
            }
            "--seed" => {
                let value = args.next().unwrap_or_else(|| die("--seed needs a number"));
                options.seed = value
                    .parse()
                    .unwrap_or_else(|_| die("--seed needs a number"));
            }
            "--inputs" => {
                options.inputs = Some(PathBuf::from(
                    args.next().unwrap_or_else(|| die("--inputs needs a file")),
                ));
            }
            "--ruby" => {
                options.ruby_command = args.next().unwrap_or_else(|| die("--ruby needs a command"))
            }
            "--ruby-bridge" => {
                options.ruby_bridge = PathBuf::from(
                    args.next()
                        .unwrap_or_else(|| die("--ruby-bridge needs a path")),
                );
            }
            "--ts" => {
                options.ts_command = args.next().unwrap_or_else(|| die("--ts needs a command"))
            }
            "--ts-root" => {
                options.ts_root = Some(PathBuf::from(
                    args.next()
                        .unwrap_or_else(|| die("--ts-root needs a directory")),
                ));
            }
            "--ts-bridge" => {
                options.ts_bridge = PathBuf::from(
                    args.next()
                        .unwrap_or_else(|| die("--ts-bridge needs a path")),
                );
            }
            "--out" => {
                options.out = Some(PathBuf::from(
                    args.next().unwrap_or_else(|| die("--out needs a file")),
                ));
            }
            "--stamp" => {
                options.stamp = Some(args.next().unwrap_or_else(|| die("--stamp needs a value")));
            }
            "--include-agree" => options.include_agree = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => die(&format!("unknown argument {other:?}")),
        }
    }
    if all {
        options.flavors = implemented_flavors().into_iter().collect();
    }
    options
}

fn parse_usize(args: &mut impl Iterator<Item = String>, flag: &str) -> usize {
    args.next()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| die(&format!("{flag} needs a number")))
}

fn die(message: &str) -> ! {
    eprintln!("{message}");
    print_help();
    std::process::exit(2);
}

fn print_help() {
    eprintln!(
        "usage: differential [--all | --flavor <name>]... [--rows N] [--mutants N] [--recombinations N]\
         \n  [--seed S] [--inputs file.json] [--out file.json] [--stamp iso8601] [--include-agree]\
         \n  [--ruby cmd] [--ruby-bridge path] [--ts cmd] [--ts-root dir] [--ts-bridge path]\
         \n  default: 8 rows + 8 mutants + 8 recombinations per flavor, every ported flavor;\
         \n  the Ruby engine needs the pubid gem (or PUBID_RUBY_LIB pointing at a checkout's lib/);\
         \n  the TS engine needs --ts-root pointing at a pubid-ts checkout (or npm tree with dist/)"
    );
}

// ---------------------------------------------------------------------------
// Deterministic randomness (splitmix64; no external crate)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            (self.next_u64() % bound as u64) as usize
        }
    }
}

// ---------------------------------------------------------------------------
// Case generation
// ---------------------------------------------------------------------------

/// One differential input and where it came from.
#[derive(Clone, Debug)]
struct Case {
    flavor: &'static str,
    input: String,
    origin: &'static str,
}

fn generate_cases(options: &Options) -> Vec<Case> {
    if let Some(path) = &options.inputs {
        return cases_from_file(path);
    }
    let universe = attested_universe();
    let mut rng = Rng::new(options.seed);
    let mutation_chars = ['ʄ', '€', '𝕏', '-', '0', ' '];
    let mut cases = Vec::new();
    for flavor in &universe.flavors {
        if !options.flavors.is_empty() && !options.flavors.contains(&flavor.name) {
            continue;
        }
        for _ in 0..options.rows {
            let Some(row) = flavor.rows.get(rng.below(flavor.rows.len())) else {
                continue;
            };
            cases.push(Case {
                flavor: flavor.name,
                input: row.clone(),
                origin: "row",
            });
        }
        for _ in 0..options.mutants {
            let Some(base) = flavor.rows.get(rng.below(flavor.rows.len())) else {
                continue;
            };
            let input = mutate_row(
                base,
                rng.below(8),
                rng.next_u64() as usize,
                mutation_chars[rng.below(mutation_chars.len())],
                rng.below(flavor.tokens.len().max(1)),
                &flavor.tokens,
            );
            cases.push(Case {
                flavor: flavor.name,
                input,
                origin: "mutant",
            });
        }
        for _ in 0..options.recombinations {
            let Some(template) = flavor.templates.get(rng.below(flavor.templates.len())) else {
                continue;
            };
            let picks: Vec<usize> = (0..template.slots.len())
                .map(|_| rng.next_u64() as usize)
                .collect();
            cases.push(Case {
                flavor: flavor.name,
                input: template.render(&picks),
                origin: "recombination",
            });
        }
    }
    cases
}

/// Explicit inputs (`--inputs file.json`): `[{"flavor": "...", "input": "..."}, ...]`.
fn cases_from_file(path: &std::path::Path) -> Vec<Case> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|err| die(&format!("{path:?}: {err}")));
    let rows: Vec<Value> =
        serde_json::from_str(&text).unwrap_or_else(|err| die(&format!("{path:?}: {err}")));
    rows.into_iter()
        .map(|row| {
            let flavor_name = row
                .get("flavor")
                .and_then(Value::as_str)
                .unwrap_or_else(|| die(&format!("{path:?}: row has no flavor")));
            let known = implemented_flavors()
                .into_iter()
                .find(|name| *name == flavor_name)
                .unwrap_or_else(|| die(&format!("{path:?}: flavor {flavor_name:?} is not ported")));
            Case {
                flavor: known,
                input: row
                    .get("input")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                origin: "explicit",
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Engine verdicts
// ---------------------------------------------------------------------------

/// One engine's verdict on one case.
#[derive(Clone, Debug, PartialEq)]
struct Verdict {
    /// `ok` | `reject` | `unimplemented` | `error`
    status: String,
    hash: Option<Value>,
    human: Option<String>,
    urn: Option<String>,
    /// The reject error code or the error detail.
    detail: Option<String>,
}

impl Verdict {
    fn ok(hash: Value, human: String, urn: Option<String>) -> Self {
        Self {
            status: "ok".to_string(),
            hash: Some(hash),
            human: Some(human),
            urn,
            detail: None,
        }
    }

    fn reject(detail: impl Into<String>) -> Self {
        Self {
            status: "reject".to_string(),
            hash: None,
            human: None,
            urn: None,
            detail: Some(detail.into()),
        }
    }

    fn unimplemented() -> Self {
        Self {
            status: "unimplemented".to_string(),
            hash: None,
            human: None,
            urn: None,
            detail: None,
        }
    }

    fn error(detail: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            hash: None,
            human: None,
            urn: None,
            detail: Some(detail.into()),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "status": self.status,
            "hash": self.hash,
            "human": self.human,
            "urn": self.urn,
            "detail": self.detail,
        })
    }

    /// Comparable verdicts carry a parse decision (not an engine gap).
    fn comparable(&self) -> bool {
        self.status == "ok" || self.status == "reject"
    }
}

/// An engine's availability record for the report header.
#[derive(Clone, Debug)]
struct EngineInfo {
    available: bool,
    version: Option<String>,
    detail: Option<String>,
}

impl EngineInfo {
    fn to_json(&self) -> Value {
        json!({
            "available": self.available,
            "version": self.version,
            "detail": self.detail,
        })
    }
}

/// The Rust engine: in-process, always available.
fn run_rust(cases: &[Case]) -> (EngineInfo, Vec<Verdict>) {
    let verdicts = cases
        .iter()
        .map(|case| {
            let Some(impl_) = flavor_impl(case.flavor) else {
                return Verdict::unimplemented();
            };
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| impl_.parse(&case.input)));
            match result {
                Ok(Ok(identifier)) => {
                    let urn = identifier.to_urn();
                    Verdict::ok(identifier.to_hash(), identifier.to_human(), urn)
                }
                Ok(Err(err)) => Verdict::reject(err.to_string()),
                Err(payload) => Verdict::error(panic_detail(&payload)),
            }
        })
        .collect();
    (
        EngineInfo {
            available: true,
            version: Some(concat!("pubid-rs ", env!("CARGO_PKG_VERSION")).to_string()),
            detail: None,
        },
        verdicts,
    )
}

fn panic_detail(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_else(|| "panic".to_string())
}

/// An external engine driven by a bridge script.
struct ExternalEngine {
    name: &'static str,
    command: String,
    bridge: PathBuf,
    extra_args: Vec<String>,
}

impl ExternalEngine {
    /// Run every case through the bridge. Returns the engine info (with
    /// availability) and the per-case verdicts (empty when unavailable).
    fn run(&self, cases: &[Case]) -> (EngineInfo, Vec<Verdict>) {
        match self.try_run(cases) {
            Ok((info, verdicts)) => (info, verdicts),
            Err(err) => (
                EngineInfo {
                    available: false,
                    version: None,
                    detail: Some(format!("engine {}: {err}", self.name)),
                },
                Vec::new(),
            ),
        }
    }

    fn try_run(&self, cases: &[Case]) -> Result<(EngineInfo, Vec<Verdict>), String> {
        if cases.is_empty() {
            return Ok((
                EngineInfo {
                    available: true,
                    version: None,
                    detail: None,
                },
                Vec::new(),
            ));
        }
        let case_file = std::env::temp_dir().join(format!(
            "pubid-differential-{}-{:016x}.json",
            std::process::id(),
            Rng::new(0x1D_1FF).next_u64(),
        ));
        let payload: Vec<Value> = cases
            .iter()
            .enumerate()
            .map(|(i, case)| json!({ "i": i, "flavor": case.flavor, "input": case.input }))
            .collect();
        std::fs::write(
            &case_file,
            serde_json::to_string(&payload).expect("cases serialize"),
        )
        .map_err(|err| format!("case file: {err}"))?;

        let output = Command::new(&self.command)
            .arg(&self.bridge)
            .arg("--cases")
            .arg(&case_file)
            .args(&self.extra_args)
            .output();
        let _ = std::fs::remove_file(&case_file);
        let output = output
            .map_err(|err| format!("spawn {} {}: {err}", self.command, self.bridge.display()))?;
        if !output.status.success() {
            return Err(format!(
                "{} exited with {}: {}",
                self.bridge.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty());

        // The engine header: availability + version.
        let header: Value = lines
            .next()
            .and_then(|line| serde_json::from_str(line).ok())
            .ok_or_else(|| format!("{}: no engine header on stdout", self.bridge.display()))?;
        if header.get("available").and_then(Value::as_bool) != Some(true) {
            return Ok((
                EngineInfo {
                    available: false,
                    version: None,
                    detail: Some(
                        header
                            .get("detail")
                            .and_then(Value::as_str)
                            .unwrap_or("unavailable")
                            .to_string(),
                    ),
                },
                Vec::new(),
            ));
        }
        let version = Some(
            header
                .get("version")
                .map(|version| match version {
                    Value::Null => "unknown".to_string(),
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(|| "unknown".to_string()),
        );

        let mut verdicts = vec![Verdict::error("missing"); cases.len()];
        let mut filled = 0usize;
        for line in lines {
            let Ok(verdict) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let index = match verdict.get("i").and_then(Value::as_u64) {
                Some(index) if (index as usize) < verdicts.len() => index as usize,
                _ => continue,
            };
            verdicts[index] = verdict_from_json(verdict);
            filled += 1;
        }
        if filled < cases.len() {
            return Err(format!(
                "{}: {} of {} verdicts returned",
                self.bridge.display(),
                filled,
                cases.len()
            ));
        }
        Ok((
            EngineInfo {
                available: true,
                version,
                detail: None,
            },
            verdicts,
        ))
    }
}

fn verdict_from_json(verdict: Value) -> Verdict {
    let status = verdict
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("error")
        .to_string();
    let detail = verdict
        .get("error_code")
        .map(|code| match code {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .or_else(|| {
            verdict
                .get("detail")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    Verdict {
        status,
        hash: verdict.get("hash").cloned().filter(|hash| !hash.is_null()),
        human: verdict
            .get("human")
            .and_then(Value::as_str)
            .map(str::to_string),
        urn: verdict
            .get("urn")
            .and_then(Value::as_str)
            .map(str::to_string),
        detail,
    }
}

// ---------------------------------------------------------------------------
// Delta classification
// ---------------------------------------------------------------------------

/// The differential delta of one case across the available engines.
#[derive(Clone, Debug)]
struct Delta {
    class: String,
    /// Which fields drifted (output drift) or the per-engine statuses
    /// (acceptance drift).
    detail: Option<Value>,
}

fn classify(verdicts: &BTreeMap<&str, &Verdict>) -> Delta {
    let comparable: Vec<(&str, &Verdict)> = verdicts
        .iter()
        .filter(|(_, verdict)| verdict.comparable())
        .map(|(name, verdict)| (*name, *verdict))
        .collect();
    if comparable.len() < 2 {
        let statuses: Value = verdicts
            .iter()
            .map(|(name, verdict)| (name.to_string(), Value::String(verdict.status.clone())))
            .collect();
        return Delta {
            class: "single_engine".to_string(),
            detail: Some(statuses),
        };
    }
    let statuses: Vec<String> = comparable
        .iter()
        .map(|(name, verdict)| format!("{name}={}", verdict.status))
        .collect();
    let acceptances = comparable.iter().all(|(_, verdict)| verdict.status == "ok");
    let all_rejects = comparable
        .iter()
        .all(|(_, verdict)| verdict.status == "reject");
    if !acceptances && !all_rejects {
        return Delta {
            class: "acceptance_drift".to_string(),
            detail: Some(json!(statuses)),
        };
    }
    if all_rejects {
        return Delta {
            class: "agree".to_string(),
            detail: None,
        };
    }
    // All ok: compare the three fields pairwise across engines.
    let mut fields = Vec::new();
    for (name, verdict) in &comparable[1..] {
        let (base_name, base) = comparable[0];
        if base.hash != verdict.hash {
            fields.push(format!("{base_name}.hash vs {name}.hash"));
        }
        if base.human != verdict.human {
            fields.push(format!("{base_name}.human vs {name}.human"));
        }
        if base.urn != verdict.urn {
            fields.push(format!("{base_name}.urn vs {name}.urn"));
        }
    }
    if fields.is_empty() {
        Delta {
            class: "agree".to_string(),
            detail: None,
        }
    } else {
        Delta {
            class: "output_drift".to_string(),
            detail: Some(json!(fields)),
        }
    }
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

fn build_report(
    options: &Options,
    cases: &[Case],
    engines: &BTreeMap<&'static str, EngineInfo>,
    verdicts: &BTreeMap<&'static str, Vec<Verdict>>,
) -> Value {
    let mut by_class: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_flavor: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut case_rows = Vec::new();
    let mut drifts = 0u64;

    for (i, case) in cases.iter().enumerate() {
        let mut engine_verdicts: BTreeMap<&str, &Verdict> = BTreeMap::new();
        for (name, list) in verdicts {
            if let Some(verdict) = list.get(i) {
                engine_verdicts.insert(name, verdict);
            }
        }
        let delta = classify(&engine_verdicts);
        *by_class.entry(delta.class.clone()).or_insert(0) += 1;
        let flavor_tally = by_flavor.entry(case.flavor.to_string()).or_default();
        *flavor_tally.entry("cases".to_string()).or_insert(0) += 1;
        if delta.class == "agree" {
            *flavor_tally.entry("agree".to_string()).or_insert(0) += 1;
        } else if delta.class != "single_engine" {
            *flavor_tally.entry("drifts".to_string()).or_insert(0) += 1;
            drifts += 1;
        }
        if options.include_agree || delta.class != "agree" {
            let engines_json: Value = Value::Object(
                engine_verdicts
                    .iter()
                    .map(|(name, verdict)| (name.to_string(), verdict.to_json()))
                    .collect(),
            );
            case_rows.push(json!({
                "flavor": case.flavor,
                "input": case.input,
                "origin": case.origin,
                "delta_class": delta.class,
                "detail": delta.detail,
                "engines": engines_json,
            }));
        }
    }

    let engines_json: Value = engines
        .iter()
        .map(|(name, info)| (name.to_string(), info.to_json()))
        .collect();
    let by_flavor_json: Value = by_flavor
        .iter()
        .map(|(flavor, tally)| (flavor.clone(), json!(tally)))
        .collect();
    json!({
        "schema": "pubid-differential-report/1",
        "generator": {
            "name": "differential",
            "crate": "pubid-rs",
            "seed": options.seed,
            "rows": options.rows,
            "mutants": options.mutants,
            "recombinations": options.recombinations,
            "inputs": if options.inputs.is_some() { "explicit" } else { "corpus-mined" },
            "stamp": options.stamp,
        },
        "engines": engines_json,
        "summary": {
            "cases": cases.len(),
            "drifts": drifts,
            "by_delta_class": json!(by_class),
            "by_flavor": by_flavor_json,
        },
        "cases": case_rows,
    })
}

fn main() {
    let options = parse_options();
    let cases = generate_cases(&options);
    if cases.is_empty() {
        eprintln!("no cases to run (check the flavor filter)");
        std::process::exit(2);
    }

    let (rust_info, rust_verdicts) = run_rust(&cases);
    let ruby = ExternalEngine {
        name: "ruby",
        command: options.ruby_command.clone(),
        bridge: options.ruby_bridge.clone(),
        extra_args: Vec::new(),
    };
    let (ruby_info, ruby_verdicts) = ruby.run(&cases);
    let mut extra_args = Vec::new();
    if let Some(root) = &options.ts_root {
        extra_args.push("--ts-root".to_string());
        extra_args.push(root.to_string_lossy().into_owned());
    }
    let ts = ExternalEngine {
        name: "ts",
        command: options.ts_command.clone(),
        bridge: options.ts_bridge.clone(),
        extra_args,
    };
    let (ts_info, ts_verdicts) = ts.run(&cases);

    let mut engines = BTreeMap::new();
    engines.insert("rust", rust_info);
    engines.insert("ruby", ruby_info);
    engines.insert("ts", ts_info);
    let mut verdicts = BTreeMap::new();
    verdicts.insert("rust", rust_verdicts);
    if !ruby_verdicts.is_empty() {
        verdicts.insert("ruby", ruby_verdicts);
    }
    if !ts_verdicts.is_empty() {
        verdicts.insert("ts", ts_verdicts);
    }

    for (name, info) in &engines {
        let state = if info.available {
            format!("available {}", info.version.clone().unwrap_or_default())
        } else {
            format!(
                "unavailable ({})",
                info.detail
                    .clone()
                    .unwrap_or_else(|| "no engine".to_string())
            )
        };
        eprintln!("engine {name}: {state}");
    }

    let report = build_report(&options, &cases, &engines, &verdicts);
    let text = serde_json::to_string_pretty(&report).expect("report serializes");
    match &options.out {
        Some(path) => {
            std::fs::write(path, &text)
                .unwrap_or_else(|err| die(&format!("write {}: {err}", path.display())));
            eprintln!(
                "report: {} ({} cases, {} drifts)",
                path.display(),
                report["summary"]["cases"],
                report["summary"]["drifts"]
            );
        }
        None => println!("{text}"),
    }
}
