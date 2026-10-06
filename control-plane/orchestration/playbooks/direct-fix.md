# Playbook: direct-fix — one task, one worktree

Phase 5 of the spec pipeline for S-sized work that skipped the spec phases.
Dispatched per atomic task, in its own worktree on the target repo.

# Goal
<the task's imperative sentence — typed by the lead from the ticket>

# Context
Repo: <repo>. Branch: created for this task. The ticket is `<TASK-nnn>` in
`tasks/` in this checkout; its description is the contract.

# Constraints
- Implement exactly the ticket's scope — nothing adjacent, nothing speculative
- Tests first or with: the ticket's acceptance criteria must be verifiable
- One logical change; if you find the scope is really two tasks, STOP and
  report instead of doing both

# Done means
- All existing tests pass; new tests cover the ticket's criteria
- `talos atomicity` would have answered READY_FOR_DEV for this scope
- The diff is limited to the ticket's module scope

# Report
thurbox-cli message send --kind result --body '<one-line verdict: what changed, test result>'
