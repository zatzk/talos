-- Talos pipeline — the fleet, as a plugin.
--
-- The run overview: every session as a row in its lead/worker tree, with the
-- status the agent reported, the branch it holds, and the shape of the work in
-- its tree. This is the supervision read of talos's own model — `--parent`
-- made the tree a fact, this pane makes it visible.
--
-- Reads come from the snapshot; writes go out as `action` commands. Nothing
-- privileged.
--
-- Install with:
--
--     talos-cli plugin install pipeline
--
-- A SWITCH pane in the centre: F8 brings it forward over the terminal.

local theme = require("lib.theme")
local widgets = require("lib.widgets")

local ROWS_MAX = 24

local function sessions()
  return (talos and talos.sessions) or {}
end

--- Sessions keyed by id, to resolve a worker's parent.
local function by_id()
  local map = {}
  for _, session in ipairs(sessions()) do
    map[session.id] = session
  end
  return map
end

--- The tree as rows: leads (no parent) with their workers nested underneath,
--- indented; orphans (parent gone) surface at top level rather than vanishing.
local function flatten()
  local map = by_id()
  local leads, workers_by_parent = {}, {}
  for _, session in ipairs(sessions()) do
    local parent = session.parent
    if parent and map[parent] then
      local list = workers_by_parent[parent] or {}
      list[#list + 1] = session
      workers_by_parent[parent] = list
    else
      leads[#leads + 1] = session
    end
  end

  local rows = {}
  local function emit(session, depth)
    rows[#rows + 1] = { session = session, depth = depth }
    for _, worker in ipairs(workers_by_parent[session.id] or {}) do
      emit(worker, depth + 1)
    end
  end
  for _, lead in ipairs(leads) do
    emit(lead, 0)
  end
  return rows
end

--- Per-pane state; a cursor survives a frame, not a restart.
local function ui()
  store.pipeline_ui = store.pipeline_ui or { cursor = 1 }
  return store.pipeline_ui
end

local function selected()
  local list = flatten()
  local at = widgets.clamp(ui().cursor, #list)
  return list[at], at
end

--- One row: indent, status glyph in its theme colour, name, agent, branch,
--- git shape. A merged tree wears the done colour: the branch's work landed.
local function row_of(entry, is_cursor)
  local session = entry.session
  local status = theme.status(session.status)
  local style = is_cursor and { fg = theme.accent, bold = true }
    or { fg = theme.text }

  local indent = string.rep("  ", entry.depth)
  local spans = {
    { text = "  " .. indent, style = { fg = theme.muted } },
    { text = status.glyph .. " ", style = { fg = status.color } },
    { text = session.name or "(unnamed)", style = style },
  }
  if session.agent then
    spans[#spans + 1] = { text = " " .. session.agent, style = { fg = theme.muted } }
  end
  if session.branch then
    spans[#spans + 1] = { text = " " .. session.branch, style = { fg = theme.branch } }
  end
  local git = session.git
  if git and git.dirty then
    spans[#spans + 1] = {
      text = string.format(" +%d -%d", git.insertions or 0, git.deletions or 0),
      style = { fg = theme.warn },
    }
    if git.merged then
      spans[#spans + 1] = { text = " merged", style = { fg = theme.info } }
    end
  end
  return spans
end

local function counts()
  local working, blocked, done = 0, 0, 0
  for _, session in ipairs(sessions()) do
    if session.status == "working" or session.status == "running" then
      working = working + 1
    elseif session.status == "blocked" then
      blocked = blocked + 1
    elseif session.status == "done" then
      done = done + 1
    end
  end
  return working, blocked, done
end

return {
  name = "pipeline",
  slot = "center",
  slot_mode = "switch",
  order = 82,
  focusable = true,

  keys = {
    { key = "f8", action = "pipeline.open", desc = "fleet", scope = "global", group = "Talos" },
    { key = "j", action = "pipeline.down", desc = "next session", group = "Talos" },
    { key = "k", action = "pipeline.up", desc = "previous session", group = "Talos" },
    { key = "enter", action = "pipeline.open_session", desc = "open session", group = "Talos" },
    { key = "d", action = "pipeline.diff", desc = "refresh diff", group = "Talos" },
  },

  render = function(ctx)
    local state = ui()
    local list = flatten()
    state.cursor = widgets.clamp(state.cursor, math.max(#list, 1))

    local children = {}
    local working, blocked, done = counts()

    if #list == 0 then
      children[#children + 1] = {
        type = "text",
        len = 1,
        text = theme.dim("  no sessions — Ctrl+N starts one"),
      }
    else
      local rows = {}
      for index, entry in ipairs(list) do
        rows[#rows + 1] = {
          spans = row_of(entry, index == state.cursor and ctx.focused),
          id = entry.session.id,
        }
      end
      local body = widgets.list({
        rows = rows,
        selected = state.cursor,
        height = math.max(1, math.min(ctx.height - 5, ROWS_MAX)),
      })
      body.fill = 1
      children[#children + 1] = body
    end

    children[#children + 1] = widgets.divider(ctx.width - 2)
    children[#children + 1] = {
      type = "text",
      len = 1,
      text = {
        {
          { text = "  " .. working .. " working", style = { fg = theme.warn } },
          { text = "  ·  " .. blocked .. " blocked", style = { fg = theme.bad } },
          { text = "  ·  " .. done .. " done", style = { fg = theme.info } },
        },
      },
    }
    children[#children + 1] = widgets.hints({
      { "enter", "open session" },
      { "d", "diff" },
      { "j/k", "move" },
    })

    return {
      type = "box",
      frame = widgets.panel("Talos fleet", ctx.focused),
      children = children,
    }
  end,

  on_action = function(action)
    local state = ui()
    local entry = selected()

    if action == "pipeline.open" then
      command("focus", { text = "pipeline", toggle = true })
      return true
    end
    if action == "pipeline.down" or action == "pipeline.up" then
      local list = flatten()
      local step = action == "pipeline.down" and 1 or -1
      state.cursor = widgets.clamp(state.cursor + step, math.max(#list, 1))
      return true
    end
    if not entry then
      return false
    end
    if action == "pipeline.open_session" then
      command("action", { text = "sessions.open", session = entry.session.id })
      return true
    end
    if action == "pipeline.diff" then
      command("diff", { session = entry.session.id })
      command("message", { text = "diff requested for " .. entry.session.name, level = "info" })
      return true
    end
    return false
  end,
}
