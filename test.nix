# nix-unit tests for repository invariants (run: `nix develop -c nix-unit ./test.nix`).
{
  testCargoTomlDeclaresWorkspace = {
    expr = (builtins.match ".*\"core\".*" (builtins.readFile ./Cargo.toml)) != null;
    expected = true;
  };

  testCoreCrateNamedDagCore = {
    expr = (builtins.match ".*name = \"dag-core\".*" (builtins.readFile ./core/Cargo.toml)) != null;
    expected = true;
  };

  testReadmeReferencesDagCore = {
    expr = (builtins.match ".*dag-core.*" (builtins.readFile ./README.md)) != null;
    expected = true;
  };

  testFlakeExposesFormatter = {
    expr = (builtins.match ".*treefmt.*" (builtins.readFile ./flake.nix)) != null;
    expected = true;
  };
}
