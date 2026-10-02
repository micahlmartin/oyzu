# Isolated mise embedding experiment

See the [recorded hypothesis and acceptance criteria](../../docs/proposals/OEP-0003-mise-integration/experiment.md).
This harness is an alternate experimental frontend linked to mise's Rust
library. It is not the production Oyzu CLI and does not implement build scenarios.

Upstream source is fetched into an explicitly selected external directory.
Its MIT license and third-party notices stay with that checkout. No upstream
source or executable is vendored into the product. Rust 1.95 or newer and native
build prerequisites and Python 3.11+ are required; the production crate's compiler
is unchanged.

Prepare an unmodified API probe, then apply the recorded integration patch:

```sh
python tooling/mise-experiment/prepare.py --source /absolute/external/mise
# In that checkout, build only the example, never the mise binary:
cargo check --locked --example oyzu-api-probe --no-default-features --features rustls,vfox/vendored-lua
python tooling/mise-experiment/prepare.py --source /absolute/external/mise --patch
cargo build --locked --example oyzu-mise-spike --no-default-features --features rustls,vfox/vendored-lua
```

The unmodified probe calls the actual configuration constructor needed to bypass
mise discovery. At the pin it fails with E0624 (private associated function).
The patched harness supplies a narrow empty-config entrypoint, prevents shim
lookup of an ambient mise executable, guards Node HTTP requests/redirects, and
exposes verbatim PATH joining for live-shell updates.
The guard is experimental process-local routing, not an OS sandbox or production
connector. The patch includes upstream context covered by [its preserved MIT
notice](UPSTREAM-LICENSE.txt). The [dependency declaration inventory](dependency-licenses.json)
includes workspace development dependencies and is not a complete legal audit.

Provision real archive fixtures outside the test's network-isolated execution:

```sh
python tooling/mise-experiment/run.py --work /absolute/fixtures --provision
python tooling/mise-experiment/run.py --work /absolute/fixtures --binary /absolute/external/mise/target/debug/examples/oyzu-mise-spike
```

Run the latter inside a container with `--network none` and pass
`--network-isolated` to also exercise an independent curl subprocess's denied
egress. This flag does not itself create isolation. The synthetic repository
runs on loopback within the container; it serves real, pre-provisioned Node
archives. It does not simulate a successful tool executable or contact a private
platform. Archive provisioning checks official HTTPS SHASUMS256 metadata; the
fixture's SHA-256 evidence is not a publisher-signature claim.

Each invocation creates a new run directory and writes machine-readable results
and the observed synthetic repository request paths. Shell tests execute the
upstream-generated activation script in two concurrent shells, with the same
experimental executable as the hook target. No profile files are modified.

The Linux image is pinned in `Dockerfile`. Mount the external upstream checkout
at `/upstream`, a native Docker volume at `/upstream/target`, and use a native
volume for `/work` when running the harness; extracting Node into a Windows bind
mount distorts timings. Build with `bash -c` so the image's Cargo PATH is retained.
The recorded build uses Rust 1.95, four jobs, the dev profile with debug symbols
and incremental compilation disabled, and the feature flags above.

Native Windows was verified with Rust `1.95.0-x86_64-pc-windows-gnu`, WinLibs GCC
16.2.0 / MinGW-w64 14.0.0 MSVCRT, CMake and Ninja. Set `CC=gcc`, `CXX=g++`,
`CMAKE_GENERATOR=Ninja` and `CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER` to that
toolchain's `gcc.exe`; put its `bin` directory on PATH for build and execution.
That run uses upstream's normal dev profile (debug level 1 and incremental
compilation), so its timings are not directly comparable with the Linux build.
Mixed LLVM/GCC attempts failed in the build script before this consistent
toolchain succeeded. MSVC remains unverified. The expanded qualification record
also contains a native macOS ARM64 run.

After a successful Linux run, process/filesystem/network traces can be collected
in the same network-isolated container using its printed run directory:

```sh
python3 /harness/trace.py --binary /build/debug/examples/oyzu-mise-spike --run-root /work/run-REPLACE
```

The image includes `strace`. Results record the binary and harness hashes, actual
host, case outcomes, timings and fixture requests. Full trace logs remain under
that run's `traces/`; their hashes and counts are in `summary.json`. Recorded
results under `results/` are evidence from this experiment, not a portable
performance target or a claim that the production CLI provides these commands.

## Real broker and enforced worker qualification

The expanded scope and current evidence are tracked in
[the qualification record](../../docs/proposals/OEP-0003-mise-integration/qualification.md).
`broker-host` links the actual production Oyzu crate and calls its broker and
executor APIs. It has its own lockfile and never receives source credentials
from a project file. The driver configuration is generated under a host-private
fixture directory and is not mounted into the worker.

Build the driver with the same consistent native compiler setup described above:

```sh
cargo +1.95.0 build --locked --manifest-path tooling/mise-experiment/broker-host/Cargo.toml
python tooling/mise-experiment/prepare_broker_image.py --binary /absolute/linux/oyzu-mise-spike --context /absolute/external/image-context
python tooling/mise-experiment/provision_backends.py --work /absolute/fixtures
python tooling/mise-experiment/run_broker.py --driver /absolute/broker-host-binary --work /absolute/fixtures
```

The image requires the previously built `oyzu-mise-experiment:trace` base and a
Linux frontend binary, even when the host is Windows. Provision the real Linux
x64 Node archives/inventory under the fixtures directory before this run. The
current positive control uses Docker Desktop's `host.docker.internal`; the
recorded host is Windows and the enforced worker is Linux x64. No native Windows
sandbox is implied. Source credentials are generated test canaries, never real
accounts or credentials, and only credential-presence booleans enter the report.
