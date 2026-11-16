#!/usr/bin/env bash
set -euo pipefail

readonly INCOMPLETE_EXIT_CODE=3
readonly BROKEN_EXIT_CODE=2

list_scenarios() {
  cargo xtask scenario --help |
    tr '\n' ' ' |
    sed --quiet 's/.*\[possible values: \([^]]*\)\].*/\1/p' |
    tr --delete ','
}

read -r -a scenarios <<<"$(list_scenarios)"
if ((${#scenarios[@]} == 0)); then
  echo "::error::xtask did not list its scenarios"
  exit "$BROKEN_EXIT_CODE"
fi

failed=()
for scenario in "${scenarios[@]}"; do
  status=0
  cargo xtask scenario "$scenario" || status=$?
  if ((status != 0 && status != INCOMPLETE_EXIT_CODE)); then
    failed+=("$scenario")
  fi
done

if ((${#failed[@]} > 0)); then
  echo "::error::scenarios that failed: ${failed[*]}"
  exit 1
fi
