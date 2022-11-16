# Gupax source files
* [Structure](#Structure)
* [Bootstrap](#Bootstrap)
* [State](#State)
* [Scale](#Scale)

## Structure
| File/Folder    | Purpose |
|----------------|---------|
| `constants.rs` | General constants needed in Gupax
| `disk.rs`      | Code for writing to disk: `state.toml`, `nodes.toml`; This holds the structs for mutable [State]
| `ferris.rs`    | Cute crab `--ferris`
| `gupax.rs`     | `Gupax` tab
| `main.rs`      | `App/Tab/State` + misc functions
| `node.rs`      | Community node feature
| `p2pool.rs`    | `P2Pool` tab
| `status.rs`    | `Status` tab
| `update.rs`    | Update code for the `Gupax` tab
| `xmrig.rs`     | `XMRig` tab

## Bootstrap
This is how Gupax works internally when starting up, divided into 3 sections.

1. **INIT**
	- Initialize custom console logging with `log`, `env_logger` || *warn!*
	- Initialize misc data (structs, text styles, thread count, images, etc) || *panic!*
	- Check for admin privilege (for XMRig) || *warn!*
	- Attempt to read `gupax.toml` || *warn!*, *initialize config with default options*
	- If errors were found, pop-up window
	
2. **AUTO**
	- If `auto_update` == `true`, pop-up auto-updating window || *info!*, *skip auto-update*
	- Multi-threaded GitHub API check on Gupax -> P2Pool -> XMRig || *warn!*, *skip auto-update*
	- Multi-threaded download if current version != new version || *warn!*, *skip auto-update*
	- After download, atomically replace current binaries with new || *warn!*, *skip auto-update*
	- Update version metadata || *warn!*, *skip auto-update*
	- If `auto_select` == `true`, ping community nodes and select fastest one || *warn!*

3. **MAIN**
	- All data must be initialized at this point, either via `gupax.toml` or default options || *panic!*
	- Start `App` frame || *panic!*
	- Write state to `gupax.toml` on user clicking `Save` (after checking input for correctness) || *warn!*
	- If `ask_before_quit` == `true`, check for running processes, unsaved state, and update connections before quitting
	- Kill processes, kill connections, exit

## State
Internal state is saved in the "OS data folder" as `gupax.toml`, using the [TOML](https://github.com/toml-lang/toml) format. If the version can't be parsed (not in the `vX.X.X` or `vX.X` format), the auto-updater will be skipped. [If not found, a default `gupax.toml` file will be created with `State::default`.](https://github.com/hinto-janaiyo/gupax/blob/main/src/state.rs) Gupax will `panic!` if `gupax.toml` has IO or parsing issues.

| OS       | Data Folder                              | Example                                                   |
|----------|----------------------------------------- |-----------------------------------------------------------|
| Windows  | `{FOLDERID_LocalAppData}`                | C:\Users\Alice\AppData\Roaming\Gupax\gupax.toml           |
| macOS    | `$HOME`/Library/Application Support      | /Users/Alice/Library/Application Support/Gupax/gupax.toml |
| Linux    | `$XDG_DATA_HOME` or `$HOME`/.local/share | /home/alice/.local/share/gupax/gupax.toml                 |

## Scale
Every frame, the max available `[width, height]` are calculated, and those are used as a baseline for the Top/Bottom bars, containing the tabs and status bar. After that, all available space is given to the middle ui elements. The scale is calculated every frame so that all elements can scale immediately as the user adjusts it; this doesn't take as much CPU as you might think since frames are only rendered on user interaction. Some elements are subtracted a fixed number because the `ui.seperator()`s add some fixed space which needs to be accounted for.

```
Main [App] outer frame (default: [1280.0, 720.0])
├─ Inner frame (1264.0, 704.0)
   ├─ TopPanel     = [width: (max-90.0)/5.0, height: max/10.0]
   ├─ BottomPanel  = [width: max, height: max/18.0]
   ├─ CentralPanel = [width: (max/8.0), height: the rest
```