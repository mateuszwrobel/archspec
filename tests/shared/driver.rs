use crate::common;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Output;

/// The three language drivers the feature-matrix suite runs scenarios against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Csharp,
    Go,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::Rust, Language::Csharp, Language::Go];

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Csharp => "csharp",
            Language::Go => "go",
        }
    }
}

/// A language-agnostic description of a codebase tree. Scenarios express their
/// fixture in this logical form; the driver materializes it as actual files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogicalTree {
    /// Logical unit names, e.g. `app`, `HomeBudget.Domain`.
    pub units: Vec<String>,
    /// Unit -> logical module paths (dotted or `::`-separated), e.g.
    /// `app -> ["core", "ui"]`, `HomeBudget.Domain -> ["Domain.Entities"]`.
    pub modules: BTreeMap<String, Vec<String>>,
    /// (from_unit, to_unit) logical hard edges.
    pub hard_edges: Vec<(String, String)>,
    /// (unit, from_module, to_module). `to_module` is a logical target; if it
    /// names a module in the tree it becomes a module edge, otherwise it is an
    /// external dependency reference.
    pub module_usings: Vec<(String, String, String)>,
    /// Unit -> package/dependency names (Rust crates, C# NuGet packages, Go
    /// modules).
    pub packages: BTreeMap<String, Vec<String>>,
    /// Unit -> IsPackable/publish flag (C#).
    pub is_packable: BTreeMap<String, bool>,
    /// External namespace/crate targets referenced by usings/imports.
    pub external_targets: Vec<String>,
    /// C# central package references: (fixture-relative dir, package). Each
    /// entry materializes as a central `PackageReference` item plus a
    /// `PackageVersion` entry in that dir's `Directory.Packages.props`; the
    /// project csprojs stay clean (central package management).
    pub central_packages: Vec<(String, String)>,
    /// C# props version-only entries: (fixture-relative dir, package) written
    /// as `PackageVersion` items no project references (transitive pins).
    pub central_version_entries: Vec<(String, String)>,
    /// C# conditional central references: (fixture-relative dir, package)
    /// written as `PackageReference` items bearing
    /// `Condition="'$(IsTestProject)'=='true'"` plus a `PackageVersion` pin —
    /// the props shape real test projects are wired with.
    pub central_conditional_packages: Vec<(String, String)>,
}

impl LogicalTree {
    pub fn new() -> Self {
        Self::default()
    }
}

/// A materializer + scanner facade for one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Driver {
    pub language: Language,
}

impl Driver {
    pub fn all() -> Vec<Driver> {
        Language::ALL
            .iter()
            .map(|language| Driver { language: *language })
            .collect()
    }

    /// The canonical tree exercising every model tier the driver can populate:
    /// two units with a hard edge, module structure with a module edge, and an
    /// external dependency. Shape is language-appropriate.
    pub fn probe_tree(&self) -> LogicalTree {
        let mut tree = LogicalTree::new();
        match self.language {
            Language::Rust => {
                tree.units = vec!["app".into(), "shared".into()];
                tree.modules.insert("app".into(), vec!["core".into(), "ui".into()]);
                tree.modules.insert("shared".into(), vec!["models".into()]);
                tree.hard_edges.push(("app".into(), "shared".into()));
                tree.module_usings
                    .push(("app".into(), "core".into(), "ui".into()));
                tree.module_usings
                    .push(("app".into(), "ui".into(), "serde".into()));
                tree.packages.insert("app".into(), vec!["serde".into()]);
                tree.external_targets.push("serde".into());
            }
            Language::Csharp => {
                tree.units = vec!["app".into(), "shared".into()];
                tree.modules.insert("app".into(), vec!["Core".into(), "Ui".into()]);
                tree.modules.insert("shared".into(), vec!["Models".into()]);
                tree.hard_edges.push(("app".into(), "shared".into()));
                tree.module_usings
                    .push(("app".into(), "Core".into(), "Ui".into()));
                tree.module_usings
                    .push(("app".into(), "Ui".into(), "Newtonsoft.Json".into()));
                tree.packages
                    .insert("app".into(), vec!["Newtonsoft.Json".into()]);
                tree.is_packable.insert("shared".into(), false);
                tree.external_targets.push("Newtonsoft.Json".into());
            }
            Language::Go => {
                tree.units = vec!["app".into(), "shared".into()];
                tree.modules.insert("app".into(), Vec::new());
                tree.modules.insert("shared".into(), Vec::new());
                tree.hard_edges.push(("app".into(), "shared".into()));
                tree.packages
                    .insert("app".into(), vec!["example.com/third/party".into()]);
                tree.external_targets
                    .push("example.com/third/party".into());
            }
        }
        tree
    }

