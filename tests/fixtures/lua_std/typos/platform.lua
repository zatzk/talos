-- `talos.platform`, with one letter wrong.
return {
  name = "std_typo_platform",
  render = function()
    return { text = talos.platform.arhc }
  end,
}
