#!/bin/sh
set -eu

root=$(pwd)
bun=${npm_execpath:-bun}
out="$root/target/template"
export CARGO_TARGET_DIR="$root/target/template-build"
layer_order="@layer bq.reset,bq.tokens,bq.base,bq.elements,bq.components,app;"

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
        first_layer=$(grep -o "@layer [^{;]*[{;]" public/build/site-*.css | head -n 1)
        if [ "$first_layer" != "$layer_order" ]; then
            echo "$name: the built CSS starts with \"$first_layer\" instead of boutique's layer order" >&2
            exit 1
        fi
    )
    echo "ok: $name"
}

check base
check database --database
check accounts --accounts
