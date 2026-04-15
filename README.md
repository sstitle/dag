# Project Name

A Nix-based development environment with 'mask' task runner.

<!-- mdformat-toc start --slug=github --no-anchors --maxlevel=6 --minlevel=1 -->

- [Project Name](#project-name)
  - [Getting Started](#getting-started)
    - [Prerequisites](#prerequisites)
    - [Quick Start](#quick-start)
    - [Available Tools](#available-tools)
    - [Task Automation](#task-automation)
    - [Code Formatting](#code-formatting)
    - [Project Structure](#project-structure)

<!-- mdformat-toc end -->

## Getting Started

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- [direnv](https://direnv.net/) for automatic environment loading

### Quick Start

1. Enter the development shell:

   ```bash
   nix develop
   ```

1. Or with direnv installed:

   ```bash
   direnv allow
   ```

### Available Tools

This development environment includes:

- **mask**: Task runner for automating common development tasks
- **treefmt**: Multi-language code formatter
- **git**: Version control
- **direnv/nix-direnv**: Automatic environment loading

### Task Automation

This project uses [mask](https://github.com/jacobdeichert/mask) for task automation. View available tasks:

```bash
mask --help
```

### Code Formatting

Use the nix formatter which is managed by `treefmt.nix`:

```bash
nix fmt
```

### Project Structure

```
.
├── flake.nix          # Nix flake configuration
├── treefmt.nix        # Formatter configuration
├── maskfile.md        # Task definitions
├── .envrc             # direnv configuration
└── README.md          # This file
```
