//! NuGet-channel static guards (2026-11-06 decision-check implementation
//! lane; findings cited by id). The executable pack behavior rides the
//! release transaction itself; this suite pins what must be TRUE WITHOUT
//! RUNNING DOTNET — the pairing between the RID table, the props selector
//! and the matrix asset names (M3), the release-transaction audit that only
//! the release job writes the release page (B2), the publish workflow's
//! gate/no-token/ordering structure (M5/M6/M7/M15), the path-like-argument
//! law for the pack/push verbs, and the payload-neutrality of the nuget
//! tree. The existence gate and the registration poll are additionally
//! table-run against stubbed registry fetches: the fail-closed exit classes
//! (200 / 404 / anything else) are falsifiable without NuGet.org.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn manifest() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn load(rel: &str) -> String {
    fs::read_to_string(format!("{}/{rel}", manifest())).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// The release machinery as the SET of workflow files under
/// `.github/workflows/` (sorted) — the same set `tests/npm_channel.rs`
/// welds, so the two channels stay one family.
fn load_workflows() -> String {
    let dir = format!("{}/.github/workflows", manifest());
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read dir {dir}: {e}"))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("read entry in {dir}: {e}"))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
        .iter()
        .map(|name| load(&format!(".github/workflows/{name}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn job_block(text: &str, job: &str) -> String {
    let anchor = format!("\n  {job}:\n");
    let start = text
        .find(&anchor)
        .unwrap_or_else(|| panic!("job {job} vanished from the workflow"));
    let rest = &text[start + 1..];
    // A job header is an indent-2 line ending in ':' — the first one after
    // this job's own header closes its block.
    let mut end = rest.len();
    let mut offset = 0usize;
    let mut first = true;
    for line in rest.lines() {
        if !first && line.starts_with("  ") && !line.starts_with("   ") && line.ends_with(':') {
            end = offset;
            break;
        }
        first = false;
        offset += line.len() + 1;
    }
    rest[..end].to_string()
}

fn write_stub(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, body).expect("write stub");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    path.to_str().expect("utf-8 stub path").to_string()
}

fn run_script(script: &str, args: &[&str]) -> std::process::Output {
    Command::new("sh")
        .arg(format!("{}/{}", manifest(), script))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run {script}: {e}"))
}

// ---------------------------------------------------------------------------
// M3: the RID table is welded to the matrix and to the props selector
// ---------------------------------------------------------------------------

/// rid | release-archive suffix | staged member — the finding R4 map.
const RID_ROWS: &[(&str, &str, &str)] = &[
    ("linux-x64", "x86_64-unknown-linux-musl.tar.gz", "archspec"),
    ("linux-arm64", "aarch64-unknown-linux-gnu.tar.gz", "archspec"),
    ("osx-x64", "x86_64-apple-darwin.tar.gz", "archspec"),
    ("osx-arm64", "aarch64-apple-darwin.tar.gz", "archspec"),
    ("win-x64", "x86_64-pc-windows-msvc.zip", "archspec.exe"),
];

#[test]
fn rid_rows_pair_with_the_matrix_and_the_props() {
    let pack = load("nuget/scripts/pack-tools.sh");
    let props = load("nuget/pkg/archspec.props");
    let mach = load_workflows();
    for (rid, suffix, member) in RID_ROWS {
        // The pack map names one release archive per RID (pack-tools.sh
        // spells the version as ${version}, the matrix jobs as ${ver}).
        assert!(
            pack.contains(&format!("{rid}|archspec-${{version}}-{suffix}|{member}")),
            "the pack map must carry the row {rid} -> {suffix}"
        );
        // The matrix pkg= lines spell the same bytes: POSIX jobs with the
        // ${ver} shell form, the PowerShell job with the $ver form.
        let ver_shell = format!("archspec-${{ver}}-{suffix}");
        let ver_ps = format!("archspec-$ver-{suffix}");
        assert!(
            mach.contains(ver_shell.as_str()) || mach.contains(ver_ps.as_str()),
            "the release machinery must emit {suffix} — matrix, pack map and props name the same bytes"
        );
        // The props table routes the RID to exactly that staged member.
        assert!(
            props.contains(&format!("runtimes/{rid}/native/{member}")),
            "the props table must route {rid} to runtimes/{rid}/native/{member}"
        );
    }
    // The musl-x64 edge (finding R4): linux-musl-x64 is folded onto the
    // static musl binary under linux-x64, never onto a missing row.
    let fold_line = props
        .lines()
        .find(|line| line.contains("<ArchspecPath") && line.contains("'linux-musl-x64'"))
        .expect("the props table must fold linux-musl-x64 onto a row");
    assert!(
        fold_line.contains("runtimes/linux-x64/native/archspec"),
        "linux-musl-x64 must route to the static musl binary under linux-x64"
    );
    // No asset ⇒ no RID: a row without a matrix asset would be a lie.
    for line in props.lines().filter(|l| l.contains("<ArchspecPath")) {
        for absent in ["linux-musl-arm64", "win-arm64", "win-x86", "linux-x86", "any"] {
            assert!(
                !line.contains(&format!("'{absent}'")),
                "the props table must not route {absent}: no matrix asset backs it — {line}"
            );
        }
    }
    // The Alpine-on-arm64 gap is named, not silently shipped (finding M3).
    assert!(
        props.contains("linux-musl-arm64") && props.contains("Alpine"),
        "the props comment must name the Alpine arm64 gap by RID and by OS"
    );
}

#[test]
fn props_refuses_unknown_rids_without_a_fallback() {
    let props = load("nuget/pkg/archspec.props");
    assert!(
        props.contains("<Error Condition=\"'$(ArchspecPath)' == ''\""),
        "an unset $(ArchspecPath) must raise an MSBuild error, not stay silent"
    );
    for rid in RID_ROWS {
        assert!(
            props.contains(rid.0),
            "the refusal text must list the supported RID {0}",
            rid.0
        );
    }
    // Every ArchspecPath assignment is conditional — an unconditional
    // default would be the silent-pass hole finding M3 refuses.
    for line in props.lines().filter(|l| l.contains("<ArchspecPath")) {
        assert!(
            line.contains("Condition="),
            "an unconditional ArchspecPath assignment is a fallback: {line}"
        );
    }
    // M4: the tools-only layout addresses the package folder directly.
    assert!(
        props.contains("$(MSBuildThisFileDirectory)../") && props.contains("runtimes/"),
        "$(ArchspecPath) must address $(MSBuildThisFileDirectory)../runtimes/... directly"
    );
}

// ---------------------------------------------------------------------------
// B2: one page transaction — only the release job writes the release page
// ---------------------------------------------------------------------------

#[test]
fn only_the_release_job_touches_the_release_page() {
    let release = load(".github/workflows/release.yml");
    let mach = load_workflows();
    let upload_script = load("release-assets.sh");
    // No workflow in the set calls gh release upload — the upload lives in
    // release-assets.sh, which exactly one job runs.
    assert!(
        !mach.contains("gh release upload"),
        "no workflow file may call gh release upload — finding B2: the pack must ride the release job's single upload transaction"
    );
    assert_eq!(
        upload_script.matches("gh release upload").count(),
        1,
        "release-assets.sh holds the one upload command of the release path"
    );
    let release_job = job_block(&release, "release");
    assert_eq!(release_job.matches("sh release-assets.sh").count(), 1);
    // The pack lands in dist BEFORE the single upload step runs.
    let pack_at = release_job
        .find("nuget/scripts/pack-tools.sh")
        .expect("the release job must pack the nupkg into the asset set");
    let upload_at = release_job
        .find("sh release-assets.sh")
        .expect("the release job must run the upload");
    assert!(
        pack_at < upload_at,
        "the pack step must precede the single upload transaction"
    );
    assert!(
        release_job.contains("actions/setup-dotnet@v4") && release_job.contains("dotnet-version"),
        "the release job pins the .NET SDK for the pack (finding M9)"
    );
    // The publishing jobs are read-only toward GitHub.
    for job in ["publish-npm", "publish-nuget"] {
        let block = job_block(&release, job);
        for verb in ["gh release upload", "gh release edit", "gh release create"] {
            assert!(!block.contains(verb), "job {job} must never {verb}");
        }
    }
    assert!(job_block(&release, "publish-nuget").contains("gh release download"));
}

// ---------------------------------------------------------------------------
// Workflow structure: gate, no-token doctrine, ordering, bounded poll
// ---------------------------------------------------------------------------

#[test]
fn workflow_carries_the_nuget_gate_and_the_publish_job() {
    let release = load(".github/workflows/release.yml");
    let mach = load_workflows();
    let nuget_job = job_block(&release, "publish-nuget");
    // Existence gate wired into BOTH the matrix publish-gate (before any
    // build — finding M7) and the job (a standalone re-run still refuses).
    assert!(mach.contains("sh nuget/scripts/gate-nuget-registry.sh"));
    assert!(nuget_job.contains("sh nuget/scripts/gate-nuget-registry.sh"));
    // Config comes from the repo variable and fails closed when unset
    // (the NPM_SCOPE pattern).
    assert!(release.contains("vars.NUGET_USER"));
    assert!(mach.contains("vars.NUGET_USER"));
    // Trusted publishing: OIDC, no token env class, login adjacent to push.
    assert!(nuget_job.contains("id-token: write"));
    assert!(nuget_job.contains("NuGet/login@v1"));
    assert!(release.contains("NUGET_API_KEY:-"));
    assert!(release.contains("NUGET_TOKEN:-"));
    for name in ["publish-npm", "publish-nuget", "build-matrix"] {
        let block = if name == "build-matrix" { &mach } else { &release };
        for wired in ["NUGET_API_KEY: ", "NUGET_TOKEN: ", "NUGET_PASSWORD: ", "NUGET_APIKEY: "] {
            for line in block.lines().filter(|l| l.contains(wired)) {
                assert!(
                    line.contains("steps.login.outputs.NUGET_API_KEY"),
                    "a workflow wires an API-key env from a non-login source: {line}"
                );
            }
        }
    }
    // The push names the exact NuGet.org source (no registry indirection).
    assert!(nuget_job.contains("--source https://api.nuget.org/v3/index.json"));
    // M15: the push is followed by the bounded poll, never by a standing
    // "awaiting validation" state.
    assert!(nuget_job.contains("assert-registration.sh"));
    assert!(nuget_job.contains("timeout-minutes: 30"));
    assert!(nuget_job.contains("--timeout-secs 1500"));
    assert!(nuget_job.contains("--space-secs 30"));
    // Exact-name download fails closed; the sidecar verifies the bytes.
    assert!(nuget_job.contains(".nupkg\" \"archspec-${ver}.nupkg.sha256")
        || nuget_job.contains(".nupkg.sha256\""));
    assert!(nuget_job.contains("sha256sum -c"));
}

#[test]
fn login_step_is_adjacent_to_the_push_step() {
    let release = load(".github/workflows/release.yml");
    let nuget_job = job_block(&release, "publish-nuget");
    let login_at = nuget_job
        .find("NuGet/login@v1")
        .expect("the login step must exist");
    let push_at = nuget_job
        .find("dotnet nuget push")
        .expect("the push step must exist");
    assert!(login_at < push_at, "the login step must precede the push step");
    // Exactly one step boundary between them: login then push, nothing in
    // between can burn the one-hour temp key (finding M6).
    let between = &nuget_job[login_at..push_at];
    assert_eq!(
        between.matches("\n      - ").count(),
        1,
        "another step sits between the login and the push — the temp key is 1 hour, one shot (finding M6)"
    );
    // Every slow step (gate, download, sidecar verify) precedes the login.
    for early in [
        "gate-nuget-registry.sh",
        "gh release download",
        "sha256sum -c",
    ] {
        let at = nuget_job.find(early).unwrap_or_else(|| panic!("{early} vanished"));
        assert!(at < login_at, "{early} must run before the key exists (finding M6)");
    }
    // The poll follows the push.
    let poll_at = nuget_job
        .find("assert-registration.sh")
        .expect("the poll step must exist");
    assert!(push_at < poll_at, "the registration poll must follow the push");
}

// ---------------------------------------------------------------------------
// The path-like-argument law for the pack/push verbs (the npm_channel.rs
// walker mirrored): a bare positional after a publish/push verb parses as a
// REGISTRY name, not a file. A vanished verb is the same failure class.
// ---------------------------------------------------------------------------

#[test]
fn nuget_push_and_pack_verbs_carry_path_like_arguments() {
    let dir = format!("{}/.github/workflows", manifest());
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read dir {dir}: {e}"))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("read entry in {dir}: {e}"))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    let mut saw_push = false;
    for name in &names {
        let text = load(&format!(".github/workflows/{name}"));
        for (idx, line) in text.lines().enumerate() {
            let lineno = idx + 1;
            let tokens: Vec<&str> = line.split_whitespace().collect();
            for (pos, token) in tokens.iter().enumerate() {
                if *token != "dotnet" {
                    continue;
                }
                let verb_at = match tokens.get(pos + 1) {
                    Some(t) if *t == "nuget" => pos + 2,
                    Some(t) if *t == "pack" => pos + 1,
                    _ => continue,
                };
                let verb = match tokens.get(verb_at) {
                    Some(v) if *v == "push" || *v == "pack" => *v,
                    _ => continue,
                };
                if verb == "push" {
                    saw_push = true;
                }
                let positional = tokens[verb_at + 1..]
                    .iter()
                    .copied()
                    .find(|t| !t.starts_with('-'));
                match positional {
                    Some(arg)
                        if arg.starts_with("./")
                            || arg.starts_with('/')
                            || arg.starts_with("\"./")
                            || arg.starts_with("\"/") => {}
                    Some(arg) => panic!(
                        "{name}:{lineno}: `dotnet nuget {verb}` takes the positional {arg:?} — a \
                         bare name parses as the registry package name or the current folder, not \
                         the file; pass it as \"./{arg}\" or an absolute path"
                    ),
                    None => {}
                }
            }
        }
    }
    assert!(
        saw_push,
        "no `dotnet nuget push` call in any file under .github/workflows/ — the publish \
         step vanished, so this guard would prove nothing"
    );
    // The pack verb lives in the pack script, not a workflow: its positional
    // must be the staged project path, never a bare name (finding B1: the
    // pack target is the stub PROJECT).
    let pack = load("nuget/scripts/pack-tools.sh");
    let pack_call = pack
        .lines()
        .find(|l| l.contains("archspec.csproj") && l.contains("pack"))
        .expect("pack-tools.sh must call dotnet pack on the stub project");
    assert!(
        pack_call.contains("/archspec.csproj"),
        "the pack verb must name the staged project by path: {pack_call}"
    );
}

// ---------------------------------------------------------------------------
// The stub project stays neutral and tools-only (B1/M4), the tree stays
// public-host-clean (M10)
// ---------------------------------------------------------------------------

#[test]
fn stub_project_is_neutral_and_tools_only() {
    let csproj = load("nuget/pkg/archspec.csproj");
    for knob in [
        "<IncludeBuildOutput>false</IncludeBuildOutput>",
        "<NoBuild>true</NoBuild>",
        "<NoPackageAnalysis>true</NoPackageAnalysis>",
        "<DevelopmentDependency>true</DevelopmentDependency>",
    ] {
        assert!(csproj.contains(knob), "the stub project must declare {knob}");
    }
    // Neutral sentinels: valid enough to pack, free of identity — the
    // identity arrives from pack-tools.sh at pack time (injection law).
    assert!(
        csproj.contains(">0.0.0</Version>"),
        "the committed version sentinel is 0.0.0"
    );
    assert!(
        csproj.contains("https://example.invalid/archspec"),
        "the committed repository-url sentinel is the reserved invalid placeholder"
    );
    assert!(
        !csproj.contains("github.com/"),
        "no payload template may name a repository — the URL is injected at pack time"
    );
}

#[test]
fn nuget_tree_hosts_stay_on_the_public_allow_list() {
    // Payload-neutrality of the nuget tree, structural form: every URL host
    // the tree names is public infrastructure (github.com — the release it
    // installs from; nuget.org — the registry it publishes to) or the
    // reserved/synthetic sentinel. The exact grep against the publish
    // guard's identifier list runs outside the payload, in the project's
    // internal test suite — a payload that names the list leaks it.
    let allowed = [
        "github.com",
        "api.nuget.org",
        "www.nuget.org",
        "nuget.org",
        "example.invalid",
    ];
    let root = format!("{}/nuget", manifest());
    let mut files = Vec::new();
    collect(&root, &mut files);
    assert!(files.len() >= 5, "the nuget tree must carry the whole channel: {files:#?}");
    for file in files {
        let bytes = fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {file}: {e}"));
        let mut rest = bytes.as_str();
        while let Some(at) = rest.find("://") {
            let tail = &rest[at + 3..];
            let host: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                .collect();
            if host.is_empty() {
                rest = tail;
                continue; // a bare "://" literal (a scanner's own needle), not a URL
            }
            assert!(
                allowed.contains(&host.as_str()),
                "host {host:?} in {file} is outside the public allow-list {allowed:?} — move such content out of the payload"
            );
            rest = tail;
        }
    }
}

// ---------------------------------------------------------------------------
// M7 table: the existence gate's exit classes against stubbed registry reads
// ---------------------------------------------------------------------------

fn collect(dir: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read dir {dir}: {e}")) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect(path.to_str().expect("utf-8 path"), out);
        } else {
            out.push(path.to_str().expect("utf-8 path").to_string());
        }
    }
}

