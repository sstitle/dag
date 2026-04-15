{ pkgs, ... }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # keep-sorted start
    cargo
    git
    mask
    maturin
    nodejs_22
    python3
    rustc
    uv
    # keep-sorted end
  ];

  shellHook = ''
    echo "🚀 Development environment loaded!"
    echo ""
    echo "Available tools:"
    echo "  cargo / rustc  Rust toolchain"
    echo "  maturin        Python extension builder"
    echo "  node / npm     Node.js toolchain"
    echo "  uv             Python package manager"
    echo "  mask           Task runner"
    echo ""
    echo "Run 'mask --help' to see available tasks."
    echo "Run 'nix fmt' to format all files."
  '';
}
