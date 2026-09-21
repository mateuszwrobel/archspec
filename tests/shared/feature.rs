use crate::common;
use crate::shared::driver::{
    materialize_go_declared_grouping, materialize_go_declared_submodule, materialize_go_workspace,
    Driver, Language, LogicalTree,
};

/// High-level capability. One per feature column in the matrix report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    ScanUnits,
    ScanSoftStructure,
    ScanModuleEdges,
    ScanExternal,
    ScanModuleExternal,
    ScanUnitManifests,
    VerifyModuleBoundaries,
    VerifyNoCycles,
    VerifyForbidExternal,
    VerifyManifestIntegrity,
    UpdateSeed,
    ReportMetrics,
    InspectFileLevel,
    InspectStructuralTree,
    InspectStructuralScanner,
    Depgraph,
    Diagram,
    Init,
    HelpDiagnostics,
    VerifyRootFacade,
    VerifyForbiddenLaundering,
    VerifySubmoduleContracts,
    CliArtefactFreshness,
}

impl Feature {
    pub const ALL: [Feature; 23] = [
        Feature::ScanUnits,
        Feature::ScanSoftStructure,
        Feature::ScanModuleEdges,
        Feature::ScanExternal,
        Feature::ScanModuleExternal,
        Feature::ScanUnitManifests,
        Feature::VerifyModuleBoundaries,
        Feature::VerifyNoCycles,
        Feature::VerifyForbidExternal,
        Feature::VerifyManifestIntegrity,
        Feature::UpdateSeed,
        Feature::ReportMetrics,
        Feature::InspectFileLevel,
        Feature::InspectStructuralTree,
        Feature::InspectStructuralScanner,
        Feature::Depgraph,
        Feature::Diagram,
        Feature::Init,
        Feature::HelpDiagnostics,
        Feature::VerifyRootFacade,
        Feature::VerifyForbiddenLaundering,
        Feature::VerifySubmoduleContracts,
        Feature::CliArtefactFreshness,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Feature::ScanUnits => "scan.units",
            Feature::ScanSoftStructure => "scan.soft_structure",
            Feature::ScanModuleEdges => "scan.module_edges",
            Feature::ScanExternal => "scan.external",
            Feature::ScanModuleExternal => "scan.module_external",
            Feature::ScanUnitManifests => "scan.unit_manifests",
            Feature::VerifyModuleBoundaries => "verify.module_boundaries",
            Feature::VerifyNoCycles => "verify.no_cycles",
            Feature::VerifyForbidExternal => "verify.forbid_external",
            Feature::VerifyManifestIntegrity => "verify.manifest_integrity",
            Feature::UpdateSeed => "update.seed",
            Feature::ReportMetrics => "report.metrics",
            Feature::InspectFileLevel => "inspect.file_level",
            Feature::InspectStructuralTree => "inspect.structural_tree",
            Feature::InspectStructuralScanner => "inspect.structural_scanner",
            Feature::Depgraph => "depgraph",
            Feature::Diagram => "diagram",
            Feature::Init => "init",
            Feature::HelpDiagnostics => "help.diagnostics",
            Feature::VerifyRootFacade => "verify.root_facade",
            Feature::VerifyForbiddenLaundering => "verify.forbidden_laundering",
            Feature::VerifySubmoduleContracts => "verify.submodule_contracts",
            Feature::CliArtefactFreshness => "cli.artefact_freshness",
        }
    }

    /// CAPABILITY PROBE: executes the driver on a canonical tree and returns
    /// true iff the feature's model tier (or command path) is actually
    /// populated for this language. This is the code-derived capability — no
    /// manual support table. Probes build their own minimal fixtures.
    pub fn probe(&self, driver: &Driver, _fx: &common::Fixture) -> bool {
        match self {
            Feature::ScanUnits => true,
            Feature::ScanSoftStructure => scan_tier_populated(driver, "soft_structure"),
            Feature::ScanModuleEdges => scan_tier_populated(driver, "module_edges"),
            Feature::ScanExternal => scan_tier_populated(driver, "external"),
            Feature::ScanModuleExternal => scan_tier_populated(driver, "module_external"),
            Feature::ScanUnitManifests => scan_tier_populated(driver, "unit_manifests"),
            Feature::VerifyModuleBoundaries => probe_verify_boundaries(driver),
            Feature::VerifyNoCycles => probe_verify_no_cycles(driver),
            Feature::VerifyForbidExternal => probe_verify_forbid_external(driver),
            Feature::VerifyManifestIntegrity => probe_verify_manifest(driver),
            Feature::UpdateSeed => probe_update(driver),
            Feature::ReportMetrics => probe_report(driver),
            Feature::InspectFileLevel => probe_inspect_file(driver),
            Feature::InspectStructuralTree => probe_inspect_structural_tree(driver),
            Feature::InspectStructuralScanner => probe_inspect_structural_scanner(driver),
            Feature::Depgraph => probe_depgraph(driver),
            Feature::Diagram => probe_diagram(driver),
            Feature::Init => probe_init(driver),
            Feature::HelpDiagnostics => probe_help_diagnostics(driver),
            Feature::VerifyRootFacade => probe_verify_root_facade(driver),
            Feature::VerifyForbiddenLaundering => probe_verify_laundering(driver),
            Feature::VerifySubmoduleContracts => probe_verify_submodule_contracts(driver),
            Feature::CliArtefactFreshness => probe_artefact_freshness(driver),
        }
    }
}