fn gate_with_stub(stub_body: &str) -> std::process::Output {
    let tmp = tempfile::tempdir().expect("tempdir");
    let stub = write_stub(tmp.path(), "curl-stub.sh", stub_body);
    run_script(
        "nuget/scripts/gate-nuget-registry.sh",
        &["--pkg", "archspec", "--version", "1.2.3", "--curl", &stub],
    )
}

#[test]
fn existence_gate_exit_classes_are_the_fail_closed_table() {
    let exists = gate_with_stub("echo 200\nexit 0\n");
    assert_eq!(exists.status.code(), Some(1), "a published version must refuse the run");
    assert!(
        String::from_utf8_lossy(&exists.stderr).contains("republish-freeze"),
        "the refusal must name the law it enforces"
    );
    let free = gate_with_stub("echo 404\nexit 0\n");
    assert_eq!(free.status.code(), Some(0), "a free version must proceed");
    let unreadable = gate_with_stub("echo 500\nexit 0\n");
    assert_eq!(
        unreadable.status.code(),
        Some(1),
        "a registry that answers 5xx is not a green light"
    );
    let unreachable = gate_with_stub("echo 000\nexit 7\n");
    assert_eq!(
        unreachable.status.code(),
        Some(1),
        "an unreachable registry refuses — fail closed, never run blind"
    );
}