    /// Materialize a `LogicalTree` as actual files in the fixture. Per-language:
    /// - Rust: workspace `Cargo.toml` + one crate per unit, a file per module
    ///   with `mod x;` wiring, path deps for hard edges, manifest deps for
    ///   packages.
    /// - C#: one `.csproj` per unit (ProjectReference for hard edges,
    ///   PackageReference for packages, IsPackable when set), a `.cs` file per
    ///   module with a file-scoped namespace, usings for module/external refs.
    /// - Go: `go.mod` (+ require block for declared packages) and one package
    ///   dir per unit; imports for hard edges and module_usings targets, blank
    ///   imports for declared packages not otherwise imported.
    pub fn materialize(&self, fx: &common::Fixture, tree: &LogicalTree) {
        match self.language {
            Language::Rust => materialize_rust(fx, tree),
            Language::Csharp => materialize_csharp(fx, tree),
            Language::Go => materialize_go(fx, tree),
        }
    }

    /// Map a logical unit name to the concrete unit name in the model.
    /// Rust/C#: identity. Go: `example.com/demo/<unit>` (go.mod module prefix).
    pub fn unit_name(&self, logical: &str) -> String {
        match self.language {
            Language::Rust | Language::Csharp => logical.to_string(),
            Language::Go => format!("example.com/demo/{logical}"),
        }
    }

    /// Map a logical module path (dotted or `::`) to the concrete `::` module
    /// path in the model. Rust/C# prefix the unit name; Go has no soft module
    /// tier, so the module maps to the unit itself.
    pub fn module_path(&self, logical_unit: &str, logical_module: &str) -> String {
        match self.language {
            Language::Rust | Language::Csharp => format!(
                "{}::{}",
                self.unit_name(logical_unit).replace('.', "::"),
                logical_module.replace('.', "::")
            ),
            Language::Go => self.unit_name(logical_unit),
        }
    }

    /// Run `archspec scan` on the fixture and return the model as JSON. Panics
    /// on a non-zero exit or invalid JSON so probes/scenarios fail loudly.
    pub fn scan(&self, fx: &common::Fixture) -> serde_json::Value {
        let output = fx.run(&["scan"]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "scan must exit 0 (stderr: {})",
            common::stderr(&output)
        );
        assert!(
            common::stderr(&output).is_empty(),
            "scan must not write stderr: {}",
            common::stderr(&output)
        );
        serde_json::from_str(&common::stdout(&output)).expect("scan stdout must be JSON")
    }

    /// Run `archspec` with arbitrary args in the fixture, returning the raw
    /// process output.
    pub fn run(&self, fx: &common::Fixture, args: &[&str]) -> Output {
        fx.run(args)
    }
}

/// Normalize a logical module path (dotted or `::`) to `::`-separated form.
fn normalized(module: &str) -> String {
    module.replace('.', "::")
}

