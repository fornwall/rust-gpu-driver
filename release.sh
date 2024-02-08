#!/bin/sh
set -e -u

bumpversion minor
cargo check
git push
git push --tags
