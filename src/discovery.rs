use crate::{
    config,
    model::{Target, Task, Workspace},
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

fn task(target: &Target, name: &str, argv: &[&str], stage: bool) -> Task {
    Task {
        name: name.into(),
        target: target.name.clone(),
        provider: target.manager.clone(),
        argv: argv.iter().map(|s| s.to_string()).collect(),
        cwd: target.path.clone(),
        env: BTreeMap::new(),
        depends_on: vec![],
        availability: None,
        build_stage: stage,
        mutates_source: false,
        stdout_must_be_empty: false,
    }
}

fn insert(target: &mut Target, name: &str, argv: &[&str], stage: bool) {
    let value = task(target, name, argv, stage);
    target.tasks.insert(name.into(), value);
}

fn unavailable(target: &mut Target, name: &str, reason: &str) {
    let mut value = task(target, name, &[], false);
    value.availability = Some(reason.into());
    target.tasks.entry(name.into()).or_insert(value);
}

pub fn discover_target(name: &str, path: &Path, explicit: Option<&str>) -> Result<Target> {
    let mut candidates = vec![];
    for (filename, builder) in [
        ("package.json", "node/app"),
        ("pyproject.toml", "python/package"),
        ("Cargo.toml", "rust/app"),
        ("go.mod", "go/app"),
        ("go.work", "go/app"),
        ("pom.xml", "java/maven"),
        ("build.gradle", "java/gradle"),
        ("build.gradle.kts", "java/gradle"),
        ("build.xml", "java/ant"),
        ("Chart.yaml", "helm/chart"),
        ("Dockerfile", "docker/image"),
    ] {
        if path.join(filename).is_file() && !candidates.contains(&builder) {
            candidates.push(builder);
        }
    }
    if path.join("setup.py").is_file() && !candidates.contains(&"python/package") {
        candidates.push("python/package");
    }
    if path.join("requirements.txt").is_file() && !candidates.contains(&"python/package") {
        candidates.push("python/app");
    }
    let builder = if let Some(value) = explicit {
        value.to_string()
    } else {
        if candidates.len() != 1 {
            bail!(
                "{}: expected one builder, found {:?}; select uses in build.yaml",
                path.display(),
                candidates
            );
        }
        candidates[0].to_string()
    };
    let mut target = Target {
        name: name.into(),
        builder: builder.clone(),
        manager: builder.clone(),
        path: path.into(),
        version: "0.0.0".into(),
        tasks: BTreeMap::new(),
    };
    match builder.as_str() {
        "node/app" | "node/package" => node(&mut target)?,
        "python/app" | "python/package" | "python/library" => python(&mut target)?,
        "go/app" | "go/library" => {
            target.manager = "go".into();
            insert(&mut target, "install", &["go", "mod", "download"], false);
            insert(&mut target, "build", &["go", "build", "./..."], true);
            insert(&mut target, "test", &["go", "test", "./..."], true);
            insert(&mut target, "lint", &["go", "vet", "./..."], true);
            insert(&mut target, "format", &["gofmt", "-w", "."], false);
            target.tasks.get_mut("format").unwrap().mutates_source = true;
            insert(&mut target, "format-check", &["gofmt", "-l", "."], true);
            target
                .tasks
                .get_mut("format-check")
                .unwrap()
                .stdout_must_be_empty = true;
        }
        "rust/app" | "rust/library" => {
            target.manager = "cargo".into();
            let value: toml::Value = toml::from_str(&fs::read_to_string(path.join("Cargo.toml"))?)?;
            if let Some(version) = value
                .get("package")
                .and_then(|v| v.get("version"))
                .and_then(|v| v.as_str())
            {
                target.version = version.into();
            }
            insert(
                &mut target,
                "install",
                &["cargo", "fetch", "--locked"],
                false,
            );
            insert(
                &mut target,
                "build",
                &["cargo", "build", "--locked", "--workspace"],
                true,
            );
            insert(
                &mut target,
                "test",
                &["cargo", "test", "--locked", "--workspace"],
                true,
            );
            insert(
                &mut target,
                "lint",
                &[
                    "cargo",
                    "clippy",
                    "--locked",
                    "--workspace",
                    "--",
                    "-D",
                    "warnings",
                ],
                true,
            );
            insert(&mut target, "format", &["cargo", "fmt", "--all"], false);
            target.tasks.get_mut("format").unwrap().mutates_source = true;
            insert(
                &mut target,
                "format-check",
                &["cargo", "fmt", "--all", "--check"],
                true,
            );
        }
        "java/maven" => {
            target.manager = "maven".into();
            let text = fs::read_to_string(path.join("pom.xml"))?;
            let doc = roxmltree::Document::parse(&text)?;
            if let Some(version) = doc
                .root_element()
                .children()
                .find(|v| v.has_tag_name("version"))
                .and_then(|v| v.text())
            {
                target.version = version.into();
            }
            let executable = if path.join("mvnw").is_file() {
                if cfg!(windows) {
                    "mvnw.cmd"
                } else {
                    "./mvnw"
                }
            } else {
                "mvn"
            };
            insert(
                &mut target,
                "install",
                &[executable, "-B", "dependency:go-offline"],
                false,
            );
            insert(&mut target, "build", &[executable, "-B", "verify"], true);
            insert(&mut target, "test", &[executable, "-B", "test"], false);
        }
        "java/gradle" => {
            target.manager = "gradle".into();
            let executable = if path.join("gradlew").is_file() {
                if cfg!(windows) {
                    "gradlew.bat"
                } else {
                    "./gradlew"
                }
            } else {
                "gradle"
            };
            insert(
                &mut target,
                "build",
                &[executable, "--no-daemon", "build"],
                true,
            );
            insert(
                &mut target,
                "test",
                &[executable, "--no-daemon", "test"],
                false,
            );
        }
        "java/ant" => {
            target.manager = "ant".into();
            let text = fs::read_to_string(path.join("build.xml"))?;
            let doc = roxmltree::Document::parse(&text)?;
            let default = doc.root_element().attribute("default");
            for element in doc
                .root_element()
                .children()
                .filter(|v| v.has_tag_name("target"))
            {
                if let Some(name) = element.attribute("name") {
                    insert(
                        &mut target,
                        name,
                        &["ant", name],
                        Some(name) == default || name == "test",
                    );
                }
            }
        }
        "helm/chart" => {
            target.manager = "helm".into();
            let value: serde_yaml::Value =
                serde_yaml::from_str(&fs::read_to_string(path.join("Chart.yaml"))?)?;
            if let Some(version) = value.get("version").and_then(|v| v.as_str()) {
                target.version = version.into();
            }
            insert(
                &mut target,
                "install",
                &["helm", "dependency", "build", "."],
                false,
            );
            insert(&mut target, "build", &["helm", "package", "."], true);
            insert(&mut target, "lint", &["helm", "lint", "."], true);
            insert(
                &mut target,
                "test",
                &["helm", "template", "oyzu-check", "."],
                true,
            );
        }
        "docker/image" => {
            target.manager = "docker".into();
            insert(&mut target, "build", &["docker", "build", "."], true);
            unavailable(
                &mut target,
                "test",
                "No image smoke-test contract is configured",
            );
        }
        _ => bail!("unsupported builder {builder}"),
    }
    for name in ["lint", "format"] {
        unavailable(
            &mut target,
            name,
            "No native configuration or registered integration was discovered",
        );
    }
    Ok(target)
}

fn node(target: &mut Target) -> Result<()> {
    let value: Value =
        serde_json::from_str(&fs::read_to_string(target.path.join("package.json"))?)?;
    let locks: Vec<_> = [
        ("package-lock.json", "npm"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
    ]
    .into_iter()
    .filter(|(file, _)| target.path.join(file).is_file())
    .collect();
    if locks.len() > 1 {
        bail!("{}: conflicting Node lockfiles", target.name);
    }
    let declared = value
        .get("packageManager")
        .and_then(Value::as_str)
        .and_then(|v| v.split('@').next());
    if let (Some(requested), Some((_, locked))) = (declared, locks.first()) {
        if requested != *locked {
            bail!("{}: packageManager disagrees with lockfile", target.name);
        }
    }
    let manager = declared
        .or_else(|| locks.first().map(|(_, m)| *m))
        .unwrap_or("npm")
        .to_string();
    if !["npm", "pnpm", "yarn"].contains(&manager.as_str()) {
        bail!("unsupported Node manager {manager}");
    }
    target.manager = manager.clone();
    target.version = value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .into();
    let args: Vec<&str> = match manager.as_str() {
        "npm" if !locks.is_empty() => vec!["npm", "ci"],
        "npm" => vec!["npm", "install"],
        "pnpm" => vec!["pnpm", "install", "--frozen-lockfile"],
        _ => vec!["yarn", "install", "--frozen-lockfile"],
    };
    insert(target, "install", &args, false);
    if let Some(scripts) = value.get("scripts").and_then(Value::as_object) {
        for (name, command) in scripts {
            if !command.is_string() {
                bail!("Node script {name} is not a string");
            }
            let stage = matches!(
                name.as_str(),
                "build" | "test" | "lint" | "format:check" | "format-check"
            );
            insert(target, name, &[&manager, "run", name], stage);
            target.tasks.get_mut(name).unwrap().mutates_source = name == "format";
        }
    }
    Ok(())
}

fn python(target: &mut Target) -> Result<()> {
    let file = target.path.join("pyproject.toml");
    let value: toml::Value = if file.is_file() {
        toml::from_str(&fs::read_to_string(file)?)?
    } else {
        toml::Value::Table(Default::default())
    };
    let uv = target.path.join("uv.lock").is_file()
        || value.get("tool").and_then(|v| v.get("uv")).is_some();
    let poetry = target.path.join("poetry.lock").is_file()
        || value.get("tool").and_then(|v| v.get("poetry")).is_some();
    if uv && poetry {
        bail!("{}: conflicting Python managers", target.name);
    }
    target.manager = if uv {
        "uv"
    } else if poetry {
        "poetry"
    } else {
        "pip"
    }
    .into();
    if let Some(version) = value
        .get("project")
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
    {
        target.version = version.into();
    }
    if uv {
        insert(target, "install", &["uv", "sync", "--locked"], false);
        insert(target, "build", &["uv", "build"], true);
        insert(target, "test", &["uv", "run", "--locked", "pytest"], true);
    } else if poetry {
        insert(target, "install", &["poetry", "install"], false);
        insert(target, "build", &["poetry", "build"], true);
        insert(target, "test", &["poetry", "run", "pytest"], true);
    } else {
        if target.path.join("requirements.txt").is_file() {
            insert(
                target,
                "install",
                &["python", "-m", "pip", "install", "-r", "requirements.txt"],
                false,
            );
        }
        if target.path.join("pyproject.toml").is_file() || target.path.join("setup.py").is_file() {
            insert(target, "build", &["python", "-m", "build"], true);
        }
        insert(target, "test", &["python", "-m", "pytest"], true);
    }
    let has_tests = target.path.join("tests").is_dir() || target.path.join("test").is_dir();
    if !has_tests {
        if let Some(test) = target.tasks.get_mut("test") {
            test.availability = Some("No conventional test directory detected".into());
            test.build_stage = false;
        }
    }
    if value.get("tool").and_then(|v| v.get("ruff")).is_some() {
        insert(target, "lint", &["ruff", "check", "."], true);
        insert(
            target,
            "format-check",
            &["ruff", "format", "--check", "."],
            true,
        );
        insert(target, "format", &["ruff", "format", "."], false);
        target.tasks.get_mut("format").unwrap().mutates_source = true;
    }
    Ok(())
}

pub fn discover(path: &Path) -> Result<Workspace> {
    let root = path
        .canonicalize()
        .context("project directory does not exist")?;
    let mut targets = BTreeMap::new();
    if let Some(configs) = config::targets(&root)? {
        for (name, config) in configs {
            let dir = config::contained(&root, config.path.as_deref().unwrap_or(Path::new(".")))?;
            targets.insert(
                name.clone(),
                discover_target(&name, &dir, Some(&config.uses))?,
            );
        }
    } else {
        // Checkout directory spelling must not change logical task identities.
        let name = "project";
        targets.insert(name.into(), discover_target(name, &root, None)?);
    }
    let mut tasks = BTreeMap::new();
    for target in targets.values() {
        for task in target.tasks.values() {
            tasks.insert(task.id(), task.clone());
        }
    }
    let config = config::project(&root)?;
    for task in tasks.values_mut() {
        task.env.extend(config.env.clone());
    }
    for (id, definition) in config.tasks {
        let (group, name) = id.split_once(':').unwrap_or(("", &id));
        let existing = tasks.get(&id);
        let base = if group.is_empty() {
            &root
        } else {
            &targets
                .get(group)
                .with_context(|| format!("unknown task group {group}"))?
                .path
        };
        let cwd = config::contained(base, definition.cwd.as_deref().unwrap_or(Path::new(".")))?;
        if definition.run.is_some() == definition.argv.is_some() {
            bail!("task {id}: specify exactly one of run or argv");
        }
        let argv = if let Some(argv) = definition.argv {
            if argv.is_empty() {
                bail!("task {id}: empty argv");
            }
            argv
        } else {
            let script = definition.run.unwrap();
            let shell = definition.shell.unwrap_or_else(|| {
                if cfg!(windows) {
                    "powershell.exe".into()
                } else {
                    "sh".into()
                }
            });
            if shell.to_lowercase().contains("powershell") || shell.to_lowercase().contains("pwsh")
            {
                vec![
                    shell,
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    script,
                ]
            } else {
                vec![shell, "-c".into(), script]
            }
        };
        let mut env = config.env.clone();
        env.extend(definition.env);
        let stage = existing.is_some_and(|t| t.build_stage);
        tasks.insert(
            id.clone(),
            Task {
                name: name.into(),
                target: group.into(),
                provider: "oyzu.toml".into(),
                argv,
                cwd,
                env,
                depends_on: definition.depends_on,
                availability: None,
                build_stage: stage,
                mutates_source: false,
                stdout_must_be_empty: false,
            },
        );
    }
    Ok(Workspace {
        root,
        targets,
        tasks,
    })
}