fn materialize_rust(fx: &common::Fixture, tree: &LogicalTree) {
    let members: Vec<&str> = tree.units.iter().map(String::as_str).collect();
    let workspace = format!(
        "[workspace]\nmembers = [{}]\n",
        members
            .iter()
            .map(|m| format!("\"{m}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    fx.write("Cargo.toml", &workspace);

    let unit_names: BTreeSet<String> = tree.units.iter().cloned().collect();
    let all_modules: BTreeSet<String> = tree
        .modules
        .values()
        .flatten()
        .map(|m| normalized(m))
        .collect();

    for unit in &tree.units {
        let mut manifest = format!(
            "[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        );
        let deps: Vec<&String> = tree
            .hard_edges
            .iter()
            .filter(|(from, _)| from == unit)
            .map(|(_, to)| to)
            .collect();
        let pkgs: Vec<&String> = tree.packages.get(unit).into_iter().flatten().collect();
        if !deps.is_empty() || !pkgs.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for to in &deps {
                manifest.push_str(&format!("{to} = {{ path = \"../{to}\" }}\n"));
            }
            for pkg in &pkgs {
                manifest.push_str(&format!("{pkg} = \"1\"\n"));
            }
        }
        fx.write(&format!("{unit}/Cargo.toml"), &manifest);

        let modules: Vec<String> = tree.modules.get(unit).cloned().unwrap_or_default();
        let normalized_modules: Vec<String> = modules.iter().map(|m| normalized(m)).collect();

        let mut top: BTreeSet<String> = BTreeSet::new();
        for m in &normalized_modules {
            if let Some(first) = m.split("::").next() {
                top.insert(first.to_string());
            }
        }
        let mut lib = String::new();
        for name in top {
            lib.push_str(&format!("mod {name};\n"));
        }
        fx.write(&format!("{unit}/src/lib.rs"), &lib);

        // file key (path rel to src/, no extension) -> (child mods, use lines).
        let mut files: BTreeMap<String, (BTreeSet<String>, Vec<String>)> = BTreeMap::new();
        for m in &normalized_modules {
            let parts: Vec<&str> = m.split("::").collect();
            for k in 1..=parts.len() {
                files.entry(parts[..k].join("/")).or_default();
            }
        }
        for m in &normalized_modules {
            let parts: Vec<&str> = m.split("::").collect();
            for k in 1..parts.len() {
                let parent = parts[..k].join("/");
                files.entry(parent).or_default().0.insert(parts[k].to_string());
            }
        }
        for (u, from, to) in &tree.module_usings {
            if u != unit {
                continue;
            }
            let key = normalized(from).replace("::", "/");
            let to_norm = normalized(to);
            let use_line = if all_modules.contains(&to_norm) {
                format!("use crate::{to_norm};")
            } else if to_norm
                .split("::")
                .next()
                .map(|first| unit_names.contains(first))
                .unwrap_or(false)
            {
                format!("use {to_norm};")
            } else {
                format!("use {to};")
            };
            files.entry(key).or_default().1.push(use_line);
        }

        for (key, (mods, usings)) in &files {
            let leaf = key.rsplit('/').next().unwrap_or("").to_string();
            let mut content = String::new();
            for m in mods {
                content.push_str(&format!("mod {m};\n"));
            }
            for u in usings {
                content.push_str(&format!("{u}\n"));
            }
            content.push_str(&format!("pub fn {leaf}() {{}}\n"));
            fx.write(&format!("{unit}/src/{key}.rs"), &content);
        }
    }
}

/// Resolve a C# using target to the concrete dotted namespace to write: the
/// owning unit's namespace when the target names a module in the tree, the
/// target itself otherwise (external).
fn resolve_csharp_using(tree: &LogicalTree, to: &str) -> String {
    let to_norm = normalized(to);
    for (unit, modules) in &tree.modules {
        for module in modules {
            if normalized(module) == to_norm {
                return format!("{unit}.{module}");
            }
        }
    }
    to.to_string()
}

fn materialize_csharp(fx: &common::Fixture, tree: &LogicalTree) {
    for unit in &tree.units {
        let mut csproj = String::from(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n",
        );
        if let Some(packable) = tree.is_packable.get(unit) {
            csproj.push_str(&format!("    <IsPackable>{packable}</IsPackable>\n"));
        }
        csproj.push_str("  </PropertyGroup>\n");
        let deps: Vec<&String> = tree
            .hard_edges
            .iter()
            .filter(|(from, _)| from == unit)
            .map(|(_, to)| to)
            .collect();
        let pkgs: Vec<&String> = tree.packages.get(unit).into_iter().flatten().collect();
        if !deps.is_empty() || !pkgs.is_empty() {
            csproj.push_str("  <ItemGroup>\n");
            for to in &deps {
                csproj.push_str(&format!(
                    "    <ProjectReference Include=\"..\\{to}\\{to}.csproj\" />\n"
                ));
            }
            for pkg in &pkgs {
                csproj.push_str(&format!("    <PackageReference Include=\"{pkg}\" />\n"));
            }
            csproj.push_str("  </ItemGroup>\n");
        }
        csproj.push_str("</Project>\n");
        fx.write(&format!("{unit}/{unit}.csproj"), &csproj);

        let modules: Vec<String> = tree.modules.get(unit).cloned().unwrap_or_default();
        for module in &modules {
            let leaf = module.rsplit('.').next().unwrap_or(module);
            let file = format!("{unit}/{}.cs", module.replace('.', "/"));
            let mut usings: BTreeSet<String> = BTreeSet::new();
            for (u, from, to) in &tree.module_usings {
                if u == unit && from == module {
                    usings.insert(resolve_csharp_using(tree, to));
                }
            }
            let mut content = String::new();
            for u in &usings {
                content.push_str(&format!("using {u};\n"));
            }
            content.push_str(&format!(
                "namespace {unit}.{module};\npublic class {leaf} {{ }}\n"
            ));
            fx.write(&file, &content);
        }
    }
    materialize_csharp_props(fx, tree);
}

/// Props entries per directory: (unconditional references, version-only
/// pins, conditional references).
type PropsEntries<'a> = (BTreeSet<&'a String>, BTreeSet<&'a String>, BTreeSet<&'a String>);

/// Write the declared central-package props files: one
/// `Directory.Packages.props` per dir that carries central entries, central
/// references as `PackageReference` + `PackageVersion` items, unreferenced
/// entries as version-only `PackageVersion` items, conditional references as
/// `Condition`-bearing `PackageReference` items + `PackageVersion` items.
fn materialize_csharp_props(fx: &common::Fixture, tree: &LogicalTree) {
    let mut by_dir: BTreeMap<&String, PropsEntries> = BTreeMap::new();
    for (dir, package) in &tree.central_packages {
        by_dir.entry(dir).or_default().0.insert(package);
    }
    for (dir, package) in &tree.central_version_entries {
        by_dir.entry(dir).or_default().1.insert(package);
    }
    for (dir, package) in &tree.central_conditional_packages {
        by_dir.entry(dir).or_default().2.insert(package);
    }
    for (dir, (references, versions, conditional)) in by_dir {
        let mut props = String::from(
            "<Project>\n  <PropertyGroup>\n    <ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally>\n  </PropertyGroup>\n  <ItemGroup>\n",
        );
        for package in references {
            props.push_str(&format!(
                "    <PackageReference Include=\"{package}\" Version=\"8.4.0\" />\n    \
                 <PackageVersion Include=\"{package}\" Version=\"8.4.0\" />\n"
            ));
        }
        for package in &conditional {
            props.push_str(&format!(
                "    <PackageReference Include=\"{package}\" Version=\"8.4.0\" \
                 Condition=\"'$(IsTestProject)'=='true'\" />\n    \
                 <PackageVersion Include=\"{package}\" Version=\"8.4.0\" />\n"
            ));
        }
        for package in versions {
            props.push_str(&format!(
                "    <PackageVersion Include=\"{package}\" Version=\"1.2.3\" />\n"
            ));
        }
        props.push_str("  </ItemGroup>\n</Project>\n");
        let rel = if dir == "." {
            "Directory.Packages.props".to_string()
        } else {
            format!("{dir}/Directory.Packages.props")
        };
        fx.write(&rel, &props);
    }
}

fn materialize_go(fx: &common::Fixture, tree: &LogicalTree) {
    let mut gomod = String::from("module example.com/demo\ngo 1.21\n");
    let all_packages: BTreeSet<&String> = tree.packages.values().flatten().collect();
    if !all_packages.is_empty() {
        gomod.push_str("\nrequire (\n");
        for pkg in &all_packages {
            gomod.push_str(&format!("\t{pkg} v0.0.0\n"));
        }
        gomod.push_str(")\n");
    }
    fx.write("go.mod", &gomod);
    for unit in &tree.units {
        let mut imports: BTreeSet<String> = BTreeSet::new();
        let mut blank_imports: BTreeSet<String> = BTreeSet::new();
        for (from, to) in &tree.hard_edges {
            if from == unit {
                imports.insert(format!("example.com/demo/{to}"));
            }
        }
        for (u, _from, to) in &tree.module_usings {
            if u == unit {
                imports.insert(to.clone());
            }
        }
        for pkg in tree.packages.get(unit).into_iter().flatten() {
            if !imports.contains(pkg.as_str()) {
                blank_imports.insert(pkg.clone());
            }
        }
        if imports.is_empty() && blank_imports.is_empty() {
            fx.write(&format!("{unit}/{unit}.go"), &format!("package {unit}\n"));
            continue;
        }
        let mut content = format!("package {unit}\n\nimport (\n");
        for imp in &imports {
            content.push_str(&format!("\t\"{imp}\"\n"));
        }
        for pkg in &blank_imports {
            content.push_str(&format!("\t_ \"{pkg}\"\n"));
        }
        content.push_str(")\n");
        fx.write(&format!("{unit}/{unit}.go"), &content);
    }
}

/// Materialize the canonical two-member Go workspace: a root `go.work` using
/// `./api` and `./store`, each member with its own `go.mod`, the `api` root
/// package and its nested `internal/handler` package importing the `store`
/// module. No `go.mod` at the workspace root — go.work membership is the only
/// module fact. Shared by the scan and depgraph workspace tests; test targets
/// that compile the shared tree without these tests never reference it.
#[allow(dead_code)]
pub fn materialize_go_workspace(fx: &common::Fixture) {
    fx.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fx.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fx.write(
        "api/api.go",
        "package api\n\nimport \"example.com/store\"\n\nfunc Api() {}\n",
    );
    fx.write(
        "api/internal/handler/handler.go",
        "package handler\n\nimport (\n\t\"fmt\"\n\n\t\"example.com/store\"\n)\n\nfunc Handle() {}\n",
    );
    fx.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fx.write("store/store.go", "package store\n\nfunc Get() {}\n");
}

/// Materialize the canonical single-module Go tree for the declared-grouping
/// derivation: one `go.mod`, packages `a`, `shell`, `store` under
/// `example.com/demo`, imports `a -> shell -> store`, no module tier in the
/// model — the spec's declarations create the grouping. Shared by the go
/// laundering and submodule-contract probes and scenarios; test targets that
/// compile the shared tree without these tests never reference it.
#[allow(dead_code)]
pub fn materialize_go_declared_grouping(fx: &common::Fixture) {
    fx.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fx.write(
        "a/a.go",
        "package a\n\nimport \"example.com/demo/shell\"\n\nfunc A() {}\n",
    );
    fx.write(
        "shell/shell.go",
        "package shell\n\nimport \"example.com/demo/store\"\n\nfunc Shell() {}\n",
    );
    fx.write("store/store.go", "package store\n\nfunc Get() {}\n");
}

/// Materialize the single-module Go tree for the declared-grouping submodule
/// contract path: `auth` with a nested `entity` package it imports under
/// `example.com/demo`. The spec's submodule boundary claims the entity
/// package by import path. Test targets that compile the shared tree without
/// these tests never reference it.
#[allow(dead_code)]
pub fn materialize_go_declared_submodule(fx: &common::Fixture) {
    fx.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fx.write(
        "auth/auth.go",
        "package auth\n\nimport \"example.com/demo/auth/entity\"\n\nfunc Auth() {}\n",
    );
    fx.write("auth/entity/entity.go", "package entity\n\nfunc Get() {}\n");
}