---
name: fmt-avant-de-rendre
description: Avant de rendre, lance `cargo fmt` puis `cargo clippy --all-targets -- -D warnings`, même si la preuve de ta tâche ne les demande pas : la preuve de la vague les demande.
---

## What happened
Le 16/11/2026, vague des règles (PR 88) : la tâche `page` a été refusée (7008). Sa preuve ne lançait ni `cargo fmt --check` ni clippy, et la main a rendu un code qui n'était pas au format de rustfmt. La preuve de la vague, rejouée après la fusion de la tâche, le vérifie : elle a rougi, la fusion a été défaite et la tâche refusée sans relance. Il a fallu une vague de plus (PR 89).

## Why
La preuve de ce dépôt finit par `cargo clippy --all-targets -- -D warnings && cargo fmt --check` (`AGENTS.md`, « La preuve »). Une tâche verte sur sa propre preuve peut encore faire rougir celle de la vague, et une tâche refusée en 7008 n'a pas de seconde tentative.

## How it is checked
Lydite rejoue la preuve de la vague sur la tête après chaque fusion de tâche : `cargo fmt --check` y rougit sur la moindre ligne hors format, clippy sur la moindre alerte.

## Pitfall
`cargo fmt --check` ne corrige rien : lance d'abord `cargo fmt`, puis la vérification. Et `--all-targets` fait lire à clippy `tests/` et `examples/` aussi : un test ou un exemple que tu écris doit être sans alerte, comme `src/`.

## Files

## Falls when
Lydite met lui-même le travail d'une main au format avant de rejouer la preuve.
