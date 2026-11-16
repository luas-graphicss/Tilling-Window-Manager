#!/usr/bin/env bash
set -euo pipefail

readonly PACKAGE_DIRECTORY=${1:?usage: smoke-test-package.sh <directory holding the .deb>}
readonly INSTANCE=smoke
readonly START_ATTEMPTS=100
readonly POLL_SECONDS=0.1
readonly EXIT_LIMIT_SECONDS=20

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install --yes --no-install-recommends "$PACKAGE_DIRECTORY"/tilling_*_amd64.deb

tilling --version
tillingctl --version

XDG_RUNTIME_DIR=$(mktemp --directory)
export XDG_RUNTIME_DIR
chmod 700 "$XDG_RUNTIME_DIR"

tilling --backend headless --instance "$INSTANCE" &
readonly COMPOSITOR=$!

attempts=0
until tillingctl --instance "$INSTANCE" get version; do
  attempts=$((attempts + 1))
  if ((attempts >= START_ATTEMPTS)); then
    echo "tilling did not answer its control socket" >&2
    exit 1
  fi
  sleep "$POLL_SECONDS"
done

tillingctl --instance "$INSTANCE" get outputs
tillingctl --instance "$INSTANCE" quit
timeout "$EXIT_LIMIT_SECONDS" tail --pid="$COMPOSITOR" --follow /dev/null
wait "$COMPOSITOR"

apt-get remove --yes tilling
if command -v tilling tillingctl; then
  echo "the programs are still installed after removing the package" >&2
  exit 1
fi
