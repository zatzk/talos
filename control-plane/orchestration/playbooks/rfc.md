# Playbook: rfc — design the technical RFC

Phase 3 of the spec pipeline. Dispatched with the approved PRD in hand.

# Goal
Design the technical RFC implementing the approved PRD's ASRs, filed under
`rfcs/rfc-<slug>.md`.

# Context
Read `prds/prd-<slug>.md` in this checkout. The ASRs are the contract; the RFC
is how they are met.

# Requirements of the document
The RFC must contain:
- Diagrama de topologia de sistemas (Mermaid) — every box a real deployable
- Diagramas de sequência (Mermaid) for the critical flows, with failure paths
- Modelagem de dados, migrações e contratos DTO/API
- Estratégia de resiliência: timeouts, circuit breakers, degradação graciosa,
  rollback via feature flags
- Tabela de decisões com ROI (esforço × impacto)

# Constraints
- One file: `rfcs/rfc-<slug>.md`
- Every ASR from the PRD is answered or explicitly deferred with a reason
- Audit before handoff: `talos audit-spec --doc-type RFC --content "$(cat rfcs/rfc-<slug>.md)"`

# Done means
- The RFC exists at `rfcs/rfc-<slug>.md`
- `talos audit-spec` answers `APPROVED`
- Every ASR has a section

# Report
thurbox-cli message send --kind result --body 'RFC ready: rfcs/rfc-<slug>.md'
