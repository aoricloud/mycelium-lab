---
name: contrat-sans-code
description: Dans une vague de tests (règle 9), une tâche qui déclare un contrat écrit les signatures et leur doc, avec des corps `todo!()` : le code vient dans la vague suivante.
---

## What happened
Le 16/11/2026, vague a de l'étape 18 (PR 90) : la tâche `declarer` devait poser le contrat du cube (`src/substrat3d.rs`, `src/cube.rs`) avec des corps `todo!()`, avant que d'autres tâches écrivent les tests. La main a écrit le calcul entier. La relecture a levé deux remarques bloquantes, et la correction a remis les `todo!()` (234 lignes retirées).

## Why
Règle 9 d'`AGENTS.md` : ce qui porte un nombre a ses tests d'abord, son code ensuite, en deux vagues. Des tests écrits à côté d'un code déjà là se règlent sur ce code, même s'il est faux. Avec des `todo!()`, ils sont écrits contre le contrat seul, et la vague du code doit les faire passer sans les toucher.

## How it is checked
Dans la vague des tests, avec la feature de l'étape allumée, chaque test de l'étape doit échouer sur un `todo!()` : un test qui passe déjà dit que du code est là.

## Pitfall
Dans la vague du code, c'est l'inverse : remplis chaque `todo!()` et retire les `#[allow(...)]` posés pour que le contrat compile. Un contrat doit compiler tel quel : types de retour exacts, doc complète de ce que chaque fonction rend et refuse.

## Files

## Falls when
Les plans de ce dépôt emploient la clé `red :` de Lydite, qui rejoue elle-même le test rouge sur la base avant d'accepter le code.
