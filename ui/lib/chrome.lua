-- What the full-height panes agree on about their borders.
--
-- The borders themselves are ordinary kernel `frame`s. They were once drawn by
-- hand, out of `text` nodes and a cell buffer, because a frame's title was a
-- plain unstyled left-aligned string and there was no way to put anything else
-- on a border cell. A frame now takes styled title runs, a `title_align`, a
-- `border_type` and an `overlay` — the strip, the scroll counts and the
-- scrollbar all paint onto the border cells the block drew — so the cell buffer
-- and the framed-pane builder are gone and what is left here is the agreement:
-- how a pane's border and title say whether it has focus — reached through
-- `lib/ui`'s `ui.panel` for a pane that uses the component layer, and directly by
-- the agent pane, which still assembles its own frame — and the box-drawing set
-- the agent pane's hint box draws by hand because it is a box INSIDE a pane, not
-- a frame around one.

local theme = require("lib.theme")
local widgets = require("lib.widgets")
local hover = require("lib.hover")
local panels = require("lib.panels")

local chrome = {}

local COLLAPSE_CHEVRON_CELLS = 3
local COLLAPSE_TOGGLE_MIN_WIDTH = 5
local COLLAPSE_HINT_MIN_WIDTH = 40

local function compact_chord(chord)
  local modifiers, key = "", chord
  while true do
    local prefix, rest = string.match(key, "^(%a+)%+(.*)$")
    if not prefix then break end
    local symbol = ({ ctrl = "^", shift = "⇧", alt = "⌥", cmd = "⌘" })[prefix]
    if not symbol then break end
    modifiers = modifiers .. symbol
    key = rest
  end
  if widgets.chars(key) == 1 then
    key = string.upper(key)
  elseif widgets.chars(key) > 1 then
    key = string.upper(string.sub(key, 1, 1)) .. string.sub(key, 2)
  end
  return modifiers .. key
end

local shortcut_cache = { src = nil, by_action = {} }

local function shortcut_for(action)
  local registry = talos and talos.registry
  local keys = (registry and registry.keys) or {}
  if not rawequal(registry, shortcut_cache.src) then
    shortcut_cache.src = registry
    shortcut_cache.by_action = {}
  end
  local cached = shortcut_cache.by_action[action]
  if cached ~= nil then
    return cached or nil
  end
  local first, found
  for _, binding in ipairs(keys) do
    if binding.action == action and binding.key then
      if string.match(binding.key, "^f%d+$") then
        found = compact_chord(binding.key)
        break
      end
      first = first or binding.key
    end
  end
  found = found or (first and compact_chord(first))
  shortcut_cache.by_action[action] = found or false
  return found
end

local function chip_style(primary)
  if primary then
    return { fg = theme.role("selection_fg"), bg = theme.role("selection_bg"), bold = true }
  end
  return { fg = theme.role("text_muted") }
end

local function chip_hover_style()
  return { fg = theme.role("inverted_fg"), bg = theme.role("accent_bright"), bold = true }
end

local function collapse_label(width)
  if width < COLLAPSE_TOGGLE_MIN_WIDTH then
    return nil
  end
  local chevron = panels.shown("sessions") and "◀" or "▶"
  local hint = width >= COLLAPSE_HINT_MIN_WIDTH and shortcut_for("sessions.toggle_panel") or nil
  if hint then
    return " " .. chevron .. " " .. hint .. " "
  end
  return " " .. chevron .. " "
end

local function tab_label(spec)
  if spec.shortcut then
    return spec.name .. " · " .. spec.shortcut
  end
  return spec.name
end

local function tabs_block_width(specs)
  if #specs == 0 then return 0 end
  local total = 0
  for _, spec in ipairs(specs) do
    total = total + widgets.len(tab_label(spec)) + 2
  end
  return total + #specs - 1
end

local function trim_tabs(specs, usable)
  while #specs > 1 and tabs_block_width(specs) > usable do
    local stripped = false
    for _, spec in ipairs(specs) do
      if spec.shortcut then
        spec.shortcut = nil
        stripped = true
      end
    end
    if not stripped then
      break
    end
  end
  return specs
end

function chrome.central_tab_specs(active)
  return {
    {
      name = "Chat",
      active = active == "chat",
      shortcut = shortcut_for("chat.open"),
      role = "action:chat.open",
    },
    {
      name = "Shell",
      active = active == "shell",
      shortcut = shortcut_for("shell.open") or "F8",
      role = "action:shell.open",
    },
    {
      name = "Board",
      active = active == "board",
      shortcut = shortcut_for("kanban.open") or "F2",
      role = "action:kanban.open",
    },
  }
end