/// Scan a canonical tree and check a named model tier is non-empty. Go keeps
/// every soft/manifest tier empty, so it reports not-implemented there.
fn scan_tier_populated(driver: &Driver, tier: &str) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let model = driver.scan(&fx);
    match tier {
        "soft_structure" => model["soft_structure"]
            .as_object()
            .map(|object| !object.is_empty())
            .unwrap_or(false),
        "module_edges" => model["module_edges"]
            .as_array()
            .map(|array| !array.is_empty())
            .unwrap_or(false),
        "external" => model["external"]
            .as_array()
            .map(|array| !array.is_empty())
            .unwrap_or(false),
        "module_external" => model["module_external"]
            .as_object()
            .map(|object| !object.is_empty())
            .unwrap_or(false),
        "unit_manifests" => model["unit_manifests"]
            .as_object()
            .map(|object| !object.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

/// A spec declaring one boundary per canonical unit with the app->shared edge
/// allowed. Verifies clean for every driver.
fn boundary_spec(driver: &Driver) -> String {
    format!(
        "[project]\nlanguage = \"{}\"\n\n\
         [[module]]\nname = \"app\"\nmatches = {{ units = [\"{}\"] }}\n\
         [module.allowed]\ndepend_on = [\"shared\"]\n\n\
         [[module]]\nname = \"shared\"\nmatches = {{ units = [\"{}\"] }}\n",
        driver.language.as_str(),
        driver.unit_name("app"),
        driver.unit_name("shared")
    )
}

fn probe_verify_boundaries(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = driver.run(&fx, &["verify"]);
    output.status.code() == Some(0)
        && common::stdout(&output).contains("matches source model")
}

fn probe_verify_no_cycles(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
    driver.materialize(&fx, &tree);
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{}\"] }}\n\
             [module.allowed]\ndepend_on = [\"a\"]\n\n\
             [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\n",
            driver.language.as_str(),
            driver.unit_name("a"),
            driver.unit_name("b")
        ),
    );
    let output = driver.run(&fx, &["verify"]);
    output.status.code() != Some(0)
}

fn probe_verify_forbid_external(driver: &Driver) -> bool {
    let ext = match driver.language {
        Language::Rust => "serde",
        Language::Csharp => "Newtonsoft.Json",
        Language::Go => "example.com/third/party",
    };
    let fx = common::Fixture::new();
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["Persistence".into()]);
    tree.module_usings
        .push(("app".into(), "Persistence".into(), ext.into()));
    tree.packages.insert("app".into(), vec![ext.into()]);
    driver.materialize(&fx, &tree);
    let module = driver.module_path("app", "Persistence");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{module}\"]\nforbid = [\"{ext}\"]\n",
            driver.language.as_str(),
            driver.unit_name("app")
        ),
    );
    let output = driver.run(&fx, &["verify"]);
    output.status.code() != Some(0)
}

fn probe_verify_manifest(driver: &Driver) -> bool {
    let pkg = match driver.language {
        Language::Rust => "serde",
        Language::Csharp => "Newtonsoft.Json",
        Language::Go => "somepkg",
    };
    let fx = common::Fixture::new();
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.packages.insert("app".into(), vec![pkg.into()]);
    driver.materialize(&fx, &tree);
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{}\"] }}\n\n\
             [[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"{pkg}\"]\n",
            driver.language.as_str(),
            driver.unit_name("app")
        ),
    );
    let output = driver.run(&fx, &["verify"]);
    output.status.code() != Some(0)
}

fn probe_update(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let output = driver.run(&fx, &["update"]);
    output.status.code() == Some(0) && fx.path("architecture.spec.toml").exists()
}

