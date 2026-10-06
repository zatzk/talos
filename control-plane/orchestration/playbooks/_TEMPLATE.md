# Playbook: <name>
#
# Copy this template to `<name>.md` for each new recipe. A playbook is ONE
# self-contained prompt a worker can be dispatched with: it restates the
# goal, the constraints and what "done" means from scratch, because workers
# share no context with the control plane and none with each other.

# Goal
<one imperative sentence describing the unit of work>

# Context
<what the worker must know: repo, module, the ASRs or tickets this traces to,
links to the PRD/RFC under orchestration/ or prds/ or rfcs/ in this checkout.
Do not paste whole documents — name the file and the sections that matter.>

# Constraints
- <hard rules: do not touch X, keep public API stable, must be one commit/PR>

# Done means
- <objective, verifiable criteria — a test that passes, a file that exists,
a diff that applies. "Done" is what the lead greps for; write it greppable.>

# Report
When finished, mail the lead:
    talos-cli message send --kind result --body '<PR url or one-line verdict>'
