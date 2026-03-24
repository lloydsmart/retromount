# Architecture Boundaries

## Pipeline Stages

1. Input
2. Identify
3. Decode
4. Normalize (Core Model)
5. Present / Encode

---

## Stage Contracts

### Input → Identify

- Input provides raw sources (files, archives, etc.)
- No semantic interpretation here

### Identify → Decode

- Identify determines what something *might be*
- Decode performs actual parsing/understanding

### Decode → Core Model

- Produces Content / GameContent
- Must not leak container-specific details

### Core Model → Presenter

- Presenter consumes normalized model
- Must not re-interpret raw inputs

---

## Known Boundary Violations (to fix)

- [ ] ...
- [ ] ...