fn probe_report(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = driver.run(&fx, &["report"]);
    let text = common::stdout(&output);
    output.status.code() == Some(0)
        && text.contains("Result:")
        && text.contains("components:")
}

fn probe_inspect_file(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let output = driver.run(&fx, &["inspect"]);
    let text = common::stdout(&output);
    output.status.code() == Some(0)
        && (text.contains("graph TD") || text.contains("@startuml"))
}

/// Structural tree view: unit subgraphs with module nodes and module edges.
/// The probe records the behavior, not the language: it materializes a tree
/// whose model carries a module tier — for rust/csharp the canonical probe
/// tree, for Go a `go.work` workspace (the shape where Go populates the
/// tier). A tier-less model is refused by that missing fact, which the
/// matrix must not credit as a rendered view.
fn probe_inspect_structural_tree(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    match driver.language {
        Language::Go => materialize_go_workspace(&fx),
        _ => driver.materialize(&fx, &driver.probe_tree()),
    }
    let output = driver.run(&fx, &["inspect", "tree"]);
    let text = common::stdout(&output);
    output.status.code() == Some(0) && text.contains("graph TD")
}

/// Structural scanner view: unit-tier edges between unit subgraphs. Probed
/// with the exact command the behavior scenario runs, so a driver is only
/// credited when the unit-edge rendering itself works.
fn probe_inspect_structural_scanner(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let output = driver.run(&fx, &["inspect", "scanner"]);
    let text = common::stdout(&output);
    output.status.code() == Some(0) && text.contains("graph TD")
}

fn probe_depgraph(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let output = driver.run(&fx, &["depgraph", "modules"]);
    let text = common::stdout(&output);
    output.status.code() == Some(0) && text.contains("graph TD")
}

fn probe_diagram(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{}\"] }}\n",
            driver.language.as_str(),
            driver.unit_name("app")
        ),
    );
    let output = driver.run(&fx, &["diagram"]);
    output.status.code() == Some(0) && common::stdout(&output).contains("graph TD")
}

fn probe_init(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let output = driver.run(&fx, &["init"]);
    output.status.code() == Some(0) && fx.path("architecture.spec.toml").exists()
}

fn probe_help_diagnostics(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    let output = driver.run(&fx, &["help", "diagnostics"]);
    output.status.code() == Some(0)
        && common::stdout(&output).contains("laundered forbidden edge")
}

/// The root facade needs the publication-surface fact `root-facade`. The
/// capability table is the single source of truth — verify's inert-rule note
/// reads that same row — so a driver whose row is `not-emitted` (csharp, go)
/// can never fire the rule: the probe declares the fact unavailable, exactly
/// how every other not-emitted fact renders skipped, rather than grepping a
/// stdout string. Only a driver emitting the fact at full granularity (rust)
/// runs the end-to-end fixture proving the violation actually fires. Grepping
/// the bare rule name would be vacuous: the inert note "facade dependency rule
/// inert for <lang>" mentions the rule name without the rule ever firing.
fn probe_verify_root_facade(driver: &Driver) -> bool {
    if !crate::shared::capability::table().granular(driver.language.as_str(), "root-facade") {
        return false;
    }
    let fx = common::Fixture::new();
    fx.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("src/lib.rs", "mod engine;\npub use engine::Thing;\n");
    fx.write(
        "src/engine.rs",
        "pub struct Thing;\nuse crate::Thing;\npub fn make() -> Thing { Thing }\n",
    );
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n",
    );
    let output = driver.run(&fx, &["verify"]);
    common::stdout(&output).contains("facade dependency:")
}

/// A single unit whose modules route an edge through a module no boundary
/// claims: `a -> hidden -> b`, with `shell` the only module holding the unit
/// catch-all, so `hidden` resolves through the unit fallback (the conduit).
/// Exercises the laundering rule on model edges, language-agnostically.
fn conduit_tree() -> LogicalTree {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["a".into(), "b".into(), "hidden".into(), "shell".into()],
    );
    tree.module_usings
        .push(("app".into(), "a".into(), "hidden".into()));
    tree.module_usings
        .push(("app".into(), "hidden".into(), "b".into()));
    tree
}

