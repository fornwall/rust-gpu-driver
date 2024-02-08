#!/bin/sh
set -e -u

bumpversion minor
git push
git push --tags
