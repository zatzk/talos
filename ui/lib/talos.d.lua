---@meta
--
-- The plugin API, as types. Loaded by lua-language-server through
-- `.luarc.json`'s `workspace.library`; never loaded by the plugin VM.
--
-- It exists because every mistake the API allows is silent. `convert.rs` drops
-- a node key it does not know (that is how a plugin carries its own bookkeeping
-- on the node table), `command` reads a fixed list of option names and ignores
-- the rest, and `lib/theme.lua`'s `__index` answers nil for a role no palette
-- defines. Each of those renders something plausible and reports nothing, so
-- the only place a typo can be caught is before it runs.
--
-- Two shapes do the catching, because lua-language-server will not flag an
-- *extra* key in a table constructor:
--
--   * every kind and every verb declares the field it cannot work without, so
--     a misspelt one reads as that field missing (`missing-fields`);
--   * every field whose value is drawn from a fixed set is that set, so a
--     misspelt value is a type mismatch (`assign-type-mismatch`).
--
-- `tests/fixtures/lua_types` + `scripts/ci/check-lua-types.sh` hold this to it.
--
-- Keep in step with `src/kernel/node.rs` and `src/kernel/convert.rs` (nodes),
-- `src/kernel/host/load.rs` (the declaration), `src/kernel/host/publish.rs` and
-- `LuaHost::enter` (`talos.*`), `src/kernel/command/mod.rs` (verbs) and
-- `src/kernel/theme.rs` (roles) — and with `talos.yml`, which is the same
-- contract for selene.
--
-- The two files split by LIFETIME. A global, or a field of a published table,
-- is a NAME selene can see, and it belongs in `talos.yml`. The shape of a
-- value that exists only while a call is running — a `hit`, a `key`, a `wheel`,
-- a row out of a list, an answer out of `talos.runs` — is unreachable from
-- there and is declared here.

---@alias talos.Color string A role's colour, `#rrggbb`, a name, or a 0-255 index.

--- A style: a bare colour is shorthand for a foreground.
---@class talos.Style
---@field fg? talos.Color
---@field bg? talos.Color
---@field bold? boolean
---@field dim? boolean
---@field italic? boolean
---@field underline? boolean
---@field reversed? boolean
---@field crossed_out? boolean

---@alias talos.StyleSpec talos.Color|talos.Style

--- One styled run within a line.
---
--- `id`/`role` make the run itself a click target over the columns it is laid
--- out at, so a chip inside a line needs no node of its own. Adjacent runs
--- carrying the SAME identity coalesce into one hitbox, which is how a
--- two-colour button (` ◀ F9 `) stays one target.
---@class talos.Span
---@field text string|number|boolean
---@field style? talos.StyleSpec
---@field id? string
---@field role? string

--- One line: a bare string, one span, or a list of spans.
---@alias talos.Line string|number|boolean|talos.Span|talos.Span[]

--- What a `text` node or a frame title accepts: one line, or a list of them.
---@alias talos.Text talos.Line|talos.Line[]

--- Runs painted onto a frame's own border cells, after the block is drawn.
---
--- Border cells cost no content row or column, which is why the status-dot
--- strip, the `▲ N`/`▼ N` scroll counts and the terminal scrollbar live here.
--- Each slot is clipped between the corners, which are never painted over.
---@class talos.Overlay
---@field top_left? talos.Text Rightward from the cell after the top-left corner.
---@field top_right? talos.Text Leftward from the cell before the top-right corner.
---@field bottom_left? talos.Text
---@field bottom_right? talos.Text
---@field right_column? talos.Text One run per inner row, down the right border column.

--- A frame drawn around a node. `borders` is all-or-none.
---@class talos.Frame
---@field title? talos.Text
---@field title_align? "left"|"center"|"centre"|"right"
---@field borders? "all"|"none"
---@field border_type? "rounded"|"square"|"plain"|"thick"
---@field border_style? talos.StyleSpec
---@field style? talos.StyleSpec
---@field padding? integer
---@field overlay? talos.Overlay

--- What every node may say about its space and its identity.
---
--- Sizing resolves exact → percentage → share, each clamped by `min`/`max`; a
--- node that asks for nothing takes an equal share of the remainder. `role`
--- carries the kernel's own click verbs (`action:`, `key:`, `focus:`, `url:`,
--- and the bare `drag`); any other role hands the click to the plugin.
---@class talos.NodeCommon
---@field len? integer
---@field pct? number
---@field fill? number
---@field min? integer
---@field max? integer
---@field id? string
---@field class? string|string[]
---@field role? string
---@field frame? talos.Frame|string|boolean
---@field block? talos.Frame|string|boolean The POC's spelling of `frame`.

