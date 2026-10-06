# Playbook: prd — draft the Product Requirements Document

Phase 1 of the spec pipeline. Dispatched to a worker with the operator's goal.

# Goal
Draft a complete PRD for the feature described below, in the repository this
session opens, filed under `prds/prd-<slug>.md`.

# Context
<operator goal, pasted by the lead>

# Requirements of the document
The PRD must contain:
- Regras de negócio (`RN`) — numbered, unambiguous
- Requisitos funcionais (`RF`) — each traceable to an RN
- Requisitos não-funcionais (`RNF`) — performance, security, operability
- **ASRs (Architecturally Significant Requirements)** — the requirements that
  shape the architecture rather than just the code, called out explicitly;
  the RFC phase starts from these
- Critérios de aceitação — testable, one per RF where possible

# Constraints
- One file: `prds/prd-<slug>.md`, Mermaid diagrams where a flow exists
- No implementation decisions — requirements, not designs
- Audit before handoff: run `talos audit-spec --doc-type PRD --content "$(cat prds/prd-<slug>.md)"`;
  `NEEDS_REVISION` means revise, not ship

# Done means
- The PRD exists at `prds/prd-<slug>.md`
- `talos audit-spec` answers `APPROVED`
- The ASRs section is non-empty

# Report
talos-cli message send --kind result --body 'PRD ready: prds/prd-<slug>.md (ASRs: N)'
