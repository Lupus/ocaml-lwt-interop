#!/bin/bash
set -euo pipefail

#################################################################################################
##########                                 RUST                                         #########
#################################################################################################

# Download all dependencies including patches defined in Cargo.toml
echo "(*) Fetching cargo deps..."
cargo fetch

###################################################################################################
##########                                  OPAM                                          #########
###################################################################################################

# Install project dependencies
echo "(*) Installing project dependencies from opam..."
opam install  . --deps-only --with-test --assume-depexts --yes
