-- Talos attention — the inbox, as a plugin.
--
-- The octomux permission-inbox semantics over talos's own session states:
-- one place that answers "what is waiting on me?" instead of a wall of panes
-- each demanding a separate look.
--
--   Needs you       sessions whose agent reported `blocked` — a permission
--                   prompt, a question, a tool waiting on a yes
--   Ready to review sessions whose agent reported `done` while their tree is
--                   still dirty — the work is there, nobody has looked at it
--
-- Nothing here is privileged: reads come from the snapshot (`talos.sessions`
-- and their git stats), writes go out as `action`/`diff` commands. `hook_state`
-- is supervision, not completion — this pane is exactly the supervision read.
--
-- Install with:
--
--     talos-cli plugin install attention
--
-- A SWITCH pane in the centre: F7 brings it forward over the terminal.

local theme = require("lib.theme")
local widgets = require("lib.widgets")

local ROWS_MAX = 16

local function sessions()
  return (talos and talos.sessions) or {}
end

--- A session needs a human: its agent reported blocked.
local function needs_you(session)
  return session.status == "blocked"
end

--- A session is ready for review: the agent said done, and there is uncommitted
--- work in its tree. A done session with a clean tree owes nobody anything.
local function ready_to_review(session)
  if session.status ~= "done" then
    return false
  end
  local git = session.git
  return git ~= nil and git.dirty == true
end

--- Both sections, best-first: within each, longest-waiting first by name order
--- (the snapshot arrives in display order; stability is all we promise).
local function collect()
  local blocked, review = {}, {}
  for _, session in ipairs(sessions()) do
    if needs_you(session) then
      blocked[#blocked + 1] = session
    elseif ready_to_review(session) then
      review[#review + 1] = session
    end
  end
  return blocked, review
end

--- Per-pane state. `store` survives a frame; a cursor is not a setting.
local function ui()
  store.attention_ui = store.attention_ui or { cursor = 1 }
  return store.attention_ui
end

local function rows()
  local blocked, review = collect()
  local rows = {}
  for _, session in ipairs(blocked) do
    rows[#rows + 1] = { session = session, section = 1 }
  end
  for _, session in ipairs(review) do
    rows[#rows + 1] = { session = session, section = 2 }
  end
  return rows
end

local function selected()
  local list = rows()
  local at = widgets.clamp(ui().cursor, #list)
  return list[at], at
end

--- One row: status glyph, session name, agent, branch, and the git shape of
--- the work sitting in the tree. The notification/activity line, when there is
--- one, is the reason the session is here — it is the sentence to read.
local function row_of(entry, is_cursor)
  local session = entry.session
  local status = theme.status(session.status)
  local style = is_cursor and { fg = theme.accent, bold = true }
    or { fg = theme.text }

  local spans = {
    { text = "  " .. status.glyph .. " ", style = { fg = status.color } },
    { text = session.name or "(unnamed)", style = style },
    { text = "  " .. (session.agent or "?"), style = { fg = theme.muted } },
  }
  if session.branch then
    spans[#spans + 1] = { text = " " .. session.branch, style = { fg = theme.branch } }
  end
  local git = session.git
  if git and git.dirty then
    spans[#spans + 1] = {
      text = string.format(
        "  +%d -%d %dF",
        git.insertions or 0,
        git.deletions or 0,
        git.files or 0
      ),
      style = { fg = theme.warn },
    }
  end
  return spans
end

--- The headline of a section: a label, a count, and nothing else. The count is
--- the whole point of an inbox — zero means the board owes you nothing.
local function section_header(label, count, color)
  return {
    type = "text",
    len = 1,
    text = {
      {
        { text = "  " .. label, style = { fg = color, bold = true } },
        { text = " " .. count, style = { fg = theme.muted } },
      },
    },
  }
end

return {
  name = "attention",
  slot = "center",
  slot_mode = "switch",
  order = 81,
  focusable = true,

  keys = {
    { key = "f7", action = "attention.open", desc = "attention", scope = "global", group = "Talos" },
    { key = "j", action = "attention.down", desc = "next", group = "Talos" },
    { key = "k", action = "attention.up", desc = "previous", group = "Talos" },
    { key = "enter", action = "attention.open_session", desc = "open session", group = "Talos" },
    { key = "d", action = "attention.diff", desc = "refresh diff", group = "Talos" },
  },

  render = function(ctx)
    local state = ui()
    local list = rows()
    state.cursor = widgets.clamp(state.cursor, math.max(#list, 1))

    local blocked, review = collect()
    local children = {}

    if #list == 0 then
      children[#children + 1] = {
        type = "text",
        len = 1,
        text = theme.dim("  nothing is waiting on you"),
      }
    else
      local seen_sections = {}
      local body_rows = {}
      local section_at = {}
      for index, entry in ipairs(list) do
        if not seen_sections[entry.section] then
          seen_sections[entry.section] = true
          section_at[#section_at + 1] = index
        end
        body_rows[#body_rows + 1] = {
          spans = row_of(entry, index == state.cursor and ctx.focused),
          id = entry.session.id,
        }
      end

      -- The section headers ride the message band logic: drawn above the list
      -- with their counts, in workflow order, however many rows each holds.
      children[#children + 1] =
        section_header("NEEDS YOU", #blocked, theme.warn)
      if #review > 0 then
        children[#children + 1] =
          section_header("READY TO REVIEW", #review, theme.info)
      end

      local body = widgets.list({
        rows = body_rows,
        selected = state.cursor,
        height = math.max(1, math.min(ctx.height - 5, ROWS_MAX)),
      })
      body.fill = 1
      children[#children + 1] = body
    end

    children[#children + 1] = widgets.divider(ctx.width - 2)
    children[#children + 1] = widgets.hints({
      { "enter", "open session" },
      { "d", "diff" },
      { "j/k", "move" },
    })

    return {
      type = "box",
      frame = widgets.panel("Talos attention", ctx.focused),
      children = children,
    }
  end,

  on_action = function(action)
    local state = ui()
    local entry = selected()

    if action == "attention.open" then
      command("focus", { text = "attention", toggle = true })
      return true
    end
    if action == "attention.down" or action == "attention.up" then
      local list = rows()
      local step = action == "attention.down" and 1 or -1
      state.cursor = widgets.clamp(state.cursor + step, math.max(#list, 1))
      return true
    end
    if not entry then
      return false
    end
    if action == "attention.open_session" then
      -- Open the session exactly as the sessions pane's own chord would: the
      -- prompt it is blocked on is answered in its pane, not in this list.
      command("action", { text = "sessions.open", session = entry.session.id })
      return true
    end
    if action == "attention.diff" then
      -- Ask for the diff; the answer lands in `talos.diffs[id]` and the
      -- built-in reviewer (Ctrl+X) is where it is read in full.
      command("diff", { session = entry.session.id })
      command("message", { text = "diff requested for " .. entry.session.name, level = "info" })
      return true
    end
    return false
  end,
}
