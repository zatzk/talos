-- A pane that reads EVERY field `talos.yml` declares on the injected tables no
-- bundled pane reads in a form selene can see — `talos.granted`, `.platform`,
-- `.metrics`, `.hover`, `.preflight.mux`, `.settings`, `.theme.roles`, the four
-- creation-flow reads and `.runs` — each as a plain dotted path.
--
-- Every field, not a sample: a declaration this file does not name is one that
-- can be deleted from `talos.yml` with nothing failing. The counts in the
-- section comments are the whole declared set for that table, so a field added
-- there and not here is visible as a count that no longer matches.
--
-- It ends with the stdlib names in the same position: `_VERSION`, and the five
-- `math` functions Lua 5.4 has that `talos.yml` did not list. They are not
-- published by anything, but they are reachable in the VM and were rejected for
-- the same reason the tables above were — nothing in `ui/` happens to name them
-- — so they would go unnoticed the same way.
--
-- Expected to lint CLEAN, and that is the whole assertion here; `typos/` holds
-- the other direction, one pane per table. selene checks a dotted path against
-- `talos.yml` one segment at a time, so a table declared as a bare property
-- rejects the exact expression `ui/README.md` and `docs/PLUGINS.md` tell an
-- author to write. Each of these was, and CI stayed green because nothing in
-- `ui/` or `examples/` reads them in a form selene can see.
return {
  name = "std_reads",
  render = function()
    local read = {
      -- granted (2)
      tostring(talos.granted.run),
      tostring(talos.granted.program),
      -- platform (2)
      tostring(talos.platform.os),
      tostring(talos.platform.arch),
      -- metrics.system (3)
      tostring(talos.metrics.system.cpu_percent),
      tostring(talos.metrics.system.memory_used),
      tostring(talos.metrics.system.memory_total),
      -- hover (2)
      tostring(talos.hover.id),
      tostring(talos.hover.role),
      -- preflight.mux (3)
      tostring(talos.preflight.mux.binary),
      tostring(talos.preflight.mux.presence),
      tostring(talos.preflight.mux.advice),
      -- settings.features (13)
      tostring(talos.settings.features.tasks),
      tostring(talos.settings.features.automations),
      tostring(talos.settings.features.file_viewer),
      tostring(talos.settings.features.global_search),
      tostring(talos.settings.features.info_panel),
      tostring(talos.settings.features.shell_pane),
      tostring(talos.settings.features.code_review),
      tostring(talos.settings.features.perf_hud),
      tostring(talos.settings.features.mouse),
      tostring(talos.settings.features.notifications),
      tostring(talos.settings.features.soft_delete),
      tostring(talos.settings.features.version_check),
      tostring(talos.settings.features.auto_update),
      -- bookmarks (3)
      tostring(talos.bookmarks.host),
      tostring(talos.bookmarks.loading),
      tostring(talos.bookmarks.rows),
      -- browse (5)
      tostring(talos.browse.host),
      tostring(talos.browse.dir),
      tostring(talos.browse.loading),
      tostring(talos.browse.error),
      tostring(talos.browse.entries),
      -- branches (5)
      tostring(talos.branches.host),
      tostring(talos.branches.repo),
      tostring(talos.branches.loading),
      tostring(talos.branches.error),
      tostring(talos.branches.list),
      -- worktrees (5)
      tostring(talos.worktrees.host),
      tostring(talos.worktrees.repo),
      tostring(talos.worktrees.loading),
      tostring(talos.worktrees.error),
      tostring(talos.worktrees.list),
      -- theme.roles (33)
      tostring(talos.theme.roles.accent),
      tostring(talos.theme.roles.accent_bright),
      tostring(talos.theme.roles.app_bg),
      tostring(talos.theme.roles.border_focused),
      tostring(talos.theme.roles.border_unfocused),
      tostring(talos.theme.roles.branch_name),
      tostring(talos.theme.roles.danger),
      tostring(talos.theme.roles.diff_added),
      tostring(talos.theme.roles.diff_added_bg),
      tostring(talos.theme.roles.diff_removed),
      tostring(talos.theme.roles.diff_removed_bg),
      tostring(talos.theme.roles.inverted_fg),
      tostring(talos.theme.roles.keybind_hint),
      tostring(talos.theme.roles.modal_bg),
      tostring(talos.theme.roles.modal_border),
      tostring(talos.theme.roles.modal_dim_bg),
      tostring(talos.theme.roles.role_name),
      tostring(talos.theme.roles.search_bar),
      tostring(talos.theme.roles.selection_bg),
      tostring(talos.theme.roles.selection_fg),
      tostring(talos.theme.roles.status_blocked),
      tostring(talos.theme.roles.status_done),
      tostring(talos.theme.roles.status_error),
      tostring(talos.theme.roles.status_idle),
      tostring(talos.theme.roles.status_running),
      tostring(talos.theme.roles.status_unknown),
      tostring(talos.theme.roles.status_unreachable),
      tostring(talos.theme.roles.status_working),
      tostring(talos.theme.roles.text_muted),
      tostring(talos.theme.roles.text_primary),
      tostring(talos.theme.roles.text_secondary),
      tostring(talos.theme.roles.tool_allowed),
      tostring(talos.theme.roles.tool_disallowed),
      -- settings scalars (3), and the one map left at the table
      tostring(talos.settings.two_panel_min_cols),
      tostring(talos.settings.three_panel_min_cols),
      tostring(talos.settings.scrollback_lines),
      tostring(talos.metrics.sessions),
      -- Keyed by the key this plugin passed to `run`, so a literal is a
      -- dotted path where every other map is reached through a variable.
      tostring(talos.runs.cpu),
      tostring(talos.runs.cpu.state),
      -- The stdlib names this change declares.
      _VERSION,
      tostring(math.acos(0)),
      tostring(math.asin(1)),
      tostring(math.atan(1, 1)),
      tostring(math.deg(1)),
      tostring(math.rad(90)),
    }
    return { text = table.concat(read, " ") }
  end,
}
