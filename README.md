# COMP3931

---

## Running

```sh
# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# clone project
git clone https://github.com/quaoz/comp3931.git
cd comp3931

# build
cargo build
# or run
cargo run
```

## Nix

```sh
# run without installing
nix run github:quaoz/comp3931
# run checks
nix flake check
# format project
nix fmt
```
