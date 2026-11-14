#!/usr/bin/env bash
# Clone or update the reference repos listed in refs/repos.txt.
#
#   refs/sync.sh            sync every uncommented entry
#   refs/sync.sh jax wing   sync only these
#
# Clones are shallow (depth 1, blobs fetched on demand) and live in refs/<name>/,
# which git ignores. Re-running moves each clone to the latest commit of its ref
# and discards any local edits there: these are read-only references.
set -euo pipefail
cd "$(dirname "$0")"

want=("$@")
fails=0
seen=()

sync_one() {
  local name=$1 url=$2 ref=$3
  shift 3
  local sparse=("$@") action=updated target=HEAD
  [[ "$ref" != "-" ]] && target=$ref

  if [[ ! -d "$name/.git" ]]; then
    action=cloned
    git clone --quiet --depth 1 --filter=blob:none --no-checkout "$url" "$name" || return 1
    if ((${#sparse[@]})); then
      git -C "$name" sparse-checkout set "${sparse[@]}" || return 1
    fi
  fi
  git -C "$name" fetch --quiet --depth 1 origin "$target" || return 1
  git -C "$name" reset --quiet --hard FETCH_HEAD || return 1
  printf '  %-13s %-8s %s  %s\n' "$name" "$action" \
    "$(git -C "$name" log -1 --format='%h %cs')" "$(du -sh "$name" | cut -f1)"
}

while read -r -u 3 name url ref rest; do
  [[ -z "${name:-}" || "$name" == \#* ]] && continue
  if ((${#want[@]})) && [[ " ${want[*]} " != *" $name "* ]]; then
    continue
  fi
  rest=${rest%%#*}  # drop trailing comments
  read -r -a sparse <<<"$rest"
  seen+=("$name")
  if ! sync_one "$name" "$url" "${ref:--}" ${sparse[@]+"${sparse[@]}"}; then
    echo "  $name: FAILED" >&2
    fails=$((fails + 1))
  fi
done 3<repos.txt

for w in ${want[@]+"${want[@]}"}; do
  [[ " ${seen[*]-} " == *" $w "* ]] || { echo "  $w: not an uncommented entry in repos.txt" >&2; fails=$((fails + 1)); }
done

echo "  total: $(du -sh . | cut -f1)"
exit $((fails > 0))