function chrome.central_tab_strip(width, border_style, active, rule)
  local runs, cursor = {}, 1
  local function put(at, text, style, role)
    if at > cursor then
      runs[#runs + 1] = { text = string.rep(rule, at - cursor), style = border_style }
    end
    runs[#runs + 1] = { text = text, style = style, role = role }
    cursor = at + widgets.len(text)
  end

  local label = collapse_label(width)
  if label then
    local toggle = "action:sessions.toggle_panel"
    local lit = hover.role(toggle)
    local band = lit and theme.role("selection_bg") or nil
    put(1, widgets.keep_left(label, COLLAPSE_CHEVRON_CELLS), { fg = theme.accent, bg = band }, toggle)
    local hint = widgets.keep_right(label, widgets.len(label) - COLLAPSE_CHEVRON_CELLS)
    if hint ~= "" then
      put(cursor, hint, { fg = theme.muted, bg = band }, toggle)
    end
  end

  local start = label and (cursor + 1) or 1
  local specs = trim_tabs(chrome.central_tab_specs(active), math.max(0, (width - 1) - start))
  local limit = width - 1
  local x = start
  for index, spec in ipairs(specs) do
    local gap = (index > 1) and 1 or 0
    local chip = widgets.len(tab_label(spec)) + 2
    if x + gap + chip > limit then
      break
    end
    x = x + gap
    put(
      x,
      " " .. tab_label(spec) .. " ",
      hover.style(spec.role, chip_hover_style(), chip_style(spec.active)),
      spec.role
    )
    x = x + chip
  end

  return runs, cursor
end

function chrome.central_border_frame(title, level, border, strip, bar)
  return {
    title = { { text = title, style = chrome.title_style(level) } },
    title_align = "right",
    border_type = chrome.border_type(level),
    border_style = border,
    overlay = { top_left = strip, right_column = bar },
  }
end

-- ── Span measurement ────────────────────────────────────────────────────────

--- Display columns across a span list (widgets.len handles one string).
function chrome.spans_len(spans)
  local total = 0
  for _, span in ipairs(spans) do
    total = total + widgets.len(span.text or "")
  end
  return total
end

-- ── Focus styling ───────────────────────────────────────────────────────────
--
-- One question every pane has to answer the same way: which one has the keys?
-- The answer is carried three ways at once, so it survives any one of them
-- being lost:
--
--   * the SHAPE of the border — thick for the focused pane, thin for the rest;
--   * a MARK and a BADGE on the title — ` ▸ Title `, bold, on a filled field;
--   * COLOUR — the theme's `border_focused` and `border_unfocused` roles.
--
-- The first two are what a monochrome terminal, a colour-blind reader and a
-- low-contrast theme are left with, which is why colour is the last of the
-- three rather than the only one. v1 drew the focused border thick as well.
--
-- The kernel publishes a single `focused` boolean and `chrome.level` maps it
-- to one of three levels. Unfocused is `inactive`: a quiet border in the
-- theme's own unfocused role, so the one accented frame on screen is the one
-- with focus. `active` (thin, accent) is kept for a pane that wants to stay
-- lit without focus — an edited pane from an older release asks for it by
-- name — but no bundled pane does: an accent on two frames at once is what
-- made "which one has focus?" a question in the first place.

--- The mark a focused pane's title carries. U+25B8, a narrow glyph in every
--- East Asian width table, so it costs one column wherever it is drawn.
chrome.MARK = "▸"

--- The level a pane's `ctx.focused` maps to.
---@param focused boolean?
---@return "focused"|"inactive"
function chrome.level(focused)
  return focused and "focused" or "inactive"
end

--- The border glyphs a level is drawn with: a `frame.border_type`.
function chrome.border_type(level)
  return level == "focused" and "thick" or "rounded"
end

--- The horizontal border glyph a level is drawn with, for a pane that paints
--- runs over its own top border and has to fill the gaps between them.
function chrome.rule(level)
  return level == "focused" and "━" or "─"
end

function chrome.border_style(level)
  if level == "focused" then
    return { fg = theme.border_focused, bold = true }
  elseif level == "active" then
    return { fg = theme.accent }
  end
  return { fg = theme.border }
end

--- v1's `ui::title_style` / `Theme::focused_title()`. Focused is a BADGE —
--- inverted foreground on the focused border's colour, bold — not merely a
--- brighter text colour, so the title and the frame around it agree.
function chrome.title_style(level)
  if level == "focused" then
    return { fg = theme.role("inverted_fg"), bg = theme.border_focused, bold = true }
  elseif level == "active" then
    return { fg = theme.accent }
  end
  return { fg = theme.secondary }
end

--- A title's text with its padding, and the mark when the level is focused.
---@param text string
---@param level string
---@return string
function chrome.label(text, level)
  if level == "focused" then
    return " " .. chrome.MARK .. " " .. text .. " "
  end
  return " " .. text .. " "
end

--- The whole focus convention as a `frame`: title, border glyphs and border
--- colour. `ui.panel` builds on it; a pane that assembles its own frame can
--- start from it and add what it needs.
---@param title string
---@param level string
---@return talos.Frame
function chrome.frame(title, level)
  return {
    title = { { text = chrome.label(title, level), style = chrome.title_style(level) } },
    border_type = chrome.border_type(level),
    border_style = chrome.border_style(level),
  }
end

--- The square box-drawing set, for a box a pane draws INSIDE itself out of
--- text — where there is no frame to ask for `border_type = "square"`.
chrome.SQUARE = { tl = "┌", tr = "┐", bl = "└", br = "┘", h = "─", v = "│" }

return chrome