--- Text. `style` paints across the whole rect before the spans go on top, so a
--- span that names its own colour keeps it — that is a selection bar or a
--- hover band, and `widgets.list` exposes it as `selected_style`/`hover_style`.
---@class (exact) talos.TextNode : talos.NodeCommon
---@field type? "text"|"paragraph"|"line"
---@field text talos.Text
---@field align? "left"|"center"|"centre"|"right"
---@field wrap? boolean
---@field scroll? integer
---@field style? talos.StyleSpec

--- A row or a column. Children live under `children`, or in the array part.
---@class (exact) talos.BoxNode : talos.NodeCommon
---@field type? "box"|"vstack"|"hstack"|"column"|"row"|"stack"
---@field axis? "vertical"|"horizontal"|"column"|"row"|"v"|"h"
---@field gap? integer
---@field children? talos.Node[]

--- A text field. `focused` claims the one caret, and `cursor` counts characters.
---@class (exact) talos.InputNode : talos.NodeCommon
---@field type? "input"|"field"
---@field value string
---@field cursor? integer
---@field placeholder? string
---@field focused? boolean
---@field style? talos.StyleSpec

--- Pre-rendered cells: a live session's terminal, a program this plugin asked
--- to run, or lines the plugin produced itself. Exactly one source.
---@class (exact) talos.SurfaceNode : talos.NodeCommon
---@field type? "surface"|"terminal"
---@field session? string
---@field program? string
---@field cells? talos.Line[]
---@field scroll? integer
---@field mark? integer A row to highlight, counted from the top of the surface — where a pane that scrolled a terminal to a line points at it.

---@alias talos.Node talos.TextNode|talos.BoxNode|talos.InputNode|talos.SurfaceNode

