#!/bin/sh
# Copy what the port needs from the Mystery House decompilation, pinned to one of its tags, and
# record where it came from and each file's SHA-256 in decomp/MANIFEST.
#
# usage: tools/sync-decomp.sh [<decomp checkout> [<tag>]]     copy (default ../mystery-house-decomp, v1.0)
#        tools/sync-decomp.sh --verify                        check decomp/ against decomp/MANIFEST
#
# The files are taken from the tag with git show, so they are exactly what the tag holds,
# whatever is checked out in the decomp. Never edit decomp/ by hand: change the decomp, tag it,
# and sync again.
set -eu
HERE=$(cd "$(dirname "$0")/.." && pwd)
OUT="$HERE/decomp"

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
	else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

if [ "${1:-}" = "--verify" ]; then
	status=0
	grep -v '^#' "$OUT/MANIFEST" | while read -r hash path; do
		if [ "$(sha256 "$OUT/$path")" != "$hash" ]; then
			echo "decomp/$path: differs from MANIFEST" >&2
			exit 1
		fi
	done || status=1
	[ $status = 0 ] && echo "decomp/: all $(grep -vc '^#' "$OUT/MANIFEST") files match MANIFEST"
	exit $status
fi

DECOMP=${1:-"$HERE/../mystery-house-decomp"}
TAG=${2:-v1.0}
COMMIT=$(git -C "$DECOMP" rev-parse "$TAG^{commit}")

# what a port uses: the data, the spec, and the conformance scripts with their expected
# transcripts and pixel hashes (the emulator-only files, *.screens.txt and coverage, stay behind)
FILES="assets/text/gamedata.json PORTING.md LICENSE"
for f in $(git -C "$DECOMP" ls-tree -r --name-only "$TAG" -- tests/play); do
	case "$f" in
	*.screens.txt|*/coverage.expected.txt) ;;
	*) FILES="$FILES $f" ;;
	esac
done

rm -rf "$OUT"
mkdir -p "$OUT"
{
	echo "# Copied from the Mystery House decompilation by tools/sync-decomp.sh; do not edit."
	echo "# https://github.com/kDylanHayes/mystery-house-decompilation"
	echo "# tag $TAG, commit $COMMIT"
	echo "# sha256 path"
} > "$OUT/MANIFEST"
for f in $FILES; do
	case "$f" in
	assets/text/*) dest=$(basename "$f") ;;
	tests/play/*) dest="play/${f#tests/play/}" ;;
	*) dest="$f" ;;
	esac
	mkdir -p "$(dirname "$OUT/$dest")"
	git -C "$DECOMP" show "$TAG:$f" > "$OUT/$dest"
	echo "$(sha256 "$OUT/$dest") $dest" >> "$OUT/MANIFEST"
done
echo "decomp/: $(grep -vc '^#' "$OUT/MANIFEST") files from $TAG ($COMMIT)"
