-- `talos.branches`, with one letter wrong.
return {
  name = "std_typo_branches",
  render = function()
    return { text = tostring(talos.branches.lst) }
  end,
}