--- What a render RETURNS: any node, plus the one key that is read only there.
---
--- `float` is on these and not on `talos.NodeCommon` because the kernel reads
--- it from the returned value itself (`host/load.rs`'s `read_float`, called on
--- the render's result in `LuaHost::render`) and never walks the tree for it.
--- On the shared shape the type said a child could carry one, which `convert.rs`
--- drops in silence.
---
--- This buys accuracy and editor completion, not a diagnostic: luals does not
--- flag an extra key in a table constructor, so a `float` on a child is no more
--- reported now than it was before, and `talos.Float` has no required field
--- for a misspelt `widht` to be missing. Verified by probing both.
---@class (exact) talos.RootText : talos.TextNode
---@field float? talos.Float|boolean `true` takes the default size.
---@class (exact) talos.RootBox : talos.BoxNode
---@field float? talos.Float|boolean
---@class (exact) talos.RootInput : talos.InputNode
---@field float? talos.Float|boolean
---@class (exact) talos.RootSurface : talos.SurfaceNode
---@field float? talos.Float|boolean

---@alias talos.Root talos.RootText|talos.RootBox|talos.RootInput|talos.RootSurface

--- A cell on the screen, as `hit.screen_x`/`hit.screen_y` report it: where an
--- anchored float opens. Past an edge the float opens the other way round.
---@class (exact) talos.FloatAt
---@field x integer
---@field y integer

--- How big a floating pane asks to be: a share of the screen, or exact cells —
--- and, with `at`, where it opens instead of the centre.
---@class (exact) talos.Float
---@field at? talos.FloatAt
---@field width? number
---@field height? number
---@field cols? integer
---@field rows? integer

--- What a render is told. `elapsed` is served through a metatable, so reading it
--- is what marks a `pure` pane as depending on the animation clock.
---@class (exact) talos.Ctx
---@field width integer
---@field height integer
---@field focused boolean
---@field frame integer
---@field name string
---@field slot string
---@field elapsed number

--- What a decorator is told. Smaller than a render's: it is handed the tree it
--- is transforming, not a slot of its own.
---@class (exact) talos.DecorateCtx
---@field width integer
---@field height integer

--- What `ui/layout.lua` is told. Smaller than a render's: an arrangement sees
--- the screen and which slots are occupied, and no plugin.
---@class (exact) talos.LayoutCtx
---@field width integer
---@field height integer
---@field slots table<string, boolean>

--- A key offered to `on_key`. Return true to consume it.
---@class (exact) talos.Key
---@field key string
---@field char? string
---@field ctrl boolean
---@field alt boolean
---@field shift boolean
---@field cmd boolean

--- A click on a node this plugin painted, with the rect it landed in.
--- Reached only for identity the kernel has no verb for.
---@class (exact) talos.Hit
---@field id? string
---@field class string
---@field role? string
---@field x integer
---@field y integer
---@field w integer
---@field h integer
---@field screen_x integer The pressed cell on the screen, 0-based — what `float.at` takes.
---@field screen_y integer
---@field dragging boolean
---@field clicks integer 2 for the second press on the same node in quick succession, else 1.

--- A wheel tick over this plugin's pane. Declining puts it back on the key path.
---@class (exact) talos.Wheel
---@field up boolean
---@field x integer
---@field y integer

---@alias talos.Event
---| "session.created"
---| "session.deleted"
---| "session.status"
---| "session.changed"
---| "session.post_create"
---| "session.post_delete"
---| "session.post_restart"
---| "session.post_restore"
---| "focus.session"
---| "focus.pane"
---| "command.done"
---| "command.failed"
---| "program.exited"
---| "interface.reloaded"
---| string A `user.<name>` a plugin emits.

--- A key a plugin declares. Declared as data so the registry can enumerate,
--- conflict-check and rebind it without calling the plugin.
---@class (exact) talos.Binding
---@field key string
---@field action string
---@field desc? string
---@field scope? "global"|"plugin"
---@field passthrough? boolean
---@field group? string

--- A palette row: an action reachable without a chord.
---@class (exact) talos.CommandDecl
---@field action string
---@field desc? string

---@class (exact) talos.ActionArgument
---@field name string
---@field kind "string"|"uuid"
---@field required? boolean

---@class (exact) talos.ActionDecl
---@field name string
---@field desc? string
---@field scope? "global"|"plugin"
---@field effect? "read"|"ui-write"|"kernel-write"
---@field destructive? boolean
---@field args? talos.ActionArgument[]

--- An entry in the action band.
---@class (exact) talos.Pill
---@field action string
---@field label string
---@field priority? integer

--- A switch in the settings panel, owned by the declaring plugin.
---@class (exact) talos.SettingDecl
---@field id string
---@field desc? string
---@field default boolean|number|string

--- What a plugin file returns.
---
--- `render` is required unless the plugin `decorates` another, which draws
--- nothing of its own. Its return type is `talos.Root` rather than
--- `talos.Node` because `float` is read from that value alone; `decorate`
--- keeps `talos.Node`, since nothing reads `float` off its result.
---@class talos.Plugin
---@field name? string Defaults to the filename, minus a numeric ordering prefix.
---@field slot? string Defaults to `"center"`.
---@field slot_mode? "stack"|"switch"
---@field order? number Defaults to 100.
---@field focusable? boolean
---@field pure? boolean Cache the tree until an input changes.
---@field floats? boolean
---@field input? "session"
---@field size? talos.Size
---@field decorates? string A slot whose tree this plugin transforms.
---@field keys? talos.Binding[]
---@field pills? talos.Pill[]
---@field settings? talos.SettingDecl[]
---@field commands? talos.CommandDecl[]
---@field actions? talos.ActionDecl[]
---@field events? talos.Event[]
---@field capabilities? ("run"|"program")[]
---@field render? fun(ctx: talos.Ctx): talos.Root
---@field decorate? fun(node: talos.Node, ctx: talos.DecorateCtx): talos.Node
---@field on_key? fun(key: talos.Key): boolean
---@field on_action? fun(action: string, args?: table<string, string>): boolean
---@field on_click? fun(hit: talos.Hit): boolean
---@field on_context? fun(hit: talos.Hit): boolean A RIGHT press on the same node.
---@field on_outside? fun(hit: talos.Hit): boolean A float's: a press of either button that missed it while it held the pointer. `hit.id` is nil.
---@field on_scroll? fun(wheel: talos.Wheel): boolean
---@field on_event? fun(name: string, payload: table<string, any>)
---@field ui_state? fun(): table<string, string|number|boolean> Bounded public projection for `ui state`; omit private values.

--- What a pane asks of its slot, in the same vocabulary a node uses.
---@class (exact) talos.Size
---@field len? integer
---@field pct? number
---@field fill? number
---@field min? integer
---@field max? integer

---@alias talos.Role
---| "accent"
---| "accent_bright"
---| "status_working"
---| "status_blocked"
---| "status_done"
---| "status_idle"
---| "status_error"
---| "status_unreachable"
---| "status_running"
---| "status_unknown"
---| "text_primary"
---| "text_secondary"
---| "text_muted"
---| "border_focused"
---| "border_unfocused"
---| "role_name"
---| "branch_name"
---| "search_bar"
---| "keybind_hint"
---| "tool_allowed"
---| "tool_disallowed"
---| "danger"
---| "selection_bg"
---| "selection_fg"
---| "modal_dim_bg"
---| "modal_bg"
---| "modal_border"
---| "inverted_fg"
---| "diff_added"
---| "diff_removed"
---| "diff_added_bg"
---| "diff_removed_bg"
---| "app_bg"

--- A session's derived status, as `SessionState::as_str` spells it.
--- `lib/theme.lua` has a glyph and a role for `working`, `blocked`, `done`,
--- `idle`, `unreachable`, `running`, `uncovered` and `unreported`, and falls
--- back to `idle` only for `stopped` — which is at rest by definition. The
--- three silences are drawn apart from `idle` on purpose: `running` is "an
--- agent holds the pane and said nothing", `uncovered` is "wired to report
--- nothing" and `unreported` is "can report, has not yet". None of them is
--- the agent saying it is at rest.
---@alias talos.Status
---| "working"
---| "blocked"
---| "done"
---| "idle"
---| "unreachable"
---| "stopped"
---| "running"
---| "uncovered"
---| "unreported"

--- Uncommitted work in a session's tree. Absent until it has been measured,
--- which a plugin must be able to tell apart from a clean tree.
---@class (exact) talos.GitStats
---@field files integer
---@field insertions integer
---@field deletions integer
---@field untracked integer
---@field dirty boolean
---@field ahead integer
---@field behind integer
---@field merged? boolean Absent when nobody could say.

---@class (exact) talos.Session
---@field id string
---@field name string
---@field agent string What the row was created as.
---@field reports_as? string What a driver declared this session runs (`session reports-as`).
---@field detected_agent? string The registered agent observed in the pane, when it is not `agent`.
---@field status talos.Status
---@field backend string
---@field repo? string
---@field branch? string
---@field base_branch? string What a diff is taken against.
---@field host? string
---@field parent? string
---@field cwd? string
---@field worktrees integer
---@field repos string[]
---@field activity? string What the agent said about itself.
---@field notification? string
---@field display_order integer
---@field git? talos.GitStats
---@field attach_error? string Why this session's terminal is not live.

---@class (exact) talos.DeletedSession
---@field id string
---@field name string
---@field agent string
---@field deleted_at integer Epoch millis, against `talos.taken_at_ms`.
---@field worktrees integer
---@field partial boolean Restoring recovers committed work only.
---@field restore_refusal? string Set when the restore would refuse, and why.

---@class (exact) talos.Repo
---@field path string
---@field name string

---@alias talos.Presence
---| "present"
---| "missing"
---| "unknown"

---@class (exact) talos.Agent
---@field name string
---@field command string
---@field presence talos.Presence Whether `command` resolves to something runnable. `unknown` is not `missing`: it means nothing was looked at.

---@class (exact) talos.Host
---@field name string
---@field detail string
---@field backend string
---@field platform "posix"|"windows"
---@field multiplexer? string
---@field available_multiplexers string[]

--- The local multiplexer every session's window is created in.
---@class (exact) talos.Mux
---@field binary string `tmux`, or `psmux` on native Windows.
---@field configured? string
---@field available string[]
---@field presence talos.Presence
---@field advice string What to do about it; empty when there is nothing to do.

--- Whether the binaries a session needs are installed, as the kernel last
--- looked. Probed on the kernel's schedule behind a TTL — reading it is a table
--- lookup, never a `which`.
---@class (exact) talos.Preflight
---@field mux talos.Mux

---@class (exact) talos.Task
---@field id integer
---@field title string
---@field description? string
---@field status string
---@field source string
---@field url? string
---@field created_at integer Epoch seconds.
---@field updated_at integer

---@class (exact) talos.AutomationRun
---@field started_at integer
---@field status string
---@field detail string

---@class (exact) talos.Automation
---@field id integer
---@field name string
---@field schedule string
---@field action string
---@field enabled boolean
---@field last_outcome? string
---@field last_detail? string
---@field runs talos.AutomationRun[]

--- A command this interface accepted but has not finished.
---@class (exact) talos.InFlight
---@field id integer
---@field kind string
---@field session string
---@field phase string
---@field subject? string A `create`'s repository name, or a `bookmark` write's path as issued.
---@field host? string The machine a creation will land on; nil for this one.
---@field error? string

--- One line a content search found, in a session's agent pane or its shell.
---@class (exact) talos.SearchHit
---@field session string
---@field shell boolean Found in the companion shell rather than the agent.
---@field text string The line, trimmed and windowed around the match.
---@field positions integer[] 1-based character indices of `text` that matched — what `lib.fuzzy.spans` lights.
---@field back integer Rows between the line and the bottom of its terminal.
---@field scroll integer The scrollback offset that puts the line on screen; 0 when it already is.
---@field row integer The screen row, from the top, the line is on once scrolled to `scroll`.
---@field exact boolean Every term matched as a substring, phrase or regex — none only as a subsequence.
---@field score integer Higher is better; hits arrive already ranked.

--- A finished content search (`kernel::search::Answer`).
---@class (exact) talos.SearchAnswer
---@field query string The `store.want_content` it answers.
---@field within? string The `store["want_content.sessions"]` it was limited to, if any.
---@field hits talos.SearchHit[] Best first, at most 200 (50 per session).
---@field total integer Matching lines found, before the cap.
---@field sessions integer Sessions whose terminals were read.
---@field lines integer Lines searched.
---@field ms number Time the worker took.
---@field error? string Why the query could not run — an invalid regex.

---@class (exact) talos.DiffFile
---@field path string
---@field added integer
---@field removed integer
---@field status string
---@field old_path? string

--- A session's diff. Branch on `state`.
---@class (exact) talos.Diff
---@field state "pending"|"ready"|"failed"
---@field error? string
---@field truncated? boolean
---@field raw_bytes? integer
---@field untracked_omitted? integer
---@field files? talos.DiffFile[]
---@field body? string[]

--- A link found in a session's terminal, at the cell it starts on.
---@class (exact) talos.Link
---@field url string
---@field row integer
---@field col integer

--- An answer to a program this plugin asked to run. Branch on `state`: a
--- program that has not answered is not one that answered with nothing.
---@class (exact) talos.Run
---@field state "pending"|"done"|"failed"
---@field error? string
---@field stdout? string
---@field stderr? string
---@field status? integer
---@field truncated? boolean
---@field timed_out? boolean
---@field ok? boolean

---@class (exact) talos.Features
---@field tasks boolean
---@field automations boolean
---@field file_viewer boolean
---@field global_search boolean
---@field info_panel boolean
---@field shell_pane boolean
---@field code_review boolean
---@field perf_hud boolean
---@field mouse boolean
---@field notifications boolean
---@field soft_delete boolean
---@field version_check boolean
---@field auto_update boolean

--- The settings in force. Read your own switch and decline to draw when it is
--- off — the kernel gates only what it owns.
---@class (exact) talos.Settings
---@field features talos.Features
---@field two_panel_min_cols integer
---@field three_panel_min_cols integer
---@field scrollback_lines integer

---@class (exact) talos.BookmarkRow
---@field path string
---@field name string
---@field parent? string
---@field label? string
---@field is_parent boolean
---@field offered boolean
---@field is_git? boolean

--- Remembered repositories, served only while `store.want_bookmarks` asks.
---@class (exact) talos.Bookmarks
---@field host string
---@field loading boolean
---@field rows talos.BookmarkRow[]

---@class (exact) talos.BrowseEntry
---@field name string
---@field is_git boolean

--- A directory listing, served only while `store.want_browse` asks.
---@class (exact) talos.Browse
---@field host string
---@field dir string
---@field loading boolean
---@field error? string
---@field entries talos.BrowseEntry[]

--- Base branches, served only while `store.want_branches` asks.
---@class (exact) talos.Branches
---@field host string
---@field repo string
---@field loading boolean
---@field error? string
---@field list string[]

---@class (exact) talos.WorktreeEntry
---@field path string
---@field branch string

--- The worktrees a repo already has, served while `store.want_worktrees` asks.
---@class (exact) talos.Worktrees
---@field host string
---@field repo string
---@field loading boolean
---@field error? string
---@field list talos.WorktreeEntry[]

---@class (exact) talos.SystemMetrics
---@field cpu_percent number
---@field memory_used integer
---@field memory_total integer

--- What an agent reported about its own turn, from its statusline. Every field
--- is absent rather than zero when the agent did not report it, so a panel
--- renders the rows it has — `publish.rs`'s `agent_metrics_table` drops a nil.
--- The counts arrive as Lua numbers rather than integers: they cross as `f64`.
---@class (exact) talos.AgentMetrics
---@field model? string The display name.
---@field model_id? string
---@field cli_version? string
---@field cost_usd? number
---@field duration_ms? number
---@field api_duration_ms? number
---@field lines_added? number
---@field lines_removed? number
---@field input_tokens? number
---@field output_tokens? number
---@field context_window? number
---@field context_used_percent? number
---@field current_input_tokens? number
---@field current_output_tokens? number
---@field cache_creation_tokens? number
---@field cache_read_tokens? number

--- One account rate-limit window.
---@class (exact) talos.UsageWindow
---@field label string
---@field used_percent number
---@field resets_at? integer Epoch seconds; absent when the account did not say.

--- The account's rate-limit windows, shared by every session on that agent.
---@class (exact) talos.Usage
---@field windows talos.UsageWindow[]
---@field plan? string
---@field note? string

---@class talos.SessionMetrics
---@field cpu_percent? number
---@field memory_bytes? integer
---@field agent? talos.AgentMetrics
---@field usage? talos.Usage

---@class (exact) talos.Metrics
---@field system talos.SystemMetrics
---@field sessions table<string, talos.SessionMetrics>

--- The machine this is running on, from the values the binary was built for.
---@class (exact) talos.Platform
---@field os string
---@field arch string

--- What the pointer is over, as whichever of the two the affordance was marked
--- with. Empty when nothing is hovered.
---@class (exact) talos.Hover
---@field id? string
---@field role? string

---@class (exact) talos.ThemeChoice
---@field name string
---@field display_name string
---@field light boolean
---@field custom boolean

---@class (exact) talos.ThemeSnapshot
---@field name string
---@field roles table<talos.Role, talos.Color>
---@field nerd_font boolean
---@field choices talos.ThemeChoice[]

---@class (exact) talos.RegistryKey
---@field plugin string
---@field action string
---@field key string
---@field default_key string
---@field desc string
---@field scope "global"|"plugin"
---@field rebound boolean
---@field group string

--- A chord-less action a plugin declares in `commands`, for the palette.
---@class (exact) talos.RegistryCommand
---@field plugin string
---@field action string
---@field desc string

---@class (exact) talos.RegistrySetting
---@field plugin string
---@field id string
---@field desc string
---@field type string
---@field value boolean|number|string
---@field default boolean|number|string

--- What every plugin declared, so help and settings render from it.
---@class (exact) talos.RegistrySnapshot
---@field keys talos.RegistryKey[]
---@field commands talos.RegistryCommand[]
---@field settings talos.RegistrySetting[]
---@field sections string[] The order help renders its sections in.

--- One of the interface's own files: where it came from, and whether it runs.
---@class (exact) talos.PluginRow
---@field path string
---@field name string
---@field kind string
---@field slot string
---@field source string
---@field state string
---@field error? string

--- What the arrangement needs to know about the bands, and no more.
---@class (exact) talos.Chrome
---@field status_rows integer

--- Capabilities THIS plugin has been granted. A boolean about a decision the
--- user already made; it grants nothing.
---@class (exact) talos.Granted
---@field run? boolean
---@field program? boolean

--- Everything readable. Rebuilt each publish; a group whose inputs did not move
--- is handed back as the same table, which is what `lib/theme.lua` memoizes on.
---@class (exact) talos.Api
---@field sessions talos.Session[]
---@field deleted talos.DeletedSession[]
---@field repos talos.Repo[]
---@field agents talos.Agent[]
---@field agent_default string What a bare launch would use.
---@field settings talos.Settings
---@field bookmarks talos.Bookmarks
---@field browse talos.Browse
---@field branches talos.Branches
---@field worktrees talos.Worktrees
---@field hosts talos.Host[]
---@field preflight talos.Preflight
---@field tasks talos.Task[]
---@field automations talos.Automation[]
---@field commands talos.InFlight[]
---@field diffs table<string, talos.Diff>
---@field links table<string, talos.Link[]> Keyed by SURFACE, not by session: a session's companion shell is `<id>#shell` and has links of its own.
---@field search? talos.SearchAnswer The content search's answer, while `store.want_content` asks. Nil until one lands.
---@field printing table<string, boolean> Sessions whose pane is producing output right now, keyed by id. The evidence `running` animates on — see `ui.status`.
---@field runs table<string, talos.Run> Answers to THIS plugin's runs.
---@field granted talos.Granted
---@field metrics talos.Metrics
---@field platform talos.Platform
---@field version string
---@field reloads integer
---@field can_open_links boolean
---@field taken_at_ms integer When these rows were read.
---@field error? string
---@field focus string Which pane holds focus, by name.
---@field selection string The mouse text selection, empty when nothing is selected.
---@field hover talos.Hover
---@field plugins talos.PluginRow[]
---@field ui_dir string
---@field chrome talos.Chrome
---@field theme talos.ThemeSnapshot
---@field registry talos.RegistrySnapshot
talos = {}

--- Persistent, shared by every plugin — the bus between them. Survives a
--- reload, not a restart.
---@type table<string, any>
store = {}

--- Persistent and private to the declaring plugin. Same lifetime as `store`.
---@type table<string, any>
state = {}

---@class (exact) talos.FileEntry
---@field name string
---@field dir boolean True for a directory, and for a symlink to one inside the session.

--- Directory entries and file text, rooted at a session's working directory.
--- Not a filesystem: a path outside the root is refused.
---@class talos.Files
files = {}

---@param session string
---@param path? string Relative to the session's root; defaults to the root.
---@return talos.FileEntry[]
function files.list(session, path) end

---@param session string
---@param path string
---@return string
function files.read(session, path) end

---@class (exact) talos.TruncateOpts
---@field ellipsis? string What marks the cut. `""` cuts without a mark.
---@field side? "right"|"left"|"middle" Which end is eaten. Defaults to `"right"`.

--- Display width, in terminal COLUMNS.
---
--- The one measurement Lua cannot make: `#` counts bytes and `utf8.len` counts
--- codepoints, and a CJK glyph is one codepoint over two columns. Backed by the
--- same `unicode-width` the painter measures with, so a budget computed here
--- agrees with what lands on the screen.
---@class talos.TextApi
text = {}

---@param str string
---@return integer
function text.width(str) end

--- Cut `str` down to `cols` columns. `opts` is the ellipsis, or a table. An
--- unknown `side` raises rather than being quietly read as `"right"`.
---@param str string
---@param cols integer
---@param opts? string|talos.TruncateOpts
---@return string
function text.truncate(str, cols, opts) end

--- Space `str` out to `cols` columns. Never truncates: what to do with text
--- that is already too wide is the caller's choice.
---@param str string
---@param cols integer
---@param align? "left"|"right"|"center"|"centre"
---@return string
function text.pad(str, cols, align) end

---@class (exact) talos.RunOpts
---@field session? string Run in this session's directory.
---@field ttl? number Seconds an answer stays fresh.
---@field timeout? number Seconds before the program is given up on.
---@field refresh? boolean Ask again even if a fresh answer is held.

--- Ask for a program and read the answer later, from `talos.runs[key]`.
---
--- Queued, never executed here — a plugin cannot call anything that waits.
--- Absent unless this plugin declares `capabilities = { "run" }` AND the user
--- has trusted it, so `if not run then` is the honest check.
---@param key string
---@param program string
---@param opts? talos.RunOpts
function run(key, program, opts) end

--- Loads any `.lua` under the interface directory, and nothing outside it.
---@param name string
---@return any
function require(name) end

--- A user event. Not exact: every other scalar on the table travels as the
--- event's payload, which is the one place an unknown key is meaningful.
---@class talos.cmd.Emit
---@field text string The event's name; subscribers see `user.<name>`.

---@class (exact) talos.cmd.Plugin
---@field file string A path within the interface directory.
---@field action "restore"|"remove"

---@class (exact) talos.cmd.Set
---@field text string A `plugin.setting` key.
---@field flag? boolean
---@field number? number
---@field value? string A text setting's value.
---@field reset? boolean Put the setting back to its default.

---@class (exact) talos.cmd.Task
---@field text? string A title: creates when there is no `number`.
---@field number? integer An existing task's id.
---@field status? string
---@field reset? boolean Delete it.

---@class (exact) talos.cmd.Dispatch
---@field number integer The task to dispatch.
---@field session? string

---@class (exact) talos.cmd.Automation
---@field number integer
---@field flag? boolean Enable or disable it.
---@field force? boolean Run it now.
---@field reset? boolean Delete it.

---@class (exact) talos.cmd.ExtraMember
---@field path string
---@field worktree? boolean

---@class (exact) talos.cmd.Create
---@field repo string
---@field text? string The session's name.
---@field branch? string
---@field base? string
---@field worktree_path? string An existing worktree to open rather than make.
---@field agent? string
---@field host? string
---@field multiplexer? string
---@field extras? talos.cmd.ExtraMember[] Further repositories to span.

---@class (exact) talos.cmd.Bookmark
---@field repo string The path to remember or forget.
---@field action "add"|"remove"|"parent"|"create"|"init"|"clone" `create`/`init`/`clone` make the directory first (it must not exist, or be empty), then remember it.
---@field text? string The URL to clone, for `clone`.
---@field host? string

---@class (exact) talos.cmd.Focus
---@field text string The plugin to focus.
---@field toggle? boolean Return to the previous pane when it already has focus.

---@class (exact) talos.cmd.Open
---@field text string The url. `url` is not read.

---@class (exact) talos.cmd.Theme
---@field text string The theme's name.

---@class (exact) talos.cmd.Order
---@field list string[] Every session id, in the order wanted.

---@class (exact) talos.cmd.Program
---@field text string This plugin's name for the pane.
---@field repo? string The program to run; required unless closing or typing.
---@field args? string[]
---@field action? "close"|"stop"
---@field keys? string Type this into the program already running in the pane.
--- The bytes reach its stdin as if typed, so `"\r"` is Enter and `"\27"` is
--- Escape. Names the pane through `text`: it is how a long-lived program is
--- told something rather than replaced. On its own it starts nothing and says
--- so when the pane holds nothing live; sent *with* `repo`, a pane with nothing
--- running is started from it instead, and these keys are not sent.

---@class (exact) talos.cmd.Delete
---@field session string
---@field force? boolean Skip the undo window.
---@field remember? talos.cmd.Remember Store an undo target only when the soft delete is issued.

---@class (exact) talos.cmd.Remember
---@field key string
---@field value string

---@class (exact) talos.cmd.Restore
---@field session string
---@field force? boolean Restore what can be restored.

---@class (exact) talos.cmd.Session
---@field session string A session id, or for `copy` a surface: `<id>#shell` reads the companion shell's screen rather than the agent's.

---@class (exact) talos.cmd.Send
---@field session string
---@field text string

---@class (exact) talos.cmd.Reorder
---@field session string
---@field delta integer Non-zero.

---@class (exact) talos.cmd.Fork
---@field session string
---@field text? string The new session's name.

--- Refused, as `command.failed` with the reason, for any name `session create`
--- would refuse or another session on the same backend already has.
---@class (exact) talos.cmd.Rename
---@field session string
---@field text string The name to give it.

--- Run a declared action, exactly as its chord or a click on it would — which
--- is how a pane opens help, settings, themes or the palette from a key
--- handler. The plugin that asked is the fallback owner, stamped by the kernel.
---@class (exact) talos.cmd.Action
---@field text string The action id. `action` is not read.
---@field session? string Pass this row as the action's `session_id` argument.
---@field target? string Pass this row as the action's `target` argument.

--- Say something in the message band. The band stays kernel-drawn; this is a
--- sentence and a severity contributed to it, like a pill or a binding.
---@class (exact) talos.cmd.Message
---@field text string
---@field level? "info"|"success"|"error" Defaults to `info`; anything else is refused.

---@alias talos.Verb
---| "emit" | "plugin" | "set" | "task" | "dispatch" | "automation"
---| "create" | "bookmark" | "focus" | "open" | "theme" | "order" | "program"
---| "delete" | "restore" | "restart" | "send" | "reorder" | "fork"
---| "sync" | "rename" | "copy" | "diff" | "shell" | "editor" | "action" | "message"

--- The only way a plugin changes anything. Enqueues and returns; it never runs
--- the operation, which is why a plugin cannot stall the loop.
---
--- An option name no verb reads is collected and ignored, so the overloads
--- below name what each verb actually requires.
---@overload fun(verb: "emit", opts: talos.cmd.Emit)
---@overload fun(verb: "plugin", opts: talos.cmd.Plugin)
---@overload fun(verb: "set", opts: talos.cmd.Set)
---@overload fun(verb: "task", opts: talos.cmd.Task)
---@overload fun(verb: "dispatch", opts: talos.cmd.Dispatch)
---@overload fun(verb: "automation", opts: talos.cmd.Automation)
---@overload fun(verb: "create", opts: talos.cmd.Create)
---@overload fun(verb: "bookmark", opts: talos.cmd.Bookmark)
---@overload fun(verb: "focus", opts: talos.cmd.Focus)
---@overload fun(verb: "open", opts: talos.cmd.Open)
---@overload fun(verb: "theme", opts: talos.cmd.Theme)
---@overload fun(verb: "order", opts: talos.cmd.Order)
---@overload fun(verb: "program", opts: talos.cmd.Program)
---@overload fun(verb: "delete", opts: talos.cmd.Delete)
---@overload fun(verb: "restore", opts: talos.cmd.Restore)
---@overload fun(verb: "restart", opts: talos.cmd.Session)
---@overload fun(verb: "send", opts: talos.cmd.Send)
---@overload fun(verb: "reorder", opts: talos.cmd.Reorder)
---@overload fun(verb: "fork", opts: talos.cmd.Fork)
---@overload fun(verb: "sync", opts: talos.cmd.Session)
---@overload fun(verb: "rename", opts: talos.cmd.Rename)
---@overload fun(verb: "copy", opts: talos.cmd.Session)
---@overload fun(verb: "diff", opts: talos.cmd.Session)
---@overload fun(verb: "shell", opts: talos.cmd.Session)
---@overload fun(verb: "editor", opts: talos.cmd.Session)
---@overload fun(verb: "action", opts: talos.cmd.Action)
---@overload fun(verb: "message", opts: talos.cmd.Message)
---@param verb talos.Verb
---@param opts? table
function command(verb, opts) end
