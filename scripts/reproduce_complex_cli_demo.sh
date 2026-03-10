#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_dir="$repo_root/tests/fixtures/repro"
workdir="$(mktemp -d)"

cleanup() {
  rm -rf "$workdir"
}
trap cleanup EXIT

cp -R "$fixture_dir/src" "$workdir/src"

echo "== Fixture: ComplexDeployment.sol =="
sed -n '1,220p' "$workdir/src/nested/ComplexDeployment.sol"

echo
echo "== Fixture: CommentsAndStrings.sol =="
sed -n '1,220p' "$workdir/src/CommentsAndStrings.sol"

echo
echo "== Contract List =="
forge-scriptgen --parser string-walker --list --contracts-dir "$workdir/src"

echo
echo "== Generate ComplexDeployment =="
(
  cd "$workdir"
  forge-scriptgen \
    --parser string-walker \
    --args '[{"raw":"Config({owner: msg.sender, limits: [1, 2, 3]})"},{"raw":"callback"},"primary",{"raw":"hex\"1234\""}]' \
    --private-key 0xabc123 \
    ComplexDeployment
)

echo
echo "== Generated ComplexDeployment.s.sol =="
sed -n '1,220p' "$workdir/script/ComplexDeployment.s.sol"

echo
echo "== Generate CommentsAndStrings =="
(
  cd "$workdir"
  forge-scriptgen \
    --parser string-walker \
    --args '["0xDeAdBeEf","hello"]' \
    --private-key 0xabc123 \
    CommentsAndStrings
)

echo
echo "== Generated CommentsAndStrings.s.sol =="
sed -n '1,220p' "$workdir/script/CommentsAndStrings.s.sol"

echo
echo "== Compare With Expected Fixtures =="
diff -u "$fixture_dir/expected/ComplexDeployment.s.sol" "$workdir/script/ComplexDeployment.s.sol"
diff -u "$fixture_dir/expected/CommentsAndStrings.s.sol" "$workdir/script/CommentsAndStrings.s.sol"
echo "All generated scripts match expected output."
