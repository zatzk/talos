-- `talos.theme.roles`, with one letter wrong.
return {
  name = "std_typo_theme_roles",
  render = function()
    return { text = tostring(talos.theme.roles.acent) }
  end,
}
