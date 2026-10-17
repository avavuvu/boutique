#!/bin/sh
set -eu

root=$(pwd)
bun=${npm_execpath:-bun}
out="$root/target/template"
export CARGO_TARGET_DIR="$root/target/template-build"

cargo build -q -p cargo-bq
rm -rf "$out"
mkdir -p "$out"

check() {
    name=$1
    shift
    "$CARGO_TARGET_DIR/debug/cargo-bq" bq new "$out/$name" --yes --boutique-path "$root" "$@" > /dev/null
    (
        cd "$out/$name"
        cargo check -q
        "$bun" install --silent
        "$bun" run --silent typecheck
        "$bun" run --silent lint
        "$bun" run --silent build > /dev/null
    )
    echo "ok: $name"
}

check base
check database --database
check accounts --accounts
