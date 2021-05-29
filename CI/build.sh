#!/bin/bash

set -e

CI_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT_DIR=`realpath $CI_DIR/..`

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
CI_WEB_STATIC_PACK_GUI=$WEB_STATIC_PACK_GUI cargo build --release --locked --features ci --examples
echo "Building Controller <"

echo "All done!"
