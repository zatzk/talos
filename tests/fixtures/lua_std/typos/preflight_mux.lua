-- `talos.preflight.mux`, with one letter wrong.
return {
  name = "std_typo_preflight_mux",
  render = function()
    return { text = talos.preflight.mux.binray }
  end,
}
