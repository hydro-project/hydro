#!/usr/bin/env python3
"""Build Hydro trybuild binaries from a hydro-manifest.json.

This script is called by the bash build script after the export step
produces the manifest. It handles:
  1. Parsing the manifest
  2. Running `cargo brazil configure` and `cargo brazil fetch` per project
  3. Building each binary with `cargo build`
  4. Copying binaries into the publishable build/ tree
"""

import json
import shutil
import subprocess
import sys
from pathlib import Path


def run(cmd: list[str], **kwargs) -> None:
    """Run a command, printing it first. Raises on failure."""
    print(f"  $ {' '.join(cmd)}", flush=True)
    subprocess.run(cmd, check=True, **kwargs)


def main() -> None:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <hydro-manifest.json>", file=sys.stderr)
        sys.exit(1)

    manifest_path = Path(sys.argv[1])
    manifest = json.loads(manifest_path.read_text())
    assets_dir = manifest_path.parent

    # Collect build entries from all processes and clusters.
    builds = [
        entry["build"]
        for entry in list(manifest.get("processes", {}).values())
        + list(manifest.get("clusters", {}).values())
    ]

    if not builds:
        print("No binaries to build.")
        return

    # --- Configure + Fetch for each unique trybuild project ---
    seen_projects: set[str] = set()
    for build in builds:
        project_dir = build["project_dir"]
        if project_dir in seen_projects:
            continue
        seen_projects.add(project_dir)
        trybuild_manifest = f"{project_dir}/Cargo.toml"

        print(f"\nConfiguring trybuild project: {project_dir}")
        run([
            "cargo", "brazil", "configure",
            "--manifest-path", "./Cargo.toml",
            "--manifest-path", trybuild_manifest,
        ])

        print(f"Fetching dependencies for: {project_dir}")
        run(["cargo", "brazil", "fetch", "--manifest-path", trybuild_manifest])

    # --- Build each binary ---
    print(f"\nBuilding {len(builds)} trybuild binaries...")
    for build in builds:
        bin_name = build["bin_name"]
        project_dir = build["project_dir"]
        target_dir = build["target_dir"]
        features = ",".join(build["features"])

        print(f"\n  Building: {bin_name}")
        run(
            [
                "cargo", "build",
                "--locked",
                "--release",
                "-p", build["package_name"],
                "--example", bin_name,
                "--target-dir", target_dir,
                "--features", features,
                "--manifest-path", f"{project_dir}/Cargo.toml",
            ],
            env={
                **dict(__import__("os").environ),
                "STAGELEFT_TRYBUILD_BUILD_STAGED": "1",
            },
        )

    # --- Copy binaries into the publishable build/ tree ---
    bin_dir = assets_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)

    print(f"\nCopying binaries to {bin_dir}/")
    for build in builds:
        bin_name = build["bin_name"]
        src = Path(build["target_dir"]) / "release" / "examples" / bin_name
        dest = bin_dir / bin_name
        if src.exists():
            shutil.copy2(src, dest)
            dest.chmod(0o755)
            print(f"  {bin_name} -> {dest}")
        else:
            print(f"  WARNING: binary not found: {src}", file=sys.stderr)

    print(f"\nDone. Artifacts in: {assets_dir}")


if __name__ == "__main__":
    main()
