#!/bin/bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./scripts/import-fajita-training.sh ARCHIVE.tar.gz

Verify and import an archive made by export-fajita-training.sh.
An existing training/fajita-main is moved to training/backups/ first.
Stop Fajita training before running this script.
EOF
}

fail() {
    printf 'import-fajita-training: %s\n' "$*" >&2
    exit 1
}

if [[ $# -ne 1 ]]; then
    usage >&2
    exit 2
fi
if [[ $1 == "--help" || $1 == "-h" ]]; then
    usage
    exit 0
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RUST_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RUN_DIR=${FAJITA_RUN_DIR:-"$RUST_ROOT/training/fajita-main"}
if [[ $RUN_DIR != /* ]]; then
    RUN_DIR="$PWD/$RUN_DIR"
fi
TRAINING_DIR=$(dirname -- "$RUN_DIR")

archive=$1
[[ -f $archive ]] || fail "archive not found: $archive"
archive_parent=$(CDPATH= cd -- "$(dirname -- "$archive")" && pwd)
archive="$archive_parent/$(basename -- "$archive")"

if command -v pgrep >/dev/null 2>&1 &&
    pgrep -f '(^|[ /])fajita-train([ /]|$).*train([ /]|$)' >/dev/null 2>&1; then
    fail "Fajita training appears to be running; stop it before importing"
fi

mkdir -p "$TRAINING_DIR"
work_dir=$(mktemp -d "$TRAINING_DIR/.fajita-import.XXXXXX")
backup_path=
cleanup() {
    status=$?
    if [[ $status -ne 0 && -n $backup_path && ! -e $RUN_DIR && -d $backup_path ]]; then
        printf 'Import failed; restoring the previous run.\n' >&2
        mv "$backup_path" "$RUN_DIR" || true
    fi
    rm -rf -- "$work_dir"
}
trap cleanup EXIT INT TERM

tar -tzf "$archive" >"$work_dir/archive-contents.txt" ||
    fail "archive is not a readable gzip-compressed tar file"

while IFS= read -r entry; do
    case "$entry" in
        /* | ../* | */../* | */..) fail "archive contains an unsafe path: $entry" ;;
        MANIFEST.txt | SHA256SUMS | fajita-main | fajita-main/ | fajita-main/*) ;;
        *) fail "archive contains an unexpected entry: $entry" ;;
    esac
done <"$work_dir/archive-contents.txt"

tar -C "$work_dir" -xzf "$archive"

unexpected=$(find "$work_dir/fajita-main" ! -type f ! -type d -print 2>/dev/null || true)
[[ -z $unexpected ]] || fail "archive contains an unsupported file type: $unexpected"
[[ -f "$work_dir/MANIFEST.txt" ]] || fail "archive is missing MANIFEST.txt"
[[ -f "$work_dir/SHA256SUMS" ]] || fail "archive is missing SHA256SUMS"
grep -qx 'format=fajita-training-export-v1' "$work_dir/MANIFEST.txt" ||
    fail "archive uses an unsupported export format"

(
    cd "$work_dir"
    shasum -a 256 -c SHA256SUMS >/dev/null
) || fail "archive checksum verification failed"

required_files=(
    state.txt
    latest.bin
    champion.bin
    optimizer.bin
    replay.bin
)
for file in "${required_files[@]}"; do
    [[ -s "$work_dir/fajita-main/$file" ]] ||
        fail "archive is missing required run file: $file"
done
grep -qx 'purity=rules-only-fresh-seed' "$work_dir/fajita-main/state.txt" ||
    fail "archive is missing Fajita's rules-only fresh-seed marker"

archive_commit=$(sed -n 's/^source_commit=//p' "$work_dir/MANIFEST.txt")
current_commit=$(git -C "$RUST_ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')
if [[ $archive_commit != "$current_commit" ]]; then
    printf 'Note: archive commit is %s; current commit is %s.\n' \
        "$archive_commit" "$current_commit"
fi

printf 'Checksums passed. Validating the imported run with fajita-train...\n'
(
    cd "$RUST_ROOT"
    cargo run --release -p fajita-train -- status \
        --run-dir "$work_dir/fajita-main"
)

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
if [[ -e $RUN_DIR ]]; then
    [[ -d $RUN_DIR ]] || fail "import target exists but is not a directory: $RUN_DIR"
    backup_dir="$TRAINING_DIR/backups"
    mkdir -p "$backup_dir"
    backup_path="$backup_dir/fajita-main-before-import-$timestamp"
    [[ ! -e $backup_path ]] || fail "backup path already exists: $backup_path"
    mv "$RUN_DIR" "$backup_path"
fi

mv "$work_dir/fajita-main" "$RUN_DIR"

printf 'Fajita training run imported to:\n%s\n' "$RUN_DIR"
if [[ -n $backup_path ]]; then
    printf 'Previous run preserved at:\n%s\n' "$backup_path"
fi
