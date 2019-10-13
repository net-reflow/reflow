#!/bin/bash
set -evx
git config --local user.name "travisci"
git config --local user.email "<>"
export TRAVIS_TAG="build-$TRAVIS_BUILD_NUMBER"
git tag $TRAVIS_TAG || true

# Output binary name
name="reflow-$(date +'%Y%m%d')"

# Everything in this directory will be offered as download for the release
mkdir -p "./target/deploy"

function windows {
    mv "target/release/reflow.exe" "target/deploy/${name}-windows-x86_64.exe"
}

function osx {
    mv "./target/release/reflow" "./target/deploy/${name}-osx-$(arch)"
}

function debian {
    mv "./target/release/reflow" "./target/deploy/${name}-linux-$(arch)"
}

if [ "$TRAVIS_OS_NAME" == "osx" ]; then
    osx || exit
elif [ "$TRAVIS_OS_NAME" == "linux" ]; then
    debian || exit
elif [ "$TRAVIS_OS_NAME" == "windows" ]; then
    windows
fi
