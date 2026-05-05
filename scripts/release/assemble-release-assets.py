#!/usr/bin/env python3

import json
import pathlib
import shutil
import sys


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: scripts/release/assemble-release-assets.py <version> <input_dir> <output_dir>",
            file=sys.stderr,
        )
        return 1

    version = sys.argv[1]
    input_dir = pathlib.Path(sys.argv[2])
    output_dir = pathlib.Path(sys.argv[3])

    if not input_dir.is_dir():
        print(f"input directory not found: {input_dir}", file=sys.stderr)
        return 1

    output_dir.mkdir(parents=True, exist_ok=True)

    manifests = sorted(input_dir.rglob("release-manifest.json"))
    if not manifests:
        print(f"no release fragments found in {input_dir}", file=sys.stderr)
        return 1

    checksum_lines: list[str] = []
    merged_artifacts: list[dict] = []
    demo_fixture = None
    benchmark_report_markdown = None
    benchmark_report_json = None

    for manifest_path in manifests:
        data = json.loads(manifest_path.read_text())
        demo_fixture = demo_fixture or data.get("demo_fixture")
        benchmark_report_markdown = benchmark_report_markdown or data.get(
            "benchmark_report_markdown"
        )
        benchmark_report_json = benchmark_report_json or data.get("benchmark_report_json")

        sums_path = manifest_path.parent / "SHA256SUMS"
        if not sums_path.is_file():
            print(f"missing checksum file next to {manifest_path}", file=sys.stderr)
            return 1

        checksum_lines.extend(
            line.strip() for line in sums_path.read_text().splitlines() if line.strip()
        )

        for item in data.get("artifacts", []):
            archive_name = item["archive"]
            source_archive = manifest_path.parent / archive_name
            if not source_archive.is_file():
                print(f"missing archive {source_archive}", file=sys.stderr)
                return 1
            shutil.copy2(source_archive, output_dir / archive_name)
            merged_artifacts.append(item)

    unique_checksums = sorted(dict.fromkeys(checksum_lines))
    (output_dir / "SHA256SUMS").write_text("\n".join(unique_checksums) + "\n")

    merged_manifest = {
        "version": version,
        "host_target": "multi",
        "artifacts": merged_artifacts,
        "demo_fixture": demo_fixture,
        "benchmark_report_markdown": benchmark_report_markdown,
        "benchmark_report_json": benchmark_report_json,
    }
    (output_dir / "release-manifest.json").write_text(
        json.dumps(merged_manifest, indent=2) + "\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
