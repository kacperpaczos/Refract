#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import shutil
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


BASE_URL = "https://extensions.gnome.org"


@dataclass
class ExtensionReport:
    uuid: str
    name: str | None
    target_shell_version: str
    online_compatible: bool
    metadata_compatible: bool | None
    newest_known_version: int | None
    newest_known_version_tag: int | None
    target_version: int | None
    target_version_tag: int | None
    download_url: str | None
    extracted_dir: str | None
    zip_path: str | None
    metadata_uuid: str | None
    metadata_shell_versions: list[str]
    status: str
    error: str | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Download GNOME Shell extensions into a work directory and inspect "
            "their online metadata plus local metadata.json compatibility."
        )
    )
    parser.add_argument(
        "uuids",
        nargs="*",
        help="Extension UUIDs, e.g. dash-to-dock@micxgx.gmail.com",
    )
    parser.add_argument(
        "--uuids-file",
        type=Path,
        help="Text file with one UUID per line",
    )
    parser.add_argument(
        "--shell-version",
        required=True,
        help="Target GNOME Shell version, e.g. 46, 47, 48, 49, 50",
    )
    parser.add_argument(
        "--work-dir",
        type=Path,
        default=Path("tmp/extensions-check"),
        help="Directory where zip files and extracted extensions will be stored",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print the final report as JSON instead of human-readable text",
    )
    parser.add_argument(
        "--keep-existing",
        action="store_true",
        help="Do not delete existing extracted directories before unpacking",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    uuids = collect_uuids(args.uuids, args.uuids_file)
    if not uuids:
        print("No extension UUIDs provided.", file=sys.stderr)
        return 2

    work_dir = args.work_dir.expanduser().resolve()
    downloads_dir = work_dir / "downloads"
    extracted_dir = work_dir / "extracted"
    downloads_dir.mkdir(parents=True, exist_ok=True)
    extracted_dir.mkdir(parents=True, exist_ok=True)

    reports: list[ExtensionReport] = []
    for uuid in uuids:
        reports.append(
            inspect_extension(
                uuid=uuid,
                target_shell_version=args.shell_version,
                downloads_dir=downloads_dir,
                extracted_root=extracted_dir,
                keep_existing=args.keep_existing,
            )
        )

    if args.json:
        print(json.dumps([asdict(report) for report in reports], indent=2, ensure_ascii=False))
    else:
        print_text_report(reports)

    return 0 if all(report.error is None for report in reports) else 1


def collect_uuids(positional: list[str], uuids_file: Path | None) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []

    def add(raw_uuid: str) -> None:
        cleaned = raw_uuid.strip()
        if not cleaned or cleaned.startswith("#"):
            return
        if cleaned not in seen:
            seen.add(cleaned)
            ordered.append(cleaned)

    for uuid in positional:
        add(uuid)

    if uuids_file:
        for line in uuids_file.read_text(encoding="utf-8").splitlines():
            add(line)

    return ordered


def inspect_extension(
    uuid: str,
    target_shell_version: str,
    downloads_dir: Path,
    extracted_root: Path,
    keep_existing: bool,
) -> ExtensionReport:
    try:
        info = fetch_extension_info(uuid, target_shell_version)
        shell_version_map = info.get("shell_version_map") or {}
        target_entry = shell_version_map.get(target_shell_version)
        newest_version, newest_version_tag = newest_known_release(shell_version_map)
        name = as_optional_str(info.get("name"))
        download_url = absolute_download_url(info.get("download_url"))

        zip_path: Path | None = None
        target_extract_dir: Path | None = None
        metadata: dict[str, Any] | None = None

        if download_url:
            zip_path = downloads_dir / build_zip_name(uuid, target_shell_version)
            target_extract_dir = extracted_root / uuid
            download_file(download_url, zip_path)
            if target_extract_dir.exists() and not keep_existing:
                shutil.rmtree(target_extract_dir)
            target_extract_dir.mkdir(parents=True, exist_ok=True)
            extract_zip(zip_path, target_extract_dir)
            metadata = read_metadata_json(target_extract_dir)

        metadata_shell_versions = normalize_shell_versions(metadata.get("shell-version")) if metadata else []
        metadata_compatible = (
            target_shell_version in metadata_shell_versions if metadata_shell_versions else None
        )

        return ExtensionReport(
            uuid=uuid,
            name=name,
            target_shell_version=target_shell_version,
            online_compatible=target_entry is not None,
            metadata_compatible=metadata_compatible,
            newest_known_version=newest_version,
            newest_known_version_tag=newest_version_tag,
            target_version=as_optional_int(target_entry.get("version")) if target_entry else None,
            target_version_tag=as_optional_int(target_entry.get("pk")) if target_entry else None,
            download_url=download_url,
            extracted_dir=str(target_extract_dir) if target_extract_dir else None,
            zip_path=str(zip_path) if zip_path else None,
            metadata_uuid=as_optional_str(metadata.get("uuid")) if metadata else None,
            metadata_shell_versions=metadata_shell_versions,
            status=build_status(target_entry is not None, metadata_compatible, download_url),
            error=None,
        )
    except Exception as exc:  # noqa: BLE001
        return ExtensionReport(
            uuid=uuid,
            name=None,
            target_shell_version=target_shell_version,
            online_compatible=False,
            metadata_compatible=None,
            newest_known_version=None,
            newest_known_version_tag=None,
            target_version=None,
            target_version_tag=None,
            download_url=None,
            extracted_dir=None,
            zip_path=None,
            metadata_uuid=None,
            metadata_shell_versions=[],
            status="error",
            error=str(exc),
        )


def fetch_extension_info(uuid: str, shell_version: str) -> dict[str, Any]:
    params = urllib.parse.urlencode({"uuid": uuid, "shell_version": shell_version})
    url = f"{BASE_URL}/extension-info/?{params}"
    return fetch_json(url)


def fetch_json(url: str) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "desktop-experience-switcher/check-gnome-extensions",
            "Accept": "application/json",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        charset = response.headers.get_content_charset() or "utf-8"
        payload = response.read().decode(charset)
    return json.loads(payload)


def absolute_download_url(download_url: str | None) -> str | None:
    if not download_url:
        return None
    if download_url.startswith("http://") or download_url.startswith("https://"):
        return download_url
    return urllib.parse.urljoin(BASE_URL, download_url)


def build_zip_name(uuid: str, shell_version: str) -> str:
    safe_uuid = "".join(char if char.isalnum() or char in ".-_@" else "_" for char in uuid)
    return f"{safe_uuid}.shell-{shell_version}.zip"


def download_file(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "desktop-experience-switcher/check-gnome-extensions"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        with destination.open("wb") as output:
            shutil.copyfileobj(response, output)


def extract_zip(zip_path: Path, destination: Path) -> None:
    with zipfile.ZipFile(zip_path) as archive:
        archive.extractall(destination)


def read_metadata_json(extracted_dir: Path) -> dict[str, Any]:
    metadata_path = extracted_dir / "metadata.json"
    if not metadata_path.exists():
        nested = sorted(extracted_dir.glob("*/metadata.json"))
        if len(nested) == 1:
            metadata_path = nested[0]
        else:
            raise FileNotFoundError(f"metadata.json not found in {extracted_dir}")
    return json.loads(metadata_path.read_text(encoding="utf-8"))


def normalize_shell_versions(value: Any) -> list[str]:
    if isinstance(value, list):
        return [str(item) for item in value]
    return []


def newest_known_release(shell_version_map: dict[str, Any]) -> tuple[int | None, int | None]:
    newest_version: int | None = None
    newest_version_tag: int | None = None

    for entry in shell_version_map.values():
        if not isinstance(entry, dict):
            continue
        version = as_optional_int(entry.get("version"))
        version_tag = as_optional_int(entry.get("pk"))
        if version is None:
            continue
        if newest_version is None or version > newest_version:
            newest_version = version
            newest_version_tag = version_tag

    return newest_version, newest_version_tag


def as_optional_str(value: Any) -> str | None:
    if value is None:
        return None
    return str(value)


def as_optional_int(value: Any) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def build_status(
    online_compatible: bool,
    metadata_compatible: bool | None,
    download_url: str | None,
) -> str:
    if not download_url:
        return "metadata-only"
    if online_compatible and metadata_compatible is True:
        return "ok"
    if online_compatible and metadata_compatible is None:
        return "ok-no-local-shell-version"
    if online_compatible and metadata_compatible is False:
        return "mismatch-online-vs-local"
    return "not-compatible"


def print_text_report(reports: list[ExtensionReport]) -> None:
    for report in reports:
        print(f"UUID: {report.uuid}")
        print(f"Name: {report.name or '(unknown)'}")
        print(f"Target GNOME: {report.target_shell_version}")
        print(f"Status: {report.status}")
        print(f"Online compatible: {'yes' if report.online_compatible else 'no'}")
        print(
            "Metadata compatible: "
            + (
                "yes"
                if report.metadata_compatible is True
                else "no"
                if report.metadata_compatible is False
                else "unknown"
            )
        )
        print(
            "Newest known version: "
            + format_release(report.newest_known_version, report.newest_known_version_tag)
        )
        print(
            "Target version: "
            + format_release(report.target_version, report.target_version_tag)
        )
        print(f"Download URL: {report.download_url or '(none)'}")
        print(f"ZIP path: {report.zip_path or '(not downloaded)'}")
        print(f"Extracted dir: {report.extracted_dir or '(not extracted)'}")
        print(f"Metadata UUID: {report.metadata_uuid or '(none)'}")
        print(
            "Metadata shell versions: "
            + (", ".join(report.metadata_shell_versions) if report.metadata_shell_versions else "(none)")
        )
        if report.error:
            print(f"Error: {report.error}")
        print("-" * 72)


def format_release(version: int | None, version_tag: int | None) -> str:
    if version is None and version_tag is None:
        return "(unknown)"
    if version is None:
        return f"version_tag={version_tag}"
    if version_tag is None:
        return f"v{version}"
    return f"v{version} (version_tag={version_tag})"


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except urllib.error.HTTPError as exc:
        print(f"HTTP error: {exc.code} {exc.reason}", file=sys.stderr)
        raise SystemExit(1)
    except urllib.error.URLError as exc:
        print(f"Network error: {exc.reason}", file=sys.stderr)
        raise SystemExit(1)
