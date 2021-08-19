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

# Build WEB
echo "Building GUI >"
cd $ROOT_DIR/Gui
export NPM_CONFIG_PROGRESS=false
export NPM_CONFIG_SPIN=false
npm ci
npm run-script build
echo "Building GUI <"

# Create GUI PACK
echo "Creating GUI Pack >"
WEB_STATIC_PACK_GUI=$WORKSPACE_DIR/web_static_pack_gui.bin
web-static-pack-packer $ROOT_DIR/Gui/build $WEB_STATIC_PACK_GUI
echo "Creating GUI Pack <"

# Build Controller
echo "Building Controller >"
cd $ROOT_DIR/Controller
CI_WEB_STATIC_PACK_GUI=$WEB_STATIC_PACK_GUI cargo build --release --locked --features ci --example test_$EXAMPLE_NAME
echo "Building Controller <"

echo "All done!"
