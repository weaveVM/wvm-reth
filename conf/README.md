# Configuration

This configuration must be attached during the deployment of a node in the data directory folder.

The default data directory is platform dependent:

- Linux: `$XDG_DATA_HOME/reth/ or $HOME/.local/share/reth/`
- Windows: `{FOLDERID_RoamingAppData}/reth/`
- macOS: `$HOME/Library/Application Support/reth/`

## Prune Node

Under the consideration that WeaveVM produces 1 block per second, the ideal pruning configuration is set to 30 days

which is the following

```
let blocksPerSecond = 1;

// Blocks per second * 60s * 1m * 24h * 30d
(blocksPerSecond * 60 * 60 * 24 * 30)
```



