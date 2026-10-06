-- `talos.browse`, with one letter wrong.
return {
  name = "std_typo_browse",
  render = function()
    return { text = tostring(talos.browse.dirr) }
  end,
}
