# nix-unit tests for repository invariants (run: `nix develop -c nix-unit ./test.nix`).
{
  testCoreLibrarySourceExists = {
    expr = builtins.pathExists ./core/src/lib.rs;
    expected = true;
  };

  testCargoTomlDeclaresWorkspace = {
    expr = builtins.elem "core" (
      (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.members or [ ]
    );
    expected = true;
  };

  testCoreCrateNamedDagCore = {
    expr = (builtins.fromTOML (builtins.readFile ./core/Cargo.toml)).package.name == "dag-core";
    expected = true;
  };

  testReadmeReferencesDagCore = {
    expr = builtins.match ".*dag-core.*" (builtins.readFile ./README.md) != null;
    expected = true;
  };

  testFlakeExposesFormatter = {
    expr = builtins.match ".*treefmt.*" (builtins.readFile ./flake.nix) != null;
    expected = true;
  };
}
