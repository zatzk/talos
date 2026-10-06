-- `talos.worktrees`, with one letter wrong.
return {
  name = "std_typo_worktrees",
  render = function()
    return { text = tostring(talos.worktrees.lst) }
  end,
}
