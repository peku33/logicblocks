#!/bin/bash

set -e

if [ -z "$1" ]
then
    echo "Usage: $0 example_name"
    exit 1
fi
EXAMPLE_NAME="$1"

CI_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR=`realpath $CI_DIR/..`

if [ ! -f "$ROOT_DIR/Controller/examples/test_$EXAMPLE_NAME.rs" ]
then
    echo "example $EXAMPLE_NAME does not exist"
    exit 1
fi

WORKSPACE_DIR=$CI_DIR/Workspace
mkdir -p $WORKSPACE_DIR
cd $WORKSPACE_DIR

function web_static_pack_gui_ensure {
    echo "web_static_pack_gui: starting"
    cd $ROOT_DIR/Gui
    
    HASH_CURRENT="HASH_CURRENT:NULL"
    if [ -d ./build ]
    then
        HASH_CURRENT=$(find . -not -path './node_modules/*' -not -path './build/*' -type f | sort | xargs cat | sha256sum | awk '{ print $1 }')
    fi

    cd $WORKSPACE_DIR
    HASH_CACHED="HASH_CACHED:NULL"
    if [ -f ./web_static_pack_gui.bin -a -f ./web_static_pack_gui.bin.sha256 ]
    then
        HASH_CACHED=$(cat ./web_static_pack_gui.bin.sha256)
    fi

    if [ "$HASH_CURRENT" != "$HASH_CACHED" ]
    then
        echo "web_static_pack_gui: hash mismatch ($HASH_CURRENT) ($HASH_CACHED), rebuilding"
        
        cd $ROOT_DIR/Gui
        export NPM_CONFIG_PROGRESS=false
        export NPM_CONFIG_SPIN=false
        npm install # using `ci` reinstalls everything, which is really slow
        npm run-script build

        echo "web_static_pack_gui: packing"
        cd $WORKSPACE_DIR
        web-static-pack-packer $ROOT_DIR/Gui/build ./web_static_pack_gui.bin
        echo $HASH_CURRENT > ./web_static_pack_gui.bin.sha256
    else
        echo "web_static_pack_gui: matches, skipping"
    fi

    echo "web_static_pack_gui: completed"
}
web_static_pack_gui_ensure

function controller_build {
    echo "controller_build: starting"
    
    cd $ROOT_DIR/Controller
    export CI_WEB_STATIC_PACK_GUI=$WORKSPACE_DIR/web_static_pack_gui.bin
    cargo build --release --locked --features ci --example test_$EXAMPLE_NAME
    
    echo "controller_build: completed"
}
controller_build

echo "all: completed"
