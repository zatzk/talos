-- Talos Chat — The native conversational orchestrator interface (F1).
--
-- The primary operator view of Talos v3:
-- 1. Conversation threads per workspace.
-- 2. Message stream history with Markdown rendering.
-- 3. Jev Intent routing badges (Auto persona, tier, backend: API vs CLI).
-- 4. Streaming responses and direct execution.
-- 5. Quick action to approve specs and commit (Ctrl+A).

local chrome = require("lib.chrome")
local theme = require("lib.theme")
local widgets = require("lib.widgets")
local textinput = require("lib.textinput")

local NAME = "chat"

local function chat_store()
  store.chat_state = store.chat_state or {
    active_thread = "General",
    input_field = textinput.new({ placeholder = "Digite sua mensagem para o Jev / Talos..." }),
    messages = {
      {
        role = "system",
        sender = "⚡ Jev Engine",
        badge = "Auto: spec-master (fast)",
        time = "10:00",
        content = "Talos v3 pronto. Workspace ativo: principal. Pressione [F1] Chat, [F2] Board, [F3] Fleet, [F4] Seletor.",
      },
    },
    streaming = false,
    has_spec_context = false,
    selected_target = "Auto (Jev)",
  }
  return store.chat_state
end

local function format_message(msg, is_last, width)
  local sender_color = theme.accent
  if msg.role == "user" then
    sender_color = theme.info
  elseif msg.role == "system" then
    sender_color = theme.warn
  end

  local header_spans = {
    { text = "● ", style = { fg = sender_color, bold = true } },
    { text = (msg.sender or msg.role) .. " ", style = { fg = sender_color, bold = true } },
  }

  if msg.badge then
    header_spans[#header_spans + 1] = {
      text = "[" .. msg.badge .. "] ",
      style = { fg = theme.muted },
    }
  end

  if msg.time then
    header_spans[#header_spans + 1] = {
      text = "(" .. msg.time .. ")",
      style = { fg = theme.muted },
    }
  end

  local lines = {
    { spans = header_spans },
    { spans = { { text = "  " .. msg.content, style = { fg = theme.text } } } },
    { spans = { { text = "", style = {} } } },
  }
  return lines
end

return {
  name = NAME,
  slot = "center",
  slot_mode = "switch",
  order = 15,
  focusable = true,

  pills = {
    { action = "chat.open", label = "chat", priority = 5 },
  },

  keys = {
    -- F1 primary and Alt+1 alternate per RFC v2.0
    { key = "f1", action = "chat.open", desc = "chat view", scope = "global", group = "Talos" },
    { key = "alt+1", action = "chat.open", desc = "chat view", scope = "global", group = "Talos" },
    { key = "enter", action = "chat.send", desc = "send message", group = "Chat" },
    { key = "ctrl+n", action = "chat.new_thread", desc = "new thread", group = "Chat" },
    { key = "ctrl+a", action = "chat.approve_spec", desc = "approve spec & commit", group = "Chat" },
  },

  render = function(ctx)
    local state = chat_store()
    local width, height = ctx.width or 80, ctx.height or 24
    local level = chrome.level(ctx.focused)

    local message_rows = {}
    for i, msg in ipairs(state.messages) do
      local formatted = format_message(msg, i == #state.messages, width)
      for _, line in ipairs(formatted) do
        message_rows[#message_rows + 1] = line
      end
    end

    local list_height = math.max(1, height - 7)
    local messages_widget = widgets.list({
      rows = message_rows,
      selected = #message_rows,
      height = list_height,
      fill = 1,
    })

    local input_box = {
      type = "box",
      axis = "horizontal",
      len = 1,
      children = {
        {
          type = "text",
          len = 4,
          text = { { { text = " > ", style = { fg = theme.accent, bold = true } } } },
        },
        textinput.render(state.input_field, width - 6, ctx.focused),
      },
    }

    local footer_hints = {
      { "enter", "send" },
      { "ctrl+a", "approve spec" },
      { "f4", "model selector" },
      { "f2", "board" },
      { "f3", "fleet" },
    }

    local header_info = "Thread: " .. state.active_thread .. " | Target: " .. state.selected_target
    if state.has_spec_context then
      header_info = header_info .. " | [Ctrl+A] Spec pronta para aprovação!"
    end

    return {
      type = "box",
      frame = chrome.frame("Talos Chat — " .. header_info, level),
      children = {
        messages_widget,
        widgets.divider(width - 2),
        input_box,
        widgets.divider(width - 2),
        widgets.hints(footer_hints),
      },
    }
  end,

  on_action = function(action)
    local state = chat_store()

    if action == "chat.open" then
      command("focus", { text = NAME, toggle = true })
      return true
    end

    if action == "chat.send" then
      local val = state.input_field.value or ""
      if val:match("%S") then
        state.messages[#state.messages + 1] = {
          role = "user",
          sender = "Operador",
          time = "Agora",
          content = val,
        }
        textinput.clear(state.input_field)

        -- Heurística de resposta imediata simulando intent do Jev
        local lower = val:lower()
        local is_spec = lower:find("prd") or lower:find("rfc") or lower:find("especifica")
        if is_spec then
          state.has_spec_context = true
          state.messages[#state.messages + 1] = {
            role = "agent",
            sender = "⚡ Jev Engine",
            badge = "Auto: architect (flagship API)",
            time = "Agora",
            content = "Estrutura de especificação detectada. Pressione [Ctrl+A] para validar os contratos, gerar as tarefas canônicas no Board e commitar em code-documentation.",
          }
        else
          state.messages[#state.messages + 1] = {
            role = "agent",
            sender = "Talos Agent",
            badge = "Auto: dev (fast API)",
            time = "Agora",
            content = "Compreendido: \"" .. val .. "\". Roteado para execução no workspace ativo.",
          }
        end
      end
      return true
    end

    if action == "chat.new_thread" then
      state.messages = {}
      state.active_thread = "Thread-" .. tostring(#state.messages + 1)
      state.has_spec_context = false
      command("message", { text = "Nova conversa iniciada: " .. state.active_thread, level = "info" })
      return true
    end

    if action == "chat.approve_spec" then
      command("action", { text = "spec.approve_and_commit" })
      state.has_spec_context = false
      state.messages[#state.messages + 1] = {
        role = "system",
        sender = "📡 Control Plane",
        badge = "git commit",
        time = "Agora",
        content = "Especificação aprovada com sucesso! Tarefas desmembradas no Workspace Board [F2] e commitadas em code-documentation.",
      }
      return true
    end

    return false
  end,

  on_key = function(key)
    local state = chat_store()
    return textinput.key(state.input_field, key)
  end,
}