// ---------------------------------------------------------------------------
// M15 table: the bounded poll answers green / appear / TIMEOUT
// ---------------------------------------------------------------------------

fn poll_with_stub(stub: &str, extra_args: &[&str], count_file: Option<&Path>) -> std::process::Output {
    let tmp = tempfile::tempdir().expect("tempdir");
    let stub_path = write_stub(tmp.path(), "curl-stub.sh", stub);
    let mut command = Command::new("sh");
    command
        .arg(format!("{}/nuget/scripts/assert-registration.sh", manifest()))
        .args(["--pkg", "archspec", "--version", "1.2.3", "--curl", &stub_path])
        .args(extra_args);
    if let Some(count) = count_file {
        command.env("NUGET_STUB_COUNT", count);
    }
    command.output().expect("run poll")
}

#[test]
fn registration_poll_is_bounded_and_loud_on_timeout() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let counter = tmp.path().join("poll.count");

    let registered = poll_with_stub("echo 200\nexit 0\n", &[], None);
    assert_eq!(
        registered.status.code(),
        Some(0),
        "an already-registered version is green on the first read"
    );

    let appeared = poll_with_stub(
        "c=0\n[ -f \"$NUGET_STUB_COUNT\" ] && c=$(cat \"$NUGET_STUB_COUNT\")\nc=$((c+1))\necho \"$c\" > \"$NUGET_STUB_COUNT\"\nif [ \"$c\" -ge 3 ]; then echo 200; else echo 404; fi\nexit 0\n",
        &["--timeout-secs", "30", "--space-secs", "0"],
        Some(&counter),
    );
    assert_eq!(
        appeared.status.code(),
        Some(0),
        "a version that appears during the poll is green"
    );

    let timed_out = poll_with_stub(
        "echo 404\nexit 0\n",
        &["--timeout-secs", "1", "--space-secs", "0"],
        None,
    );
    assert_eq!(
        timed_out.status.code(),
        Some(1),
        "the never-appearing class is red, never a standing state"
    );
    assert!(
        String::from_utf8_lossy(&timed_out.stderr).contains("TIMEOUT"),
        "the timeout must name itself: {}",
        String::from_utf8_lossy(&timed_out.stderr)
    );
    // The real deadlines (1500 s / 30 s, step timeout 30 min) are pinned in
    // the workflow by the structure test, not re-executed here.
}

// ---------------------------------------------------------------------------
// The payload docs name the channel (dotnet-level usage, leak-safe prose)
// ---------------------------------------------------------------------------

#[test]
fn readme_names_the_nuget_channel_at_the_dotnet_level() {
    let readme = load("README.md");
    assert!(
        readme.contains("dotnet add package archspec"),
        "the README install surface must carry the NuGet line"
    );
    assert!(
        readme.contains("$(ArchspecPath)"),
        "the README must name the consumer-facing property the props file exposes"
    );
    assert!(
        readme.contains("Alpine"),
        "the README must name the Alpine-on-arm64 gap, not ship it silently"
    );
    // The props file and the README name the same supported RID set.
    let props = load("nuget/pkg/archspec.props");
    for rid in RID_ROWS {
        assert!(
            readme.contains(rid.0),
            "the README must name the supported RID {}",
            rid.0
        );
        assert!(props.contains(rid.0), "the props table must name the RID {}", rid.0);
    }
}