/// Bans `a -> b` while allowing only the route through `shell`; the hop into
/// `hidden` is unit-fallback-owned, so the ban route is a conduit.
fn conduit_spec(driver: &Driver) -> String {
    let app = driver.unit_name("app");
    let a = driver.module_path("app", "a");
    let b = driver.module_path("app", "b");
    let shell = driver.module_path("app", "shell");
    format!(
        "[project]\nlanguage = \"{}\"\n\n\
         [[module]]\nname = \"a\"\nmatches = {{ modules = [\"{a}\"] }}\n\
         [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"b\"]\n\n\
         [[module]]\nname = \"b\"\nmatches = {{ modules = [\"{b}\"] }}\n\n\
         [[module]]\nname = \"shell\"\nmatches = {{ modules = [\"{shell}\"], units = [\"{app}\"] }}\n\
         [module.allowed]\ndepend_on = [\"b\"]\n",
        driver.language.as_str()
    )
}

fn probe_verify_laundering(driver: &Driver) -> bool {
    if driver.language == Language::Go {
        // Go honesty note: the declared-grouping derivation (spec modules over
        // a single go.mod tree) proves the check enforces on go. The native
        // derivation cannot be probed from the canonical two-member workspace
        // fixture — laundering needs a third member to ride, so no go.work
        // shape in this harness expresses the conduit; native enforcement is
        // covered end-to-end by tests/verify.rs
        // (verify_go_work_members_enforce_laundered_forbidden_edge).
        let fx = common::Fixture::new();
        materialize_go_declared_grouping(&fx);
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
             [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n\
             [[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\
             [module.allowed]\ndepend_on = [\"store\"]\n\n\
             [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
        );
        let output = driver.run(&fx, &["verify"]);
        return common::stdout(&output).contains("laundered forbidden edge: a -> store via shell");
    }
    let fx = common::Fixture::new();
    driver.materialize(&fx, &conduit_tree());
    fx.write("architecture.spec.toml", &conduit_spec(driver));
    let output = driver.run(&fx, &["verify"]);
    common::stdout(&output).contains("laundered forbidden edge")
}

/// One unit with module `b` containing submodule `b.c`, for the submodule
/// contract enforcement path (needs a soft module tier). Dotted so both rust
/// and c# materialize the nested file.
fn submodule_tree() -> LogicalTree {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into()];
    tree.modules
        .insert("a".into(), vec!["b".into(), "b.c".into()]);
    tree
}

/// A submodule contract on `a::b` forbidding stereotype `c`; module `a::b::c`
/// sits under the boundary, so the contract leaks.
fn submodule_spec(driver: &Driver) -> String {
    let a = driver.unit_name("a");
    let ab = driver.module_path("a", "b");
    format!(
        "[project]\nlanguage = \"{}\"\n\n\
         [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
         [[stereotype]]\nname = \"c\"\nmatch = {{ names = [\"c\"] }}\n\n\
         [[module.submodules]]\nname = \"{ab}\"\nmatches = {{ modules = [\"{ab}\"] }}\n\
         contract = {{ forbid = [\"c\"] }}\n",
        driver.language.as_str()
    )
}

fn probe_verify_submodule_contracts(driver: &Driver) -> bool {
    if driver.language == Language::Go {
        // Same honesty note as the laundering probe: the declared derivation
        // (a submodule boundary claiming a package by import path) proves the
        // check enforces on go; native workspace submodules are covered by
        // tests/verify.rs, not this probe.
        let fx = common::Fixture::new();
        materialize_go_declared_submodule(&fx);
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[stereotype]]\nname = \"entity\"\nmatch = { paths = [\"example.com/demo/auth/entity\"] }\n\n\
             [[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\", \"example.com/demo/auth/entity\"] }\n\n\
             [[module.submodules]]\nname = \"auth::entity\"\nmatches = { modules = [\"example.com/demo/auth/entity\"] }\n\
             contract = { forbid = [\"entity\"] }\n",
        );
        let output = driver.run(&fx, &["verify"]);
        return common::stdout(&output).contains("contract leak");
    }
    let fx = common::Fixture::new();
    driver.materialize(&fx, &submodule_tree());
    fx.write("architecture.spec.toml", &submodule_spec(driver));
    let output = driver.run(&fx, &["verify"]);
    common::stdout(&output).contains("contract leak")
}

fn probe_artefact_freshness(driver: &Driver) -> bool {
    let fx = common::Fixture::new();
    driver.materialize(&fx, &driver.probe_tree());
    let make = driver.run(&fx, &["scan", "--output", "model.json"]);
    if make.status.code() != Some(0) {
        return false;
    }
    let fresh = driver.run(&fx, &["scan", "--check", "--output", "model.json"]);
    if fresh.status.code() != Some(0) {
        return false;
    }
    fx.write("model.json", "STALE ON PURPOSE\n");
    let stale = driver.run(&fx, &["scan", "--check", "--output", "model.json"]);
    stale.status.code() != Some(0)
        && common::stderr(&stale).contains("out of date model")
}